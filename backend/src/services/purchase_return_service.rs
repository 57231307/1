//! 采购退货 Service
//!
//! 采购退货服务层，负责采购退货的核心业务逻辑
//!
// 批次 101 v6 复审 P2 修复：update_item / delete / update_return_totals 三处审计操作人
// Some(0) 占位符改为真实 user_id，调用方 create_item / update_item / delete_item / delete
// 同步添加 user_id 参数透传（P2-3 / P2-4 / P2-5）。

use crate::models::status::purchase_return as pr_status;
use crate::models::{inventory_stock, product, purchase_return, purchase_return_item};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::inventory_stock_query::RecordTransactionArgs;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
// 批次 258 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

/// 采购退货服务
pub struct PurchaseReturnService {
    db: Arc<DatabaseConnection>,
}

/// 退货扣减库存上下文：approve_return 循环内逐项扣减时复用的字段
///
/// D08-1 第二梯队拆分：将逐项扣减所需的 txn/仓库/单号/单 ID/操作人封装为单一上下文，
/// 避免逐项 helper 出现 7+ 参数触发 clippy too_many_arguments。
struct ReturnItemDeductionCtx<'a> {
    txn: &'a sea_orm::DatabaseTransaction,
    warehouse_id: i32,
    return_no: &'a str,
    return_id: i32,
    user_id: i32,
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

        // 批次 103 P0-4 修复：删除过时 TODO 注释（submit_return 已在批次 59b 透传 user_id）
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

        let return_order =
            Self::load_and_validate_return_for_approval(&txn, return_id).await?;
        let return_order =
            Self::update_return_status_to_approved(&txn, return_order, user_id).await?;

        // 1. 扣减库存（在事务内执行，保证原子性）
        let (items, stock_map) =
            Self::load_return_items_with_stock_map(&txn, &return_order).await?;
        Self::deduct_stock_for_return_items(
            &txn,
            &return_order,
            items,
            &stock_map,
            user_id,
            &mut pending_events,
        )
        .await?;

        // 2. 提交事务（库存扣减和状态更新在同一事务内）
        txn.commit().await?;

        // P0 5-2 修复：commit 成功后统一发布库存流水事件，避免事务回滚时幻事件
        for ev in pending_events {
            EVENT_BUS.publish(ev);
        }

        // 3. 自动生成应付红字账单（冲销）- 在事务外执行，失败不影响库存扣减
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

    /// 加载退货明细 + 批量加载库存记录
    ///
    /// v14 批次 41 修复：批量查询所有退货明细对应的库存记录（同一 warehouse_id），
    /// 避免循环内逐个调用 find_by_product_and_warehouse_txn（N+1 查询）。
    /// 若 return_order.warehouse_id 为 None，stock_map 为空，循环内逐项 continue（保留原行为）。
    async fn load_return_items_with_stock_map(
        txn: &sea_orm::DatabaseTransaction,
        return_order: &purchase_return::Model,
    ) -> Result<(
        Vec<purchase_return_item::Model>,
        std::collections::HashMap<i32, inventory_stock::Model>,
    ), AppError> {
        let items = purchase_return_item::Entity::find()
            .filter(purchase_return_item::Column::ReturnId.eq(return_order.id))
            .all(txn)
            .await?;

        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            match return_order.warehouse_id {
                Some(warehouse_id) => {
                    let product_ids: Vec<i32> =
                        items.iter().map(|item| item.product_id).collect();
                    if product_ids.is_empty() {
                        std::collections::HashMap::new()
                    } else {
                        let stocks = inventory_stock::Entity::find()
                            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
                            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
                            .all(txn)
                            .await?;
                        stocks.into_iter().map(|s| (s.product_id, s)).collect()
                    }
                }
                None => std::collections::HashMap::new(),
            };

        Ok((items, stock_map))
    }

    /// 循环扣减每项退货明细的库存
    async fn deduct_stock_for_return_items(
        txn: &sea_orm::DatabaseTransaction,
        return_order: &purchase_return::Model,
        items: Vec<purchase_return_item::Model>,
        stock_map: &std::collections::HashMap<i32, inventory_stock::Model>,
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
            let Some(s) = stock_map.get(&item.product_id).cloned() else {
                return Err(AppError::business(format!(
                    "产品 {} 在仓库 {} 没有库存记录，无法退货",
                    item.product_id, warehouse_id
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
            tracing::error!(
                "自动生成应付账单失败 (退货单 {}): {}",
                return_no,
                e
            );
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

        // 批次 103 P0-4 修复：删除过时 TODO 注释（reject_return 已在批次 59b 透传 user_id）
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
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<purchase_return::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let mut query = purchase_return::Entity::find();

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

        // 批次 258 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(purchase_return::Column::CreatedAt, Order::Desc)
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
            // v14 批次 417：面料行业追溯字段（D-P1-4），使用 NotSet 让 DB 默认值处理
            color_no: sea_orm::ActiveValue::NotSet,
            dye_lot_no: sea_orm::ActiveValue::NotSet,
            batch_no: sea_orm::ActiveValue::NotSet,
        }
        .insert(&txn)
        .await?;

        self.update_return_totals(return_id, &txn, user_id).await?;
        txn.commit().await?;

        Ok(item)
    }

    /// 更新退货单明细
    ///
    /// 批次 101 v6 复审 P2-3 修复：原 update_with_audit 调用 Some(0) 占位符导致审计日志操作人为 0，
    /// 改为透传真实操作人 user_id。
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

        if return_record.return_status.as_deref() != Some(pr_status::DRAFT) {
            return Err(AppError::business(
                "只有草稿状态的退货单可以修改明细".to_string(),
            ));
        }

        let mut active_item: purchase_return_item::ActiveModel = item.clone().into();

        if let Some(line_no) = req.line_no {
            active_item.line_no = Set(line_no);
        }
        if let Some(material_id) = req.material_id {
            active_item.product_id = Set(material_id);
        }

        let quantity = req.quantity_returned.unwrap_or(item.quantity);
        let unit_price = req.unit_price.unwrap_or(item.unit_price);
        let discount_percent = req.discount_percent.unwrap_or(item.discount_percent);
        let tax_percent = req.tax_rate.unwrap_or(item.tax_percent);

        active_item.quantity = Set(quantity);
        active_item.unit_price = Set(unit_price);
        active_item.unit_price_foreign = Set(unit_price);
        active_item.discount_percent = Set(discount_percent);
        active_item.tax_percent = Set(tax_percent);

        // 批次 97 P1-4 修复（v5 复审）：金额计算补 round_dp(2) 防止精度漂移
        let subtotal = (quantity * unit_price).round_dp(2);
        let discount_amount = (subtotal * (discount_percent / Decimal::new(100, 0))).round_dp(2);
        let taxable_amount = (subtotal - discount_amount).round_dp(2);
        let tax_amount = (taxable_amount * (tax_percent / Decimal::new(100, 0))).round_dp(2);
        let total_amount = (taxable_amount + tax_amount).round_dp(2);

        active_item.subtotal = Set(subtotal);
        active_item.discount_amount = Set(discount_amount);
        active_item.tax_amount = Set(tax_amount);
        active_item.total_amount = Set(total_amount);

        if let Some(notes) = req.notes {
            active_item.notes = Set(Some(notes));
        }

        active_item.updated_at = Set(Utc::now());

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

    /// 删除退货单明细
    ///
    /// 批次 101 v6 复审 P2-5 修复：透传 user_id 给 update_return_totals，使合计重算的审计日志
    /// 操作人为真实用户（原 Some(0) 占位导致审计追溯失效）。
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

    /// 删除采购退货单
    ///
    /// 批次 101 v6 复审 P2-4 修复：原 delete_with_audit 调用 Some(0) 占位符导致审计日志操作人为 0，
    /// 改为透传真实操作人 user_id。
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
    ///
    /// 批次 101 v6 复审 P2-5 修复：原 update_with_audit 调用 Some(0) 占位符导致审计日志操作人为 0，
    /// 改为透传真实操作人 user_id。user_id 由调用方（create_item / update_item / delete_item）注入。
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
