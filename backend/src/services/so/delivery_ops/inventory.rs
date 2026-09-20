//! 销售发货-库存辅助子模块（delivery_ops/inventory）
//!
//! 批次 488 D10-3 拆分：从原 `so/delivery.rs` L747-1082 迁移。
//! 包含 6 个库存辅助方法：
//! - check_inventory（库存充足性校验，批量查询消除 N+1）
//! - lock_inventory（锁定库存，创建预留记录）
//! - reduce_inventory（扣减库存，返回变更前后数量 + 色号/缸号）
//! - release_reservations（释放订单未出库的预留，保留预留行用于审计追溯）
//! - delete_reservations（订单硬删除前还原全部库存效果并物理删除预留行）
//! - restore_reserved_stock（按状态作用域回滚预留占用的库存，供上两者复用）

use std::collections::HashMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, Condition, EntityTrait, ExprTrait, QueryFilter, QuerySelect, Set};

use crate::models::status::inventory_reservation as reservation_status;
use crate::models::{inventory_reservation, inventory_stock, sales_order_item};
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

impl SalesService {
    // ========== 库存辅助方法（私有） ==========

    /// 检查库存是否充足
    pub(crate) async fn check_inventory(
        &self,
        order_id: i32,
        items: &[ShipOrderItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        if items.is_empty() {
            return Ok(());
        }

        // v11 批次 38 修复：批量查询所有预留记录和库存记录，避免循环内逐个查询（N+1，最坏 2N 次查询）
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();

        // 批量查询该订单所有 pending 预留记录，按 product_id 索引（取每组第一条，与原 .one() 语义一致）
        let reservations = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.is_in(product_ids.clone()))
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

        // 批量查询所有相关库存记录，按 product_id 索引
        let stocks = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, &inventory_stock::Model> =
            stocks.iter().map(|s| (s.product_id, s)).collect();

        for item in items {
            // 优先从预留记录查询
            if let Some(res) = reservation_map.get(&item.product_id) {
                if res.quantity < item.quantity {
                    return Err(AppError::business(format!(
                        "产品 {} 预留数量 {} 小于发货数量 {}",
                        item.product_id, res.quantity, item.quantity
                    )));
                }
                continue;
            }

            // 没有预留记录时直接查询库存
            match stock_map.get(&item.product_id) {
                Some(s) => {
                    if s.quantity_available < item.quantity {
                        return Err(AppError::business(format!(
                            "产品 {} 库存 {} 小于发货数量 {}",
                            item.product_id, s.quantity_available, item.quantity
                        )));
                    }
                }
                None => {
                    return Err(AppError::business(format!(
                        "产品 {} 库存不存在",
                        item.product_id
                    )));
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

    /// 扣减库存
    /// 返回 (变更前可用数量, 变更后可用数量)，用于记录库存流水
    pub(crate) async fn reduce_inventory(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity: Decimal,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(Decimal, Decimal, String, Option<String>), AppError> {
        // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发发货导致超扣
        let stock = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 {} 库存记录", product_id)))?;

        if stock.quantity_available < quantity {
            return Err(AppError::business(format!(
                "产品 {} 库存 {} 小于发货数量 {}",
                product_id, stock.quantity_available, quantity
            )));
        }

        // 批次 9（2026-06-28）：UPDATE 加防御性 WHERE 条件 quantity_available >= quantity，
        // 即使并发绕过 SELECT FOR UPDATE（理论上不会发生），也能阻止超扣
        let reduce_result = inventory_stock::Entity::update_many()
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .filter(inventory_stock::Column::QuantityAvailable.gte(quantity))
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                    .sub(quantity),
            )
            .col_expr(
                inventory_stock::Column::QuantityShipped,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityShipped)
                    .add(quantity),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .exec(txn)
            .await?;

        if reduce_result.rows_affected == 0 {
            return Err(AppError::business(format!(
                "产品 {} 库存不足（并发冲突或库存已被其他事务扣减）",
                product_id
            )));
        }

        // 标记预留为已完成
        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.eq(product_id))
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

        // 批次 356 v13 复审 B-P0-2 修复：返回变更前后的可用数量，供调用方记录库存流水
        // v14 批次 418 修复 D-P0-5：同时返回库存的 color_no/dye_lot_no，
        // 供调用方在库存流水中记录真实缸号/色号，替代原 None/空字符串硬编码
        let qty_before = stock.quantity_available;
        let qty_after = qty_before - quantity;
        Ok((
            qty_before,
            qty_after,
            stock.color_no.clone(),
            stock.dye_lot_no.clone(),
        ))
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
