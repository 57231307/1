//! 销售发货-库存辅助子模块（delivery_ops/inventory）
//!
//! 批次 488 D10-3 拆分：从原 `so/delivery.rs` L747-1082 迁移。
//! 包含 6 个库存辅助方法：
//! - check_inventory（库存充足性校验，出库四维口径）
//! - lock_inventory（锁定库存，创建预留记录）
//! - reduce_inventory_four_dim（按款号+色号+缸号+批次四维扣减库存，
//!   指定缸不足时走显式跨缸回退，返回每笔实际扣减行的数量前后与真实缸号/批次）
//! - release_reservations（释放订单未出库的预留，保留预留行用于审计追溯）
//! - delete_reservations（订单硬删除前还原全部库存效果并物理删除预留行）
//! - restore_reserved_stock（按状态作用域回滚预留占用的库存，供上两者复用）

use std::collections::HashMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, Condition, EntityTrait, ExprTrait, QueryFilter, QuerySelect, Set};

use crate::models::status::inventory_reservation as reservation_status;
use crate::models::{inventory_reservation, inventory_stock, sales_order_item};
use crate::services::inventory_deduction::{
    AllocationSource, DeductionCandidate, DeductionError, plan_deduction,
    require_outbound_dimensions,
};
use crate::utils::error::AppError;

use super::super::delivery::ShipOrderItemRequest;
use super::super::order::SalesService;

/// 软终态（拒绝/取消）需要回滚的预留状态：仅尚未出库的 pending/locked。
///
/// `consumed` 代表货物已实际出库，其 available 扣减与 shipped 累加都是真实发生的业务事实；
/// 拒绝或取消订单不改变已发货部分，反向抹除会让账面凭空回退并销毁消耗审计。
/// `released`/`cancelled` 的库存效果已在此前回滚过，再次回加会虚增可用库存。
const RELEASE_SCOPED_STATUSES: &[&str] = &[reservation_status::PENDING, reservation_status::LOCKED];

/// 订单硬删除需要回滚的预留状态：在软终态范围之外，额外还原 `consumed` 的 shipped 扣减。
///
/// 主表与预留行都将被物理删除，追溯载体随之消失，因此必须把该订单造成的全部库存效果还原，
/// 否则 shipped 数量永久偏高；`released`/`cancelled` 同样必须排除，理由同上。
const DELETE_SCOPED_STATUSES: &[&str] = &[
    reservation_status::PENDING,
    reservation_status::LOCKED,
    reservation_status::CONSUMED,
];

/// 把预留状态集合拼成 OR 过滤条件，使查询与状态更新共用同一份作用域清单。
fn reservation_status_filter(statuses: &[&str]) -> Condition {
    let mut any = Condition::any();
    for status in statuses {
        any = any.add(inventory_reservation::Column::Status.eq(*status));
    }
    any
}

/// 单个发货明细的库存充足性判定结果（预留 + 四维同源，纯逻辑无 DB）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StockDecision {
    /// 通过
    Ok,
    /// 存在产品级预留但数量不足
    ReservationShort { reserved: Decimal },
    /// 四维候选行数为 0（含可回退其他缸亦无库存）
    NoStockRows,
    /// 四维候选合计不足
    Insufficient { available: Decimal },
}

/// 判定单个发货明细是否可出库（预留 + 四维同源口径，纯逻辑便于单测）。
///
/// 依据用户拍板规则：`check_inventory` 的预留分支不得短路 `continue` 而跳过四维校验，
/// 必须与 `reduce_inventory_four_dim` 一致——即便存在数量充足的产品级预留，仍要求
/// 四维候选（发货仓 + 款号 + 色号 + 批次；缸号允许显式跨缸回退）在库且合计覆盖发货量。
/// 缺维度由调用方先经 `require_outbound_dimensions` 拒绝；本函数只处理数量充足性。
///
/// 优先级：预留不足 > 四维无行 > 四维合计不足（与错误信息语义对齐）。
pub(crate) fn decide_item_stock(
    reserved: Option<Decimal>,
    four_dim_rows: usize,
    four_dim_available: Decimal,
    required: Decimal,
) -> StockDecision {
    if let Some(reserved) = reserved {
        if reserved < required {
            return StockDecision::ReservationShort { reserved };
        }
    }
    if four_dim_rows == 0 {
        return StockDecision::NoStockRows;
    }
    if four_dim_available < required {
        return StockDecision::Insufficient {
            available: four_dim_available,
        };
    }
    StockDecision::Ok
}

/// 四维扣减的一笔实际出库结果（调用方必须按此逐笔写出库明细 + 库存流水）。
#[derive(Debug, Clone)]
pub struct StockReduction {
    /// 实际被扣的库存行 ID
    pub stock_id: i32,
    /// 本笔实际扣减数量（米）
    pub quantity: Decimal,
    /// 扣减前该库存行可用数量
    pub quantity_before: Decimal,
    /// 扣减后该库存行可用数量
    pub quantity_after: Decimal,
    /// 实际被扣库存行的色号
    pub color_no: String,
    /// 实际被扣库存行的缸号（跨缸回退时与出库单指定缸号不同，必须如实记录）
    pub dye_lot_no: Option<String>,
    /// 实际被扣库存行的批次
    pub batch_no: String,
    /// 出库单指定的缸号
    pub requested_dye_lot_no: String,
    /// 精确命中还是显式跨缸回退
    pub source: AllocationSource,
}

impl StockReduction {
    /// 是否发生了跨缸回退（实际扣的缸 ≠ 出库单指定缸）
    pub fn is_cross_dye_lot(&self) -> bool {
        self.source == AllocationSource::CrossDyeLot
    }

    /// 写入出库明细/库存流水备注的缸号说明（跨缸回退必须留痕）
    pub fn dye_lot_trace_note(&self) -> String {
        if self.is_cross_dye_lot() {
            format!(
                "（跨缸回退：指定缸号 {} 数量不足，本笔实扣缸号 {}）",
                self.requested_dye_lot_no,
                self.dye_lot_no.clone().unwrap_or_default()
            )
        } else {
            String::new()
        }
    }
}

impl SalesService {
    // ========== 库存辅助方法（私有） ==========

    /// 检查库存是否充足（出库四维口径：款号+色号+缸号+批次）
    ///
    /// 用户拍板规则：出库必须按入库四维匹配扣减，缺维度直接报业务错误（不做兜底）；
    /// 指定缸号数量不足时允许跨缸回退，因此充足性按"发货仓 + 同款号+色号+批次"的
    /// 全部候选行合计判断，与 `reduce_inventory_four_dim` 的可扣口径严格一致。
    ///
    /// 预留分支与扣减同源（废除原 `continue` 短路）：即便存在数量充足的产品级预留，
    /// 仍逐项按四维候选核实在库，四维不全/无库存/不足在建单校验期即报错，
    /// 判定逻辑由纯函数 [`decide_item_stock`] 承载并单测锁定。
    pub(crate) async fn check_inventory(
        &self,
        order_id: i32,
        warehouse_id: i32,
        items: &[ShipOrderItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        if items.is_empty() {
            return Ok(());
        }

        // v11 批次 38 修复：批量查询预留记录，避免循环内逐个查询（N+1）
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();

        // 批量查询该订单所有 pending 预留记录，按 product_id 索引（取每组第一条，与原 .one() 语义一致）
        let reservations = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.is_in(product_ids))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .all(txn)
            .await?;
        let reservation_map: std::collections::HashMap<i32, &inventory_reservation::Model> =
            reservations
                .iter()
                .fold(std::collections::HashMap::new(), |mut acc, r| {
                    // 仅保留每个 product_id 的第一条（与原 .one() 语义一致）
                    acc.entry(r.product_id).or_insert(r);
                    acc
                });

        for item in items {
            // 缺维度先报错：即使存在产品级预留也不能掩盖出库单维度缺失（不做兜底）
            let dims = require_outbound_dimensions(
                "销售发货明细",
                item.product_id,
                item.color_no.as_deref(),
                item.dye_lot_no.as_deref(),
                item.batch_no.as_deref(),
            )?;

            // 预留分支不再短路 continue：产品级预留只保证锁定数量，四维口径校验必须与
            // reduce_inventory_four_dim 同源——存在预留时先校验其数量是否覆盖发货量，随后仍
            // 按四维候选（发货仓 + 同款号+色号+批次；缸号允许显式跨缸回退）核实在库充足。
            // 四维不全或无库存/不足即在建单校验期返回业务错误，而非留到运行期扣减才暴露口径冲突。
            let candidates =
                Self::load_four_dim_candidates(item.product_id, warehouse_id, &dims, false, txn)
                    .await?;
            let available_total: Decimal = candidates.iter().map(|s| s.quantity_available).sum();
            let reserved = reservation_map.get(&item.product_id).map(|r| r.quantity);
            match decide_item_stock(reserved, candidates.len(), available_total, item.quantity) {
                StockDecision::Ok => {}
                StockDecision::ReservationShort { reserved } => {
                    return Err(AppError::business(format!(
                        "产品 {} 预留数量 {} 小于发货数量 {}",
                        item.product_id, reserved, item.quantity
                    )));
                }
                StockDecision::NoStockRows => {
                    return Err(Self::no_stock_error(item.product_id, &dims));
                }
                StockDecision::Insufficient { available } => {
                    return Err(Self::insufficient_error(
                        item.product_id,
                        &dims,
                        available,
                        item.quantity,
                    ));
                }
            }
        }
        Ok(())
    }

    /// 锁定库存（创建预留记录）
    pub(crate) async fn lock_inventory(
        &self,
        order_id: i32,
        items: &[super::super::SalesOrderItemRequest],
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let existing_ids =
            Self::query_existing_reservation_ids(order_id, &product_ids, txn).await?;
        let stock_map = Self::query_locked_stock_map(&product_ids, &existing_ids, txn).await?;
        let reservations = Self::build_and_lock_reservations(
            order_id,
            items,
            user_id,
            &existing_ids,
            &stock_map,
            txn,
        )
        .await?;
        Self::batch_insert_reservations(reservations, txn).await
    }

    /// 查询订单已存在的 pending 预留 product_id 集合
    async fn query_existing_reservation_ids(
        order_id: i32,
        product_ids: &[i32],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<std::collections::HashSet<i32>, AppError> {
        if product_ids.is_empty() {
            return Ok(std::collections::HashSet::new());
        }
        let ids: std::collections::HashSet<i32> = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.is_in(product_ids.to_vec()))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .all(txn)
            .await?
            .into_iter()
            .map(|r| r.product_id)
            .collect();
        Ok(ids)
    }

    /// 批量加锁查询需锁定的库存记录
    async fn query_locked_stock_map(
        product_ids: &[i32],
        existing_ids: &std::collections::HashSet<i32>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<std::collections::HashMap<i32, inventory_stock::Model>, AppError> {
        let need_lock: Vec<i32> = product_ids
            .iter()
            .filter(|pid| !existing_ids.contains(pid))
            .copied()
            .collect();
        if need_lock.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let map: std::collections::HashMap<i32, inventory_stock::Model> =
            inventory_stock::Entity::find()
                .filter(inventory_stock::Column::ProductId.is_in(need_lock))
                .lock_exclusive()
                .all(txn)
                .await?
                .into_iter()
                .map(|s| (s.product_id, s))
                .collect();
        Ok(map)
    }

    /// 遍历 items 构建预留记录并逐条锁定库存
    async fn build_and_lock_reservations(
        order_id: i32,
        items: &[super::super::SalesOrderItemRequest],
        user_id: i32,
        existing_ids: &std::collections::HashSet<i32>,
        stock_map: &std::collections::HashMap<i32, inventory_stock::Model>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<inventory_reservation::ActiveModel>, AppError> {
        let mut reservations: Vec<inventory_reservation::ActiveModel> = Vec::new();
        for item in items {
            if existing_ids.contains(&item.product_id) {
                tracing::info!("产品 {} 已存在预留记录，跳过创建", item.product_id);
                continue;
            }
            let stock = stock_map.get(&item.product_id).cloned().ok_or_else(|| {
                AppError::business(format!("产品 {} 没有库存记录，无法锁定", item.product_id))
            })?;
            Self::check_stock_sufficient(&stock, item)?;
            reservations.push(Self::build_reservation_active_model(
                order_id, item, user_id, &stock,
            ));
            Self::execute_stock_lock(&stock, item, txn).await?;
        }
        Ok(reservations)
    }

    /// 校验库存是否充足
    fn check_stock_sufficient(
        stock: &inventory_stock::Model,
        item: &super::super::SalesOrderItemRequest,
    ) -> Result<(), AppError> {
        if stock.quantity_available < item.quantity {
            return Err(AppError::business(format!(
                "产品 {} 库存不足，无法锁定",
                item.product_id
            )));
        }
        Ok(())
    }

    /// 构建单条预留记录 ActiveModel
    fn build_reservation_active_model(
        order_id: i32,
        item: &super::super::SalesOrderItemRequest,
        user_id: i32,
        stock: &inventory_stock::Model,
    ) -> inventory_reservation::ActiveModel {
        inventory_reservation::ActiveModel {
            id: Default::default(),
            order_id: Set(order_id),
            product_id: Set(item.product_id),
            warehouse_id: Set(stock.warehouse_id),
            quantity: Set(item.quantity),
            status: Set(reservation_status::PENDING.to_string()),
            reserved_at: Set(chrono::Utc::now()),
            released_at: Set(None),
            notes: Set(None),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
    }

    /// 执行库存锁定 UPDATE（带防御性 WHERE 条件）
    async fn execute_stock_lock(
        stock: &inventory_stock::Model,
        item: &super::super::SalesOrderItemRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let lock_result = inventory_stock::Entity::update_many()
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .filter(inventory_stock::Column::QuantityAvailable.gte(item.quantity))
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                    .sub(item.quantity),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .exec(txn)
            .await?;
        if lock_result.rows_affected == 0 {
            return Err(AppError::business(format!(
                "产品 {} 库存不足（并发冲突或库存已被其他事务扣减）",
                item.product_id
            )));
        }
        Ok(())
    }

    /// 批量插入预留记录
    async fn batch_insert_reservations(
        reservations: Vec<inventory_reservation::ActiveModel>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        if !reservations.is_empty() {
            inventory_reservation::Entity::insert_many(reservations)
                .exec(txn)
                .await?;
        }
        Ok(())
    }

    /// 扣减库存（出库四维匹配：款号+色号+缸号+批次）
    ///
    /// 规则（用户拍板，禁止兜底）：
    /// - 出库明细必须显式携带色号/缸号/批次（款号由 product_id 承载），缺失直接业务错误；
    /// - 优先扣"款号+色号+缸号+批次"精确命中的库存行；
    /// - 仅当指定缸号在该款号+色号+批次下数量不足时，才走**显式跨缸回退**，
    ///   次序确定可解释：缸号字典序升序（无缸号行最后）→ 入库时间升序 → 库存行 ID 升序；
    /// - 返回每一笔实际扣减的库存行（真实缸号/批次 + 前后数量 + 是否跨缸），
    ///   调用方必须按此逐笔写出库明细与库存流水，不得合并掩盖实际扣的哪个缸。
    pub(crate) async fn reduce_inventory_four_dim(
        &self,
        item: &ShipOrderItemRequest,
        warehouse_id: i32,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<StockReduction>, AppError> {
        let dims = require_outbound_dimensions(
            "销售发货明细",
            item.product_id,
            item.color_no.as_deref(),
            item.dye_lot_no.as_deref(),
            item.batch_no.as_deref(),
        )?;

        // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发发货导致超扣
        let candidates =
            Self::load_four_dim_candidates(item.product_id, warehouse_id, &dims, true, txn).await?;
        if candidates.is_empty() {
            // 该四维组合（含可回退的其他缸）完全无库存记录：明确报错，不回退到"产品+色号"
            return Err(Self::no_stock_error(item.product_id, &dims));
        }

        let plan_candidates: Vec<DeductionCandidate> = candidates
            .iter()
            .map(|s| DeductionCandidate {
                stock_id: s.id,
                dye_lot_no: s.dye_lot_no.clone(),
                quantity_available: s.quantity_available,
                created_at: s.created_at,
            })
            .collect();
        let allocations = plan_deduction(
            &plan_candidates,
            dims.dye_lot_no.as_deref(),
            item.quantity,
        )
        .map_err(|e| match e {
            DeductionError::NoStockAtAll => Self::no_stock_error(item.product_id, &dims),
            DeductionError::Insufficient {
                available_total,
                required,
            } => Self::insufficient_error(item.product_id, &dims, available_total, required),
        })?;

        let mut reductions: Vec<StockReduction> = Vec::with_capacity(allocations.len());
        for alloc in allocations {
            let stock = candidates
                .iter()
                .find(|s| s.id == alloc.stock_id)
                .ok_or_else(|| AppError::internal("扣减规划返回了候选之外的库存行"))?;
            // 批次 9（2026-06-28）：UPDATE 加防御性 WHERE 条件 quantity_available >= 扣减量，
            // 即使并发绕过 SELECT FOR UPDATE（理论上不会发生），也能阻止超扣
            let reduce_result = inventory_stock::Entity::update_many()
                .filter(inventory_stock::Column::Id.eq(stock.id))
                .filter(inventory_stock::Column::QuantityAvailable.gte(alloc.quantity))
                .col_expr(
                    inventory_stock::Column::QuantityAvailable,
                    sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                        .sub(alloc.quantity),
                )
                .col_expr(
                    inventory_stock::Column::QuantityShipped,
                    sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityShipped)
                        .add(alloc.quantity),
                )
                .col_expr(
                    inventory_stock::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()),
                )
                .exec(txn)
                .await?;
            if reduce_result.rows_affected == 0 {
                return Err(AppError::business(format!(
                    "款号（产品 {}）色号 {} 批次 {} 库存不足（并发冲突或库存已被其他事务扣减）",
                    item.product_id, dims.color_no, dims.batch_no
                )));
            }
            reductions.push(StockReduction {
                stock_id: stock.id,
                quantity: alloc.quantity,
                quantity_before: alloc.quantity_before,
                quantity_after: alloc.quantity_after,
                // 如实记录：实际被扣库存行自己的色号/缸号/批次
                color_no: stock.color_no.clone(),
                dye_lot_no: stock.dye_lot_no.clone(),
                batch_no: stock.batch_no.clone(),
                // 白坯出库单无缸号维度，归一为空串（不会写入跨缸留痕，is_cross_dye_lot 恒 false）
                requested_dye_lot_no: dims.dye_lot_no.clone().unwrap_or_default(),
                source: alloc.source,
            });
        }

        // 标记预留为已完成（产品级预留，四维消耗完成后同样置 consumed）
        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.eq(item.product_id))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .col_expr(
                inventory_reservation::Column::Status,
                sea_orm::sea_query::Expr::val(reservation_status::CONSUMED.to_string()),
            )
            .col_expr(
                inventory_reservation::Column::ReleasedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .col_expr(
                inventory_reservation::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .exec(txn)
            .await?;

        Ok(reductions)
    }

    /// 四维候选库存行查询：发货仓 + 款号 + 色号 + 批次（缸号不过滤——跨缸回退允许扣其他缸，
    /// 缸号维度的取舍与排序由 `plan_deduction` 统一负责）。
    async fn load_four_dim_candidates(
        product_id: i32,
        warehouse_id: i32,
        dims: &crate::services::inventory_deduction::OutboundDimensions,
        for_update: bool,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<inventory_stock::Model>, AppError> {
        let query = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .filter(inventory_stock::Column::ColorNo.eq(&dims.color_no))
            .filter(inventory_stock::Column::BatchNo.eq(&dims.batch_no));
        let rows = if for_update {
            query.lock_exclusive().all(txn).await?
        } else {
            query.all(txn).await?
        };
        Ok(rows)
    }

    /// 四维组合完全无库存的业务错误（不做兜底）
    fn no_stock_error(
        product_id: i32,
        dims: &crate::services::inventory_deduction::OutboundDimensions,
    ) -> AppError {
        let color_disp = if dims.color_no.is_empty() {
            "白坯（无颜色）"
        } else {
            dims.color_no.as_str()
        };
        let dims_disp = match dims.dye_lot_no.as_deref() {
            Some(lot) => format!(
                "色号 {} + 缸号 {} + 批次 {}",
                color_disp, lot, dims.batch_no
            ),
            None => format!("色号 {} + 批次 {}（白坯免缸号）", color_disp, dims.batch_no),
        };
        AppError::business(format!(
            "款号（产品 {}）+ {} 无任何库存记录，出库被拒绝（不回退到产品+色号扣减）",
            product_id, dims_disp
        ))
    }

    /// 四维口径（含跨缸回退范围）库存不足的业务错误
    fn insufficient_error(
        product_id: i32,
        dims: &crate::services::inventory_deduction::OutboundDimensions,
        available_total: Decimal,
        required: Decimal,
    ) -> AppError {
        let color_disp = if dims.color_no.is_empty() {
            "白坯（无颜色）"
        } else {
            dims.color_no.as_str()
        };
        let reason = match dims.dye_lot_no.as_deref() {
            Some(lot) => format!(
                "款号（产品 {}）+ 色号 {} + 批次 {} 可用库存合计 {}（含跨缸回退的其他缸）小于出库数量 {}，指定缸号 {} 数量不足且其他缸亦不足以补足",
                product_id, color_disp, dims.batch_no, available_total, required, lot
            ),
            None => format!(
                "款号（产品 {}）+ 色号 {} + 批次 {} 无缸号库存合计 {} 小于出库数量 {}（白坯不做跨缸回退）",
                product_id, color_disp, dims.batch_no, available_total, required
            ),
        };
        AppError::business(reason)
    }

    /// 释放订单的库存预留记录（回滚未出库预留占用的库存；保留预留行用于审计追溯）
    ///
    /// 用于拒绝/取消等软终态场景：订单主表仍存在，预留行必须保留，否则后续查询与追溯丢失。
    /// 仅处理 pending/locked 行：已 consumed 的行代表实际出库事实，既不回滚其库存，
    /// 也不改写成 cancelled，否则消耗审计被抹除且账面凭空回退。
    pub(crate) async fn release_reservations(
        &self,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        self.restore_reserved_stock(order_id, RELEASE_SCOPED_STATUSES, txn)
            .await?;

        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(reservation_status_filter(RELEASE_SCOPED_STATUSES))
            .col_expr(
                inventory_reservation::Column::Status,
                sea_orm::sea_query::Expr::val(reservation_status::CANCELLED.to_string()),
            )
            .col_expr(
                inventory_reservation::Column::ReleasedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .col_expr(
                inventory_reservation::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .exec(txn)
            .await?;

        Ok(())
    }

    /// 硬删除订单预留记录并回滚库存（订单主表将被物理删除）
    ///
    /// fk_inventory_reservations_order 无 ON DELETE 动作，遗留任何预留行都会阻断 sales_orders
    /// 主表删除，报"数据关联错误"并返回 500。因此本方法物理删除该订单的全部预留行。
    ///
    /// 回滚范围含 consumed：主表与预留行都即将消失，追溯载体随之删除，必须把该订单造成的
    /// 全部库存效果还原，否则 quantity_shipped 永久偏高。released/cancelled 排除，
    /// 它们的库存效果此前已回滚，重复回加会虚增可用库存。
    pub(crate) async fn delete_reservations(
        &self,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        self.restore_reserved_stock(order_id, DELETE_SCOPED_STATUSES, txn)
            .await?;

        inventory_reservation::Entity::delete_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .exec(txn)
            .await?;

        Ok(())
    }

    /// 回滚指定状态集合内预留记录占用的库存（按 (product_id, warehouse_id) 聚合，不修改预留行）
    ///
    /// 与 lock_inventory / reduce_inventory 严格对称：
    /// - pending/locked：create_order 时只执行 quantity_available -= qty（仅锁定），回滚时回加 available；
    /// - consumed：发货时 reduce_inventory 执行 available -= qty 且 quantity_shipped += qty，
    ///   回滚时只能回减 shipped（available 已在发货时扣减，重复回加会导致库存超发）。
    ///
    /// `statuses` 由调用方给出，用于把"可回滚"限定在仍持有库存效果的行上，
    /// 使重复释放/已释放行不会被二次回加。
    ///
    /// 部分发货注意：reduce_inventory 会把整行预留标记为 consumed，但只扣减实际发货数量，
    /// 所以 consumed 行的回滚量必须取 min(res.quantity, 订单明细 shipped_quantity)，
    /// 直接扣 res.quantity 会把 quantity_shipped 扣成负数。
    async fn restore_reserved_stock(
        &self,
        order_id: i32,
        statuses: &[&str],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let reservations = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(reservation_status_filter(statuses))
            .all(txn)
            .await?;

        if reservations.is_empty() {
            return Ok(());
        }

        // 订单明细实际已发货数量（权威值，用于修正部分发货场景的 consumed 预留行）
        // 同一产品可能有多行明细，先按 product_id 汇总为可回滚的 shipped 池
        let mut shipped_pool: HashMap<i32, Decimal> = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .all(txn)
            .await?
            .into_iter()
            .filter(|it| it.shipped_quantity > Decimal::ZERO)
            .fold(HashMap::new(), |mut map, it| {
                *map.entry(it.product_id).or_insert(Decimal::ZERO) += it.shipped_quantity;
                map
            });

        let mut restore: HashMap<(i32, i32), (Decimal, Decimal)> = HashMap::new();
        for res in &reservations {
            let entry = restore
                .entry((res.product_id, res.warehouse_id))
                .or_insert((Decimal::ZERO, Decimal::ZERO));
            if res.status == reservation_status::CONSUMED {
                // shipped 池无该产品 = 订单明细 shipped_quantity 为 0，说明没有真实出库量可回减；
                // 此处按 0 处理并留痕，不能臆造回滚量（否则会把 shipped 回减成不存在的负值区间）
                let Some(remaining) = shipped_pool.get_mut(&res.product_id) else {
                    tracing::warn!(
                        order_id,
                        product_id = res.product_id,
                        reserved_quantity = %res.quantity,
                        "预留行状态为 consumed 但订单明细无已发货数量，跳过 shipped 回减，需人工核查账实一致性"
                    );
                    continue;
                };
                if *remaining <= Decimal::ZERO {
                    // shipped 已耗尽：实际发货量已被其他预留行回滚，跳过避免扣成负数
                    continue;
                }
                // 部分发货：reduce_inventory 把整行标记 consumed 但只扣实际发货量，
                // 回滚量取 min(预留量, 剩余 shipped 池)，防止 quantity_shipped 被扣成负数
                let return_shipped = res.quantity.min(*remaining);
                *remaining -= return_shipped;
                entry.1 += return_shipped;
            } else {
                entry.0 += res.quantity;
            }
        }

        let now = chrono::Utc::now();
        for ((product_id, warehouse_id), (return_available, return_shipped)) in restore {
            let stock = inventory_stock::Entity::find()
                .filter(inventory_stock::Column::ProductId.eq(product_id))
                .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
                .lock_exclusive()
                .one(txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("产品 {} 库存记录", product_id)))?;

            let mut update = inventory_stock::Entity::update_many()
                .filter(inventory_stock::Column::Id.eq(stock.id));
            if !return_available.is_zero() {
                update = update.col_expr(
                    inventory_stock::Column::QuantityAvailable,
                    sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                        .add(return_available),
                );
            }
            if !return_shipped.is_zero() {
                update = update
                    .col_expr(
                        inventory_stock::Column::QuantityShipped,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityShipped)
                            .sub(return_shipped),
                    )
                    .filter(inventory_stock::Column::QuantityShipped.gte(return_shipped));
            }
            update = update.col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(now),
            );

            let result = update.exec(txn).await?;
            if result.rows_affected == 0 {
                return Err(AppError::business(format!(
                    "产品 {} 库存回滚失败（并发冲突或已发货数量不足）",
                    product_id
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{StockDecision, decide_item_stock};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(v: &str) -> Decimal {
        Decimal::from_str(v).unwrap()
    }

    #[test]
    fn covered_reservation_still_rejected_when_four_dim_has_no_rows() {
        // 预留四维校验核心不变量：产品级预留数量充足（100 >= 50），但四维候选为 0 行
        // （如锁定后被其他出库消耗）——旧实现 `continue` 会误判通过，新口径必须报无库存。
        let decision = decide_item_stock(Some(d("100")), 0, Decimal::ZERO, d("50"));
        assert_eq!(decision, StockDecision::NoStockRows);
    }

    #[test]
    fn covered_reservation_with_sufficient_four_dim_stock_passes() {
        assert_eq!(
            decide_item_stock(Some(d("100")), 2, d("100"), d("50")),
            StockDecision::Ok
        );
    }

    #[test]
    fn reservation_short_takes_priority_over_four_dim() {
        // 预留数量不足时先报预留短，与既有错误信息语义保持一致
        assert_eq!(
            decide_item_stock(Some(d("10")), 0, Decimal::ZERO, d("50")),
            StockDecision::ReservationShort { reserved: d("10") }
        );
    }

    #[test]
    fn four_dim_total_short_reports_insufficient_with_available() {
        assert_eq!(
            decide_item_stock(None, 3, d("20"), d("50")),
            StockDecision::Insufficient { available: d("20") }
        );
    }

    #[test]
    fn no_reservation_and_no_four_dim_rows_rejected() {
        assert_eq!(
            decide_item_stock(None, 0, Decimal::ZERO, d("1")),
            StockDecision::NoStockRows
        );
    }
}
