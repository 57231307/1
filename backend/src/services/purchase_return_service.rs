//! 采购退货 Service
//!
//! 采购退货服务层，负责采购退货的核心业务逻辑
//!
// 批次 101 v6 复审 P2 修复：update_item / delete / update_return_totals 三处审计操作人
// Some(0) 占位符改为真实 user_id，调用方 create_item / update_item / delete_item / delete
// 同步添加 user_id 参数透传（P2-3 / P2-4 / P2-5）。

use crate::models::status::purchase_order as po_status;
use crate::models::status::purchase_return as pr_status;
use crate::models::{
    inventory_stock, product, purchase_order, purchase_order_item, purchase_return,
    purchase_return_item, supplier, user,
};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::inventory_stock_query::RecordTransactionArgs;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{DataScopeContext, apply_data_scope, check_resource_owner};
use crate::utils::error::AppError;
// 批次 258 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use crate::utils::sql_escape::safe_like_pattern;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, JoinType,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// 采购退货单读模型：实体列 + LEFT JOIN 关联出的来源采购订单号、供应商名、创建人名。
///
/// 三个 JOIN 名列均 `Option<String>`（order_id 可空 / 关联缺失时为 NULL）；实体自身列
/// 按 `purchase_return` 约束保持原类型。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct PurchaseReturnView {
    pub id: i32,
    pub return_no: String,
    pub receipt_id: Option<i32>,
    pub order_id: Option<i32>,
    pub supplier_id: i32,
    pub return_date: chrono::NaiveDate,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub reason_type: Option<String>,
    pub reason_detail: Option<String>,
    pub return_status: Option<String>,
    pub total_quantity: Option<Decimal>,
    pub total_quantity_alt: Option<Decimal>,
    pub total_amount: Option<Decimal>,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_by: Option<i32>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rejected_reason: Option<String>,
    pub purchase_order_no: Option<String>,
    pub supplier_name: Option<String>,
    pub created_by_name: Option<String>,
}

/// 解析日期筛选边界：前端下发 ISO / `YYYY-MM-DD`，取日期部分转 `NaiveDate`；解析失败视为未提供。
fn parse_date_bound(raw: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(&raw[..raw.len().min(10)], "%Y-%m-%d").ok()
}

/// 采购退货服务
pub struct PurchaseReturnService {
    db: Arc<DatabaseConnection>,
}

/// 退货扣减库存上下文：封装逐项扣减所需 txn/仓库/单号/单 ID/操作人，避免 7+ 参数触发 clippy
struct ReturnItemDeductionCtx<'a> {
    txn: &'a sea_orm::DatabaseTransaction,
    warehouse_id: i32,
    return_no: &'a str,
    return_id: i32,
    user_id: i32,
}

/// compute_item_amounts 计算结果：明细金额四元组
struct ItemAmounts {
    subtotal: Decimal,
    discount_amount: Decimal,
    tax_amount: Decimal,
    total_amount: Decimal,
}

/// 库存四维定位键：产品 + 色号 + 缸号 + 批次（仓库由退货单 `warehouse_id` 统一约束）。
///
/// 与采购收货侧 `purchase_receipt_private::StockDimKey`（产品+批次+色号+缸号+等级）同口径的
/// 四维子集：退货明细 `purchase_return_item` 不携带等级维度，故键不含等级。缸号在库存行侧为
/// `Option<String>`（白坯 NULL）、在退货明细侧为 `String`（白坯空串），两侧统一 trim 归一为空串，
/// 与 `so::delivery_ops::cancel::restore_inventory` 的空缸号回位判定同源（不做兜底、不任选一行）。
type StockDimKey = (i32, String, String, String);

/// 归一化缸号：trim 后空白视为白坯的空串（对齐 inventory_stock.dye_lot_no NULL→''、
/// purchase_return_item.dye_lot_no '' 两侧口径）。
fn normalize_dye_lot(raw: &str) -> String {
    raw.trim().to_string()
}

/// 退货明细行的四维库存定位键。
fn return_item_stock_key(item: &purchase_return_item::Model) -> StockDimKey {
    (
        item.product_id,
        item.color_no.clone(),
        normalize_dye_lot(&item.dye_lot_no),
        item.batch_no.clone(),
    )
}

/// 库存行的四维库存定位键，与 `return_item_stock_key` 同口径。
fn stock_row_key(stock: &inventory_stock::Model) -> StockDimKey {
    (
        stock.product_id,
        stock.color_no.clone(),
        normalize_dye_lot(stock.dye_lot_no.as_deref().unwrap_or("")),
        stock.batch_no.clone(),
    )
}

/// 四维库存索引：`by_key` 为唯一命中行；`ambiguous` 记录同一四维键命中多行
/// （退货明细未携带等级维度、无法判定实际扣哪一行）的键，查询时对 `ambiguous` 命中报业务错误，
/// 绝不任选一行，也不回退到"任意同产品行"。
struct StockIndex {
    by_key: std::collections::HashMap<StockDimKey, inventory_stock::Model>,
    ambiguous: std::collections::HashSet<StockDimKey>,
}

impl PurchaseReturnService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成退货单号
    // 格式：RT + 年月日 + 三位序号（RT20260315001）
    crate::impl_generate_no!(
        generate_return_no,
        "RT",
        purchase_return::Entity,
        purchase_return::Column::ReturnNo
    );

    /// 创建采购退货单
    pub async fn create_return(
        &self,
        req: CreatePurchaseReturnRequest,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let return_no = self.generate_return_no().await?;

        let return_order = purchase_return::ActiveModel {
            id: Default::default(),
            return_no: Set(return_no),
            receipt_id: Set(req.receipt_id),
            order_id: Set(req.order_id),
            supplier_id: Set(req.supplier_id),
            return_date: Set(req.return_date),
            warehouse_id: Set(req.warehouse_id),
            department_id: Set(req.department_id),
            reason_type: Set(Some(req.reason_type)),
            reason_detail: Set(req.reason_detail),
            return_status: Set(Some(pr_status::DRAFT.to_string())),
            total_quantity: Set(None),
            total_quantity_alt: Set(None),
            total_amount: Set(None),
            notes: Set(req.notes),
            created_by: Set(Some(user_id)),
            updated_by: Set(None),
            approved_by: Set(None),
            approved_at: Set(None),
            rejected_reason: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let return_order = return_order.insert(&txn).await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 更新采购退货单
    pub async fn update_return(
        &self,
        return_id: i32,
        req: UpdatePurchaseReturnRequest,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购退货单 {}", return_id)))?;

        if return_order.return_status.as_deref() != Some(pr_status::DRAFT) {
            return Err(AppError::business(format!(
                "退货单状态不允许修改，当前状态：{:?}",
                return_order.return_status
            )));
        }

        let mut return_active: purchase_return::ActiveModel = return_order.into();

        if let Some(reason_type) = req.reason_type {
            return_active.reason_type = Set(Some(reason_type));
        }
        if let Some(reason_detail) = req.reason_detail {
            return_active.reason_detail = Set(Some(reason_detail));
        }
        if let Some(notes) = req.notes {
            return_active.notes = Set(Some(notes));
        }
        return_active.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            return_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        Ok(return_order)
    }

    /// 提交采购退货单
    pub async fn submit_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无事务、无行锁，并发提交会基于过期状态通过状态检查后重复写入。
        let txn = (*self.db).begin().await?;

        // 1. 加 lock_exclusive 串行化并发状态变更
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购退货单 {}", return_id)))?;

        // 2. 检查状态
        if return_order.return_status.as_deref() != Some(pr_status::DRAFT) {
            return Err(AppError::business(format!(
                "退货单状态不允许提交，当前状态：{:?}",
                return_order.return_status
            )));
        }

        // 3. 更新状态 + 审计日志（事务内原子提交）
        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.return_status = Set(Some(pr_status::SUBMITTED.to_string()));
        return_active.updated_at = Set(Utc::now());

        // submit_return 已在批次 59b 透传 user_id（原 TODO 已随实现落地移除）
        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            return_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 审批采购退货单
    pub async fn approve_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现先在事务外用 &*self.db 裸查询退货单状态，再 begin() 开启事务，
        // 并发 approve_return 均通过状态检查后基于过期状态写入，导致状态门失效。
        let txn = (*self.db).begin().await?;

        // P0 5-2 修复：收集 record_transaction_txn 返回的库存流水事件，
        // 在 commit 成功后统一 publish，避免事务回滚时幻事件
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        let return_order = Self::load_and_validate_return_for_approval(&txn, return_id).await?;
        let return_order =
            Self::update_return_status_to_approved(&txn, return_order, user_id).await?;

        // 1. 扣减库存（在事务内执行，保证原子性）
        let (items, stock_index) =
            Self::load_return_items_with_stock_map(&txn, &return_order).await?;
        Self::deduct_stock_for_return_items(
            &txn,
            &return_order,
            items.clone(),
            &stock_index,
            user_id,
            &mut pending_events,
        )
        .await?;

        // 2. 回写来源采购订单进度：退货即撤销一部分已收货，必须在同一事务内把来源 PO 明细的
        //    已收货量（received_quantity/quantity_alt）按退货量减回并重算 PO 状态，与库存扣减
        //    原子一致（不得在 commit 后单独裸写，否则回滚时库存已扣而订单进度未撤，账实漂移）。
        Self::writeback_source_order_received_quantity(&txn, &return_order, &items, user_id)
            .await?;

        // 3. 提交事务（库存扣减、退货状态更新与订单进度回写在同一事务内）
        txn.commit().await?;

        // P0 5-2 修复：commit 成功后统一发布库存流水事件，避免事务回滚时幻事件
        for ev in pending_events {
            EVENT_BUS.publish(ev);
        }

        // 4. 自动生成应付红字账单（冲销）- 在事务外执行，失败不影响库存扣减
        self.try_generate_ap_invoice_from_return(return_id, user_id, &return_order.return_no)
            .await;

        Ok(return_order)
    }

    /// 锁定退货单（lock_exclusive）+ 状态校验（SUBMITTED）+ 明细非空校验
    async fn load_and_validate_return_for_approval(
        txn: &sea_orm::DatabaseTransaction,
        return_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        // 获取退货单（加 lock_exclusive 串行化并发状态变更）
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购退货单 {}", return_id)))?;

        if return_order.return_status.as_deref() != Some(pr_status::SUBMITTED) {
            return Err(AppError::business(format!(
                "退货单状态不允许审批，当前状态：{:?}",
                return_order.return_status
            )));
        }

        // 检查是否有退货明细
        // 批次 27 v7 P1 修复：事务边界泄漏，原实现 count 用 &*self.db 裸查询
        // 存在 TOCTOU 风险（并发 approve + add_item 时计数读快照不一致，可绕过"明细非空"校验）
        let item_count = purchase_return_item::Entity::find()
            .filter(purchase_return_item::Column::ReturnId.eq(return_id))
            .count(txn)
            .await?;

        if item_count == 0 {
            return Err(AppError::business("退货单至少需要一行明细".to_string()));
        }

        Ok(return_order)
    }

    /// 更新退货单状态为 APPROVED + 写入审计日志
    async fn update_return_status_to_approved(
        txn: &sea_orm::DatabaseTransaction,
        return_order: purchase_return::Model,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.return_status = Set(Some(pr_status::APPROVED.to_string()));
        return_active.approved_by = Set(Some(user_id));
        return_active.approved_at = Set(Some(Utc::now()));
        return_active.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            return_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        Ok(return_order)
    }

    /// 加载退货明细 + 批量加载库存记录（避免 N+1；warehouse_id 缺失时索引为空）。
    ///
    /// 缺陷修复：原实现仅按 `product_id` 建 `HashMap<i32, _>`，`.collect()` 会把同仓库、同产品
    /// 但色号/缸号/批次/等级不同的多条库存行相互覆盖（后读到的赢），导致扣错行或扣到不存在的那行。
    /// 现按退货明细真实携带的四维（产品+色号+缸号+批次）建索引，同键多行登记为歧义供扣减时显式报错。
    async fn load_return_items_with_stock_map(
        txn: &sea_orm::DatabaseTransaction,
        return_order: &purchase_return::Model,
    ) -> Result<(Vec<purchase_return_item::Model>, StockIndex), AppError> {
        let items = purchase_return_item::Entity::find()
            .filter(purchase_return_item::Column::ReturnId.eq(return_order.id))
            .all(txn)
            .await?;

        let (by_key, ambiguous) = match return_order.warehouse_id {
            Some(warehouse_id) => {
                let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
                let mut by_key: std::collections::HashMap<StockDimKey, inventory_stock::Model> =
                    std::collections::HashMap::new();
                let mut ambiguous: std::collections::HashSet<StockDimKey> =
                    std::collections::HashSet::new();
                if !product_ids.is_empty() {
                    let stocks = inventory_stock::Entity::find()
                        .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
                        .filter(inventory_stock::Column::ProductId.is_in(product_ids))
                        .all(txn)
                        .await?;
                    // 按四维键建索引：首个命中行入 by_key；同键再来一行即登记 ambiguous 并保留
                    // 已入行（歧义判定用）。不再像原 `HashMap<i32, _>` 那样静默后读覆盖先读。
                    for s in stocks {
                        let key = stock_row_key(&s);
                        match by_key.get(&key) {
                            Some(_) => {
                                ambiguous.insert(key);
                            }
                            None => {
                                by_key.insert(key, s);
                            }
                        }
                    }
                }
                (by_key, ambiguous)
            }
            None => (
                std::collections::HashMap::new(),
                std::collections::HashSet::new(),
            ),
        };

        Ok((items, StockIndex { by_key, ambiguous }))
    }

    /// 循环扣减每项退货明细的库存（按四维精确命中库存行；找不到或多行歧义均报业务错误，不兜底）
    async fn deduct_stock_for_return_items(
        txn: &sea_orm::DatabaseTransaction,
        return_order: &purchase_return::Model,
        items: Vec<purchase_return_item::Model>,
        stock_index: &StockIndex,
        user_id: i32,
        pending_events: &mut Vec<BusinessEvent>,
    ) -> Result<(), AppError> {
        // return_order.warehouse_id 在采购退货单创建时应必填；缺失时跳过所有项
        let Some(warehouse_id) = return_order.warehouse_id else {
            for item in items {
                tracing::warn!(
                    "采购退货单 {} 缺少调入仓库ID，跳过行项 {}",
                    return_order.id,
                    item.id
                );
            }
            return Ok(());
        };

        let ctx = ReturnItemDeductionCtx {
            txn,
            warehouse_id,
            return_no: &return_order.return_no,
            return_id: return_order.id,
            user_id,
        };

        for item in items {
            let key = return_item_stock_key(&item);
            // 同四维键命中多条库存行（差异仅在退货明细未携带的等级维度）：无法唯一定位，显式报错。
            if stock_index.ambiguous.contains(&key) {
                return Err(AppError::business(format!(
                    "退货明细 {} 的四维（产品 {}+色号 {}+缸号 {}+批次 {}）在仓库 {} 命中多条库存行，退货明细未携带等级维度无法唯一定位，需人工核查（不兜底、不任选一行）",
                    item.id,
                    item.product_id,
                    item.color_no,
                    item.dye_lot_no,
                    item.batch_no,
                    warehouse_id
                )));
            }
            // 四维精确匹配不到库存行：报业务错误并指明缺失的维度，不回退到"任意同产品行"。
            let Some(s) = stock_index.by_key.get(&key).cloned() else {
                return Err(AppError::business(format!(
                    "退货明细 {} 的四维（产品 {}+色号 {}+缸号 {}+批次 {}）在仓库 {} 找不到对应库存行，无法退货（缺少对应维度库存，不兜底）",
                    item.id,
                    item.product_id,
                    item.color_no,
                    item.dye_lot_no,
                    item.batch_no,
                    warehouse_id
                )));
            };

            let event = Self::deduct_single_item_stock(&ctx, &item, &s).await?;
            if let Some(ev) = event {
                pending_events.push(ev);
            }
        }
        Ok(())
    }

    /// 扣减单个明细项的库存 + 记录库存流水
    async fn deduct_single_item_stock(
        ctx: &ReturnItemDeductionCtx<'_>,
        item: &purchase_return_item::Model,
        stock: &inventory_stock::Model,
    ) -> Result<Option<BusinessEvent>, AppError> {
        if stock.quantity_meters < item.quantity {
            return Err(AppError::business(format!(
                "产品 {} 库存不足，当前库存：{}，需要退货：{}",
                item.product_id, stock.quantity_meters, item.quantity
            )));
        }

        let new_quantity_meters = stock.quantity_meters - item.quantity;
        let new_quantity_kg = stock.quantity_kg - item.quantity_alt;

        crate::services::inventory_stock_service::InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
            ctx.txn, stock.id, new_quantity_meters, new_quantity_kg, stock.version,
        ).await?;

        // P0 5-2 修复：record_transaction_txn 不再在函数内 publish 事件，
        // 改为返回 (Model, Option<BusinessEvent>)，由调用方在 commit 后统一 publish
        // 批次 338 v10 复审 P3 修复：使用参数对象替代多参数
        let (_, txn_event) = crate::services::inventory_stock_service::InventoryStockService::record_transaction_txn(
            ctx.txn,
            RecordTransactionArgs {
                transaction_type: "PURCHASE_RETURN".to_string(),
                product_id: item.product_id,
                warehouse_id: ctx.warehouse_id,
                batch_no: stock.batch_no.clone(),
                color_no: stock.color_no.clone(),
                dye_lot_no: stock.dye_lot_no.clone(),
                grade: stock.grade.clone(),
                quantity_meters: -item.quantity,
                quantity_kg: -item.quantity_alt,
                source_bill_type: Some("purchase_return".to_string()),
                source_bill_no: Some(ctx.return_no.to_string()),
                source_bill_id: Some(ctx.return_id),
                quantity_before_meters: Some(stock.quantity_meters),
                quantity_before_kg: Some(stock.quantity_kg),
                quantity_after_meters: Some(new_quantity_meters),
                quantity_after_kg: Some(new_quantity_kg),
                notes: Some("采购退货扣减库存".to_string()),
                created_by: Some(ctx.user_id),
            },
        ).await?;
        Ok(txn_event)
    }

    /// 退货审批回写来源采购订单进度：把退货量从来源 PO 明细的已收货量
    /// （`received_quantity`/`received_quantity_alt`）减回（下限 0，不得负），并按收货进度重算 PO 状态。
    ///
    /// 依据权威实现 `purchase_receipt_private`（正向累加 + 全收/部分收判定）反向对称落地；
    /// 状态取值来自 `crate::models::status::purchase_order`，与写入方同源，不自造词表。
    /// 必须在审批事务内、与库存扣减同一 `txn` 原子提交（不得在 commit 后单独裸写）。
    ///
    /// 退货明细不携带来源订单行 ID（`order_item_id`），故按产品维度归集退货量、在该订单同产品的
    /// 订单明细间按 `line_no` 升序逐行减回（确定性次序，逐行 `min` 保证下限 0，绝不写负）。
    async fn writeback_source_order_received_quantity(
        txn: &sea_orm::DatabaseTransaction,
        return_order: &purchase_return::Model,
        items: &[purchase_return_item::Model],
        user_id: i32,
    ) -> Result<(), AppError> {
        // 退货单可无来源采购订单（order_id 可空）：显式告警跳过，不静默。
        let Some(order_id) = return_order.order_id else {
            tracing::warn!(
                "采购退货单 {} 未关联来源采购订单，跳过已收货量回写",
                return_order.return_no
            );
            return Ok(());
        };

        // 1. 按产品维度归集退货量（主单位 + 辅单位），BTreeMap 保证遍历确定性
        let mut returned_by_product: std::collections::BTreeMap<i32, (Decimal, Decimal)> =
            std::collections::BTreeMap::new();
        for item in items {
            let entry = returned_by_product
                .entry(item.product_id)
                .or_insert((Decimal::ZERO, Decimal::ZERO));
            entry.0 += item.quantity;
            entry.1 += item.quantity_alt;
        }
        if returned_by_product.is_empty() {
            return Ok(());
        }

        // 2. 锁定来源 PO 明细行（串行化并发收货/退货，防已收货量丢失更新）
        let po_items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .lock_exclusive()
            .all(txn)
            .await?;

        // 3. 按产品分组、line_no 升序，逐行把退货量减回（每行 min(退货, 该行已收) 保证下限 0）
        let mut po_items_by_product: std::collections::BTreeMap<
            i32,
            Vec<purchase_order_item::Model>,
        > = std::collections::BTreeMap::new();
        for oi in po_items {
            po_items_by_product
                .entry(oi.product_id)
                .or_default()
                .push(oi);
        }
        for po_lines in po_items_by_product.values_mut() {
            po_lines.sort_by_key(|oi| oi.line_no);
        }

        for (product_id, (ret_qty, ret_qty_alt)) in returned_by_product {
            let Some(lines) = po_items_by_product.get_mut(&product_id) else {
                return Err(AppError::business(format!(
                    "采购订单 {} 无产品 {} 的订单明细行，无法回写退货已收货量",
                    order_id, product_id
                )));
            };
            let mut remaining = ret_qty;
            let mut remaining_alt = ret_qty_alt;
            for line in lines.iter_mut() {
                if remaining.is_zero() && remaining_alt.is_zero() {
                    break;
                }
                let take = remaining.min(line.received_quantity);
                let take_alt = remaining_alt.min(line.received_quantity_alt);
                let before = line.received_quantity;
                let new_received = line.received_quantity - take;
                let new_received_alt = line.received_quantity_alt - take_alt;
                remaining -= take;
                remaining_alt -= take_alt;
                let updated = Self::decrease_po_item_received(
                    txn,
                    line.clone(),
                    new_received,
                    new_received_alt,
                    user_id,
                )
                .await?;
                tracing::info!(
                    "退货单 {} 回写采购订单 {} 明细 {}（产品 {}）已收货量：{} → {}",
                    return_order.return_no,
                    order_id,
                    updated.id,
                    product_id,
                    before,
                    updated.received_quantity
                );
                *line = updated;
            }
            // 退货量超出该订单该产品累计已收货量：已按 0 下限逐行截断；仍有剩余即数据异常，显式告警不静默。
            if remaining > Decimal::ZERO || remaining_alt > Decimal::ZERO {
                tracing::warn!(
                    "退货单 {}：采购订单 {} 产品 {} 退货量超出其累计已收货量，已按 0 下限截断（剩余主 {} 辅 {}）",
                    return_order.return_no,
                    order_id,
                    product_id,
                    remaining,
                    remaining_alt
                );
            }
        }

        // 4. 按重算后的收货进度同步 PO 状态（与收货侧判定同源，补充退货至零的「未收」态）
        let new_status = Self::determine_order_status_after_return(txn, order_id).await?;
        Self::save_order_status_update(txn, order_id, new_status, user_id).await?;
        tracing::info!(
            "退货单 {} 回写采购订单 {} 状态为 {}",
            return_order.return_no,
            order_id,
            new_status
        );
        Ok(())
    }

    /// 写回单个采购订单明细的已收货量（含审计日志），返回带新值的模型。
    async fn decrease_po_item_received(
        txn: &sea_orm::DatabaseTransaction,
        order_item: purchase_order_item::Model,
        new_received: Decimal,
        new_received_alt: Decimal,
        user_id: i32,
    ) -> Result<purchase_order_item::Model, AppError> {
        let mut active: purchase_order_item::ActiveModel = order_item.into();
        active.received_quantity = Set(new_received);
        active.received_quantity_alt = Set(new_received_alt);
        active.updated_at = Set(Utc::now());
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active,
            // P1 1-1 同口径：审计日志操作人为真实审批人 user_id
            Some(user_id),
        )
        .await?;
        Ok(updated)
    }

    /// 退货回写后重新判定采购订单收货状态。
    ///
    /// 依据 `purchase_receipt_private::determine_order_receipt_status` 的全收/部分收规则，
    /// 补充退货至零的「未收」态回退到收货前态 [`po_status::APPROVED`]（与写入方 po/contract.rs
    /// 审批落库值同源）；状态取值全部来自 `crate::models::status::purchase_order`，不自造字面量。
    async fn determine_order_status_after_return(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
    ) -> Result<&'static str, AppError> {
        let all_order_items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(txn)
            .await?;
        let mut is_fully_received = true;
        let mut has_received = false;
        for oi in &all_order_items {
            if oi.received_quantity > Decimal::ZERO {
                has_received = true;
            }
            if oi.received_quantity < oi.quantity {
                is_fully_received = false;
            }
        }
        Ok(if is_fully_received {
            po_status::COMPLETED
        } else if has_received {
            po_status::PARTIAL_RECEIVED
        } else {
            po_status::APPROVED
        })
    }

    /// 更新采购订单状态并写审计日志（与 `purchase_receipt_private::save_order_status_update` 同口径）。
    async fn save_order_status_update(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
        new_status: &str,
        user_id: i32,
    ) -> Result<(), AppError> {
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;
        let mut active_order: purchase_order::ActiveModel = order.into();
        active_order.order_status = Set(new_status.to_string());
        active_order.updated_at = Set(Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_order,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 后置：自动生成应付红字账单（冲销）- 在事务外执行，失败不影响库存扣减
    async fn try_generate_ap_invoice_from_return(
        &self,
        return_id: i32,
        user_id: i32,
        return_no: &str,
    ) {
        let ap_service =
            crate::services::ap_invoice_service::ApInvoiceService::new(self.db.clone());
        if let Err(e) = ap_service
            .auto_generate_from_return(return_id, user_id)
            .await
        {
            tracing::error!("自动生成应付账单失败 (退货单 {}): {}", return_no, e);
            // 记录失败但不阻断流程，可以后续手动重试
        } else {
            tracing::info!("成功自动生成应付账单 (退货单 {})", return_no);
        }
    }

    /// 拒绝采购退货单
    pub async fn reject_return(
        &self,
        return_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无事务、无行锁，并发拒绝会基于过期状态通过状态检查后重复写入。
        let txn = (*self.db).begin().await?;

        // 1. 加 lock_exclusive 串行化并发状态变更
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购退货单 {}", return_id)))?;

        // 2. 检查状态
        if return_order.return_status.as_deref() != Some(pr_status::SUBMITTED) {
            return Err(AppError::business(format!(
                "退货单状态不允许拒绝，当前状态：{:?}",
                return_order.return_status
            )));
        }

        // 3. 更新状态 + 审计日志（事务内原子提交）
        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.return_status = Set(Some(pr_status::REJECTED.to_string()));
        return_active.reason_detail = Set(Some(reason));
        return_active.updated_at = Set(Utc::now());

        // reject_return 已在批次 59b 透传 user_id（原 TODO 已随实现落地移除）
        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            return_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 获取退货单列表
    pub async fn list_returns(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
        keyword: Option<String>,
        date_from: Option<String>,
        date_to: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<PurchaseReturnView>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        // 单次查询：实体 + 来源订单号 / 供应商名 / 创建人名（均为多对一 LEFT JOIN，不倍增行）。
        let mut query = purchase_return::Entity::find()
            .column_as(purchase_order::Column::OrderNo, "purchase_order_no")
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(user::Column::RealName, "created_by_name")
            .join(JoinType::LeftJoin, purchase_return::Relation::Order.def())
            .join(
                JoinType::LeftJoin,
                purchase_return::Relation::Supplier.def(),
            )
            .join(JoinType::LeftJoin, purchase_return::Relation::Creator.def());

        // V15 P0-S01：行级数据权限过滤（purchase_return 表有 created_by + department_id，支持完整 Dept）
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                purchase_return::Column::CreatedBy,
                purchase_return::Column::DepartmentId,
            );
        }

        if let Some(status) = status {
            query = query.filter(purchase_return::Column::ReturnStatus.eq(&status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_return::Column::SupplierId.eq(supplier_id));
        }
        // 关键字：匹配退货单号
        if let Some(kw) = keyword.as_deref().filter(|s| !s.is_empty()) {
            query = query.filter(purchase_return::Column::ReturnNo.like(safe_like_pattern(kw)));
        }
        // 退货日期范围
        if let Some(d) = date_from.as_deref().and_then(parse_date_bound) {
            query = query.filter(purchase_return::Column::ReturnDate.gte(d));
        }
        if let Some(d) = date_to.as_deref().and_then(parse_date_bound) {
            query = query.filter(purchase_return::Column::ReturnDate.lte(d));
        }

        // 批次 258 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(purchase_return::Column::CreatedAt, Order::Desc)
            .into_model::<PurchaseReturnView>()
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 获取退货单详情
    pub async fn get_return(
        &self,
        return_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购退货单 {}", return_id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // purchase_return 表有 created_by + department_id，支持完整 Dept
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, return_order.created_by, return_order.department_id) {
                return Err(AppError::permission_denied(format!(
                    "无权访问采购退货单 {}（数据范围限制）",
                    return_id
                )));
            }
        }

        Ok(return_order)
    }
}

// =====================================================
// 请求/响应 DTO
// =====================================================

/// 创建采购退货单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseReturnRequest {
    /// 入库单 ID
    pub receipt_id: Option<i32>,

    /// 采购订单 ID
    pub order_id: Option<i32>,

    /// 供应商 ID
    pub supplier_id: i32,

    /// 退货日期
    pub return_date: chrono::NaiveDate,

    /// 仓库 ID
    pub warehouse_id: Option<i32>,

    /// 部门 ID
    pub department_id: Option<i32>,

    /// 退货原因类型
    pub reason_type: String,

    /// 退货原因详情
    pub reason_detail: Option<String>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新采购退货单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseReturnRequest {
    pub reason_type: Option<String>,
    pub reason_detail: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateReturnItemRequest {
    pub line_no: i32,
    pub material_id: i32,
    pub quantity_ordered: Option<Decimal>,
    pub quantity_returned: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Option<Decimal>,
    pub discount_percent: Option<Decimal>,
    pub notes: Option<String>,
    /// 面料追溯维度：审批时按 (产品+色号+缸号+批次) 精确定位库存行扣减，
    /// 缺省即空串（对应白坯/单库存行产品）；同产品多批次时必须传入以唯一定位。
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateReturnItemRequest {
    pub line_no: Option<i32>,
    pub material_id: Option<i32>,
    pub quantity_returned: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub discount_percent: Option<Decimal>,
    pub notes: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sea_orm::FromQueryResult)]
pub struct PurchaseReturnItemDto {
    pub id: i32,
    pub return_id: i32,
    pub line_no: i32,
    pub material_id: i32,
    pub material_code: Option<String>,
    pub material_name: Option<String>,
    pub quantity_returned: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_percent: Decimal,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub notes: Option<String>,
}

impl PurchaseReturnService {
    /// 获取退货单明细列表
    pub async fn list_items(&self, return_id: i32) -> Result<Vec<PurchaseReturnItemDto>, AppError> {
        use sea_orm::{JoinType, RelationTrait};
        let items = purchase_return_item::Entity::find()
            .column_as(product::Column::Code, "material_code")
            .column_as(product::Column::Name, "material_name")
            .column_as(purchase_return_item::Column::ProductId, "material_id")
            .column_as(purchase_return_item::Column::Quantity, "quantity_returned")
            .column_as(purchase_return_item::Column::TaxPercent, "tax_rate")
            .join(
                JoinType::LeftJoin,
                purchase_return_item::Relation::Product.def(),
            )
            .filter(purchase_return_item::Column::ReturnId.eq(return_id))
            .order_by_asc(purchase_return_item::Column::LineNo)
            .into_model::<PurchaseReturnItemDto>()
            .all(&*self.db)
            .await?;

        Ok(items)
    }

    /// 添加退货单明细
    pub async fn create_item(
        &self,
        return_id: i32,
        req: CreateReturnItemRequest,
        user_id: i32,
    ) -> Result<purchase_return_item::Model, AppError> {
        let txn = self.db.begin().await?;

        // 验证主表状态（只有草稿可以修改明细，实际业务可能放宽，这里简化）
        let return_record = purchase_return::Entity::find_by_id(return_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货单 {}", return_id)))?;

        if return_record.return_status.as_deref() != Some(pr_status::DRAFT) {
            return Err(AppError::business(
                "只有草稿状态的退货单可以修改明细".to_string(),
            ));
        }

        let quantity = req.quantity_returned;
        let unit_price = req.unit_price;
        let discount_percent = req.discount_percent.unwrap_or(Decimal::ZERO);
        let tax_percent = req.tax_rate.unwrap_or(Decimal::ZERO);

        // 批次 97 P1-4 修复（v5 复审）：金额计算补 round_dp(2) 防止精度漂移
        let subtotal = (quantity * unit_price).round_dp(2);
        let discount_amount = (subtotal * (discount_percent / Decimal::new(100, 0))).round_dp(2);
        let taxable_amount = (subtotal - discount_amount).round_dp(2);
        let tax_amount = (taxable_amount * (tax_percent / Decimal::new(100, 0))).round_dp(2);
        let total_amount = (taxable_amount + tax_amount).round_dp(2);

        let item = purchase_return_item::ActiveModel {
            id: Default::default(),
            return_id: Set(return_id),
            line_no: Set(req.line_no),
            product_id: Set(req.material_id),
            quantity: Set(quantity),
            quantity_alt: Set(Decimal::ZERO),
            unit_price: Set(unit_price),
            unit_price_foreign: Set(unit_price),
            discount_percent: Set(discount_percent),
            tax_percent: Set(tax_percent),
            subtotal: Set(subtotal),
            tax_amount: Set(tax_amount),
            discount_amount: Set(discount_amount),
            total_amount: Set(total_amount),
            notes: Set(req.notes),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            // 面料行业追溯字段（D-P1-4）：由请求带入，缺省落空串（对齐 inventory_stock
            // dye_lot_no NULL→'' 归一口径），供审批按四维精确定位库存行扣减。
            color_no: Set(req.color_no.unwrap_or_default()),
            dye_lot_no: Set(req.dye_lot_no.unwrap_or_default()),
            batch_no: Set(req.batch_no.unwrap_or_default()),
        }
        .insert(&txn)
        .await?;

        self.update_return_totals(return_id, &txn, user_id).await?;
        txn.commit().await?;

        Ok(item)
    }

    /// 更新退货单明细（批次 101 v6 复审 P2-3 修复：原 update_with_audit 调用 Some(0) 占位符导致审计日志操作人为 0，；改为透传真实操作人 user_id。）
    pub async fn update_item(
        &self,
        item_id: i32,
        req: UpdateReturnItemRequest,
        user_id: i32,
    ) -> Result<purchase_return_item::Model, AppError> {
        let txn = self.db.begin().await?;

        let item = purchase_return_item::Entity::find_by_id(item_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货明细 {}", item_id)))?;

        let return_record = purchase_return::Entity::find_by_id(item.return_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货单 {}", item.return_id)))?;

        // 状态门校验：仅草稿状态可修改明细
        Self::validate_update_item(&return_record)?;

        // 解析请求字段并计算金额
        let (quantity, unit_price, discount_percent, tax_percent) =
            Self::resolve_item_params(&req, &item);
        let amounts =
            Self::compute_item_amounts(quantity, unit_price, discount_percent, tax_percent);

        // 构建 ActiveModel
        let active_item = Self::build_update_item_active(
            item,
            req,
            quantity,
            unit_price,
            discount_percent,
            tax_percent,
            amounts,
        );

        let updated_item = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_item,
            // 批次 101 v6 复审 P2-3：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        self.update_return_totals(updated_item.return_id, &txn, user_id)
            .await?;
        txn.commit().await?;

        Ok(updated_item)
    }

    /// 校验退货单状态：仅草稿状态可修改明细
    fn validate_update_item(return_record: &purchase_return::Model) -> Result<(), AppError> {
        if return_record.return_status.as_deref() != Some(pr_status::DRAFT) {
            return Err(AppError::business(
                "只有草稿状态的退货单可以修改明细".to_string(),
            ));
        }
        Ok(())
    }

    /// 解析请求中的明细参数，未提供则使用原值（返回 (quantity, unit_price, discount_percent, tax_percent)）
    fn resolve_item_params(
        req: &UpdateReturnItemRequest,
        item: &purchase_return_item::Model,
    ) -> (Decimal, Decimal, Decimal, Decimal) {
        let quantity = req.quantity_returned.unwrap_or(item.quantity);
        let unit_price = req.unit_price.unwrap_or(item.unit_price);
        let discount_percent = req.discount_percent.unwrap_or(item.discount_percent);
        let tax_percent = req.tax_rate.unwrap_or(item.tax_percent);
        (quantity, unit_price, discount_percent, tax_percent)
    }

    /// 计算明细金额（含 round_dp(2) 防精度漂移）（批次 97 P1-4 修复（v5 复审）：金额计算补 round_dp(2) 防止精度漂移）
    fn compute_item_amounts(
        quantity: Decimal,
        unit_price: Decimal,
        discount_percent: Decimal,
        tax_percent: Decimal,
    ) -> ItemAmounts {
        let subtotal = (quantity * unit_price).round_dp(2);
        let discount_amount = (subtotal * (discount_percent / Decimal::new(100, 0))).round_dp(2);
        let taxable_amount = (subtotal - discount_amount).round_dp(2);
        let tax_amount = (taxable_amount * (tax_percent / Decimal::new(100, 0))).round_dp(2);
        let total_amount = (taxable_amount + tax_amount).round_dp(2);
        ItemAmounts {
            subtotal,
            discount_amount,
            tax_amount,
            total_amount,
        }
    }

    /// 构建更新后的明细 ActiveModel
    fn build_update_item_active(
        item: purchase_return_item::Model,
        req: UpdateReturnItemRequest,
        quantity: Decimal,
        unit_price: Decimal,
        discount_percent: Decimal,
        tax_percent: Decimal,
        amounts: ItemAmounts,
    ) -> purchase_return_item::ActiveModel {
        let mut active_item: purchase_return_item::ActiveModel = item.clone().into();

        if let Some(line_no) = req.line_no {
            active_item.line_no = Set(line_no);
        }
        if let Some(material_id) = req.material_id {
            active_item.product_id = Set(material_id);
        }

        active_item.quantity = Set(quantity);
        active_item.unit_price = Set(unit_price);
        active_item.unit_price_foreign = Set(unit_price);
        active_item.discount_percent = Set(discount_percent);
        active_item.tax_percent = Set(tax_percent);

        active_item.subtotal = Set(amounts.subtotal);
        active_item.discount_amount = Set(amounts.discount_amount);
        active_item.tax_amount = Set(amounts.tax_amount);
        active_item.total_amount = Set(amounts.total_amount);

        if let Some(notes) = req.notes {
            active_item.notes = Set(Some(notes));
        }

        if let Some(color_no) = req.color_no {
            active_item.color_no = Set(color_no);
        }
        if let Some(dye_lot_no) = req.dye_lot_no {
            active_item.dye_lot_no = Set(dye_lot_no);
        }
        if let Some(batch_no) = req.batch_no {
            active_item.batch_no = Set(batch_no);
        }

        active_item.updated_at = Set(Utc::now());
        active_item
    }

    /// 删除退货单明细（批次 101 v6 复审 P2-5 修复：透传 user_id 给 update_return_totals，使合计重算的审计日志；操作人为真实用户（原 Some(0) 占位导致审计追溯失效）。）
    pub async fn delete_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = self.db.begin().await?;

        let item = purchase_return_item::Entity::find_by_id(item_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货明细 {}", item_id)))?;

        let return_record = purchase_return::Entity::find_by_id(item.return_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货单 {}", item.return_id)))?;

        if return_record.return_status.as_deref() != Some(pr_status::DRAFT) {
            return Err(AppError::business(
                "只有草稿状态的退货单可以修改明细".to_string(),
            ));
        }

        purchase_return_item::Entity::delete_by_id(item_id)
            .exec(&txn)
            .await?;

        self.update_return_totals(item.return_id, &txn, user_id)
            .await?;
        txn.commit().await?;

        Ok(())
    }

    /// 删除采购退货单（批次 101 v6 复审 P2-4 修复：原 delete_with_audit 调用 Some(0) 占位符导致审计日志操作人为 0，；改为透传真实操作人 user_id。）
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = self.db.begin().await?;

        let ret = purchase_return::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("Return not found"))?;
        if ret.return_status.as_deref() != Some(pr_status::DRAFT) {
            return Err(AppError::business(
                "Only DRAFT returns can be deleted".to_string(),
            ));
        }

        purchase_return_item::Entity::delete_many()
            .filter(purchase_return_item::Column::ReturnId.eq(id))
            .exec(&txn)
            .await?;

        // P0 8-3 修复：delete 操作补审计日志
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            purchase_return::Entity,
            _,
        >(
            &txn,
            "purchase_return",
            id,
            // 批次 101 v6 复审 P2-4：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 更新主单合计金额和数量
    /// 批次 101 v6 复审 P2-5 修复：原 update_with_audit 调用 Some(0) 占位符导致审计日志操作人为 0，；改为透传真实操作人 user_id。user_id 由调用方（create_item / update_item / delete_item）注入。
    async fn update_return_totals(
        &self,
        return_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        user_id: i32,
    ) -> Result<(), AppError> {
        let items = purchase_return_item::Entity::find()
            .filter(purchase_return_item::Column::ReturnId.eq(return_id))
            .all(txn)
            .await?;

        let mut total_quantity = Decimal::ZERO;
        let mut total_quantity_alt = Decimal::ZERO;
        let mut total_amount = Decimal::ZERO;

        for item in items {
            total_quantity += item.quantity;
            total_quantity_alt += item.quantity_alt;
            total_amount += item.total_amount;
        }

        let return_record = purchase_return::Entity::find_by_id(return_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货单 {}", return_id)))?;

        let mut active_return: purchase_return::ActiveModel = return_record.into();
        active_return.total_quantity = Set(Some(total_quantity));
        active_return.total_quantity_alt = Set(Some(total_quantity_alt));
        active_return.total_amount = Set(Some(total_amount));
        active_return.updated_at = Set(Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_return,
            // 批次 101 v6 复审 P2-5：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        Ok(())
    }
}
