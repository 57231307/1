//! 销售退货 Service
//!
//! 销售退货服务层，负责销售退货的核心业务逻辑

use crate::models::{inventory_stock, product, sales_return, sales_return_item};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use std::sync::Arc;

use super::ar_invoice_service::{ArInvoiceService, CreateArInvoiceRequest};
use super::inventory_stock_service::InventoryStockService;

/// 创建销售退货请求
#[derive(Deserialize)]
pub struct CreateSalesReturnRequest {
    pub order_id: Option<i32>,
    pub customer_id: i32,
    pub return_date: chrono::NaiveDate,
    pub warehouse_id: i32,
    pub reason_type: String,
    pub reason_detail: Option<String>,
    pub notes: Option<String>,
}

/// 更新销售退货请求
#[derive(Deserialize)]
pub struct UpdateSalesReturnRequest {
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub return_date: Option<chrono::NaiveDate>,
    pub warehouse_id: Option<i32>,
    pub reason_type: Option<String>,
    pub reason_detail: Option<String>,
    pub notes: Option<String>,
}

/// 添加退货明细项请求
#[derive(Deserialize)]
pub struct CreateSalesReturnItemRequest {
    pub product_id: i32,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub reason: Option<String>,
}

/// 销售退货服务
pub struct SalesReturnService {
    db: Arc<DatabaseConnection>,
}

impl SalesReturnService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn update_return_totals(
        &self,
        return_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        user_id: i32,
    ) -> Result<(), AppError> {
        use sea_orm::ColumnTrait;
        let items = crate::models::sales_return_item::Entity::find()
            .filter(crate::models::sales_return_item::Column::ReturnId.eq(return_id))
            .all(txn)
            .await?;

        let mut total = rust_decimal::Decimal::new(0, 0);
        for item in items {
            // Because sales_return_item might not have `amount`, we multiply quantity by a unit price or assume it's pre-calculated if the field exists.
            // Let's check what fields are actually in sales_return_item
            // Wait, sales_return_item doesn't have an `amount` field. We must use unit_price * quantity.
            let qty = item.quantity;
            let price = item.unit_price;
            total += qty * price;
        }

        let return_order = crate::models::sales_return::Entity::find_by_id(return_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货单 {}", return_id)))?;

        let mut return_active: crate::models::sales_return::ActiveModel = return_order.into();
        return_active.total_amount = sea_orm::ActiveValue::Set(total);
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            return_active,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    // 生成退货单号
    // 格式：SR + 年月日 + 三位序号（SR20260315001）
    crate::impl_generate_no!(
        generate_return_no,
        "SR",
        sales_return::Entity,
        sales_return::Column::ReturnNo
    );

    /// 创建销售退货单
    pub async fn create_return(
        &self,
        req: CreateSalesReturnRequest,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let return_no = self.generate_return_no().await?;

        // 将 reason_type 和 reason_detail 组合成 reason 字段
        let reason = if let Some(detail) = &req.reason_detail {
            format!("{}: {}", req.reason_type, detail)
        } else {
            req.reason_type
        };

        let return_order = sales_return::ActiveModel {
            return_no: Set(return_no),
            sales_order_id: Set(req.order_id),
            customer_id: Set(req.customer_id),
            return_date: Set(req.return_date),
            warehouse_id: Set(req.warehouse_id),
            reason: Set(reason),
            status: Set("DRAFT".to_string()),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(req.notes),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 添加退货明细项
    pub async fn add_return_item(
        &self,
        return_id: i32,
        req: CreateSalesReturnItemRequest,
        user_id: i32,
    ) -> Result<sales_return_item::Model, AppError> {
        // P1-6 修复（批次 79 v1 复审）：状态门 + insert 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门用 self.db 裸查询无锁、insert 用 txn，
        // 并发场景下可能在状态检查通过后、insert 之前发生 approve/submit 状态变更，
        // 导致已审批退货单被追加明细。
        let txn = (*self.db).begin().await?;

        // 验证退货单存在且为草稿状态（加 lock_exclusive 串行化并发状态变更）
        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "DRAFT" {
            return Err(AppError::business(format!(
                "退货单状态不允许添加明细，当前状态：{}",
                return_order.status
            )));
        }

        let item = sales_return_item::ActiveModel {
            return_id: Set(return_id),
            product_id: Set(req.product_id),
            quantity: Set(req.quantity),
            unit_price: Set(req.unit_price),
            notes: Set(req.reason),
            quantity_alt: Set(Decimal::ZERO),
            ..Default::default()
        };

        let item = item.insert(&txn).await?;

        // 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(return_id, &txn, user_id).await?;

        txn.commit().await?;

        Ok(item)
    }

    /// 更新销售退货单
    pub async fn update_return(
        &self,
        return_id: i32,
        req: UpdateSalesReturnRequest,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        // P1-7 修复（批次 79 v1 复审）：状态门 + update 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门用 self.db 裸查询、update_with_audit 也用 self.db，无事务边界，
        // 并发场景下可能在状态检查通过后、update 之前发生状态变更导致已审批单被篡改。
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "DRAFT" {
            return Err(AppError::business(format!(
                "退货单状态不允许修改，当前状态：{}",
                return_order.status
            )));
        }

        let mut active_model: sales_return::ActiveModel = return_order.into();

        if let Some(order_id) = req.order_id {
            active_model.sales_order_id = Set(Some(order_id));
        }
        if let Some(customer_id) = req.customer_id {
            active_model.customer_id = Set(customer_id);
        }
        if let Some(return_date) = req.return_date {
            active_model.return_date = Set(return_date);
        }
        if let Some(warehouse_id) = req.warehouse_id {
            active_model.warehouse_id = Set(warehouse_id);
        }
        if let Some(reason_type) = req.reason_type {
            let reason = if let Some(detail) = req.reason_detail {
                format!("{}: {}", reason_type, detail)
            } else {
                reason_type
            };
            active_model.reason = Set(reason);
        }
        if let Some(notes) = req.notes {
            active_model.remarks = Set(Some(notes));
        }

        active_model.updated_at = Set(Utc::now());
        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 提交销售退货单
    pub async fn submit_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现先在事务外用 &*self.db 裸查询退货单状态，再 begin() 开启事务，
        // 并发 submit_return 均通过状态检查后基于过期状态写入，导致状态门失效。
        let txn = (*self.db).begin().await?;

        // 获取退货单（加 lock_exclusive 串行化并发状态变更）
        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "DRAFT" {
            return Err(AppError::business(format!(
                "退货单状态不允许提交，当前状态：{}",
                return_order.status
            )));
        }

        // 验证是否包含明细
        // 批次 27 v7 P1 修复：事务边界泄漏，原实现 count 用 &*self.db 裸查询
        // 存在 TOCTOU 风险（并发 submit + add_item 时计数读快照不一致，可绕过"明细非空"校验）
        let items_count = sales_return_item::Entity::find()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .count(&txn)
            .await?;

        if items_count == 0 {
            return Err(AppError::business("退货单没有明细，无法提交".to_string()));
        }

        // 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(return_id, &txn, user_id).await?;

        // 更新状态为已提交
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        let mut active_model: sales_return::ActiveModel = return_order.into();
        active_model.status = Set("SUBMITTED".to_string());
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 审批销售退货单
    ///
    /// P2 1-5 修复：原函数 167 行混合 6 职责（lock+校验+总金额更新+批量库存入库+状态变更+AR 生成），
    /// 拆为 validate_and_lock_submitted_txn / apply_stock_inbound_txn / mark_approved_txn / generate_red_ar_txn 4 个私有方法
    pub async fn approve_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. lock_exclusive + 状态校验 + 获取明细
        let (return_order, items) =
            Self::validate_and_lock_submitted_txn(&txn, return_id).await?;

        // 2. 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(return_id, &txn, user_id).await?;

        // 3. 批量库存入库
        self.apply_stock_inbound_txn(&txn, &return_order, &items, user_id)
            .await?;

        // 4. 状态变更（APPROVED）
        let return_order = Self::mark_approved_txn(&txn, return_order, user_id).await?;

        // 5. P1 5-5/1-3 修复（批次 62）：红字应收单生成移入事务内，失败则整体回滚
        // 原实现在 commit 后调用 ar_invoice_service.create，但 create 强制 amount > 0，
        // 红字金额（负数）注定失败，且失败仅 tracing::error 不回滚，导致账实不符。
        // 改用 create_credit_memo（支持负金额 + 外部事务 + 幂等检查），在 commit 前调用。
        Self::generate_red_ar_txn(&self.db, &txn, &return_order, user_id).await?;

        txn.commit().await?;

        tracing::info!(
            "成功自动生成红字应收单 (退货单 {})",
            return_order.return_no
        );

        Ok(return_order)
    }

    /// P2 1-5 修复：lock_exclusive + 状态校验 + 获取明细（从 approve_return 抽取）
    ///
    /// 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    async fn validate_and_lock_submitted_txn(
        txn: &sea_orm::DatabaseTransaction,
        return_id: i32,
    ) -> Result<(sales_return::Model, Vec<sales_return_item::Model>), AppError> {
        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "SUBMITTED" {
            return Err(AppError::business(format!(
                "退货单状态不允许审批，当前状态：{}",
                return_order.status
            )));
        }

        // 获取明细记录
        let items = sales_return_item::Entity::find()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .all(txn)
            .await?;

        Ok((return_order, items))
    }

    /// P2 1-5 修复：批量库存入库（从 approve_return 抽取）
    ///
    /// 批量获取商品信息和库存记录（优化 N+1 查询），循环更新或创建库存记录并记录 SALES_RETURN 流水
    async fn apply_stock_inbound_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        return_order: &sales_return::Model,
        items: &[sales_return_item::Model],
        user_id: i32,
    ) -> Result<(), AppError> {
        // 保留原行为：record_transaction 内部使用 self.db（非 txn 路径）
        // 注：这是批次 27 v7 P1 修复中提及的遗留事务边界问题，本次 1-5 拆分不改变语义
        let stock_service = InventoryStockService::new(self.db.clone());

        // 批量获取商品信息和库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids.clone()))
            .all(txn)
            .await?;
        let product_map: std::collections::HashMap<i32, product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();

        let stocks = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(return_order.warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        for item in items {
            // 获取商品信息
            let _product_info = product_map
                .get(&item.product_id)
                .ok_or_else(|| AppError::not_found(format!("商品 {} 不存在", item.product_id)))?;

            // 查找是否已有库存记录
            let stock = stock_map.get(&item.product_id);

            let (batch_no, color_no, grade) = if let Some(s) = stock {
                (s.batch_no.clone(), s.color_no.clone(), s.grade.clone())
            } else {
                (String::new(), String::new(), String::from("A"))
            };

            if let Some(s) = stock {
                // 更新现有库存
                let new_qty = s.quantity_on_hand + item.quantity;
                let new_avail = s.quantity_available + item.quantity;
                let mut stock_update: inventory_stock::ActiveModel = s.clone().into();
                stock_update.quantity_on_hand = Set(new_qty);
                stock_update.quantity_available = Set(new_avail);
                stock_update.updated_at = Set(Utc::now());
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    txn,
                    "auto_audit",
                    stock_update,
                    // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
                    Some(user_id),
                )
                .await?;
            } else {
                // 创建新库存记录
                let new_stock = inventory_stock::ActiveModel {
                    warehouse_id: Set(return_order.warehouse_id),
                    product_id: Set(item.product_id),
                    batch_no: Set(batch_no.clone()),
                    color_no: Set(color_no.clone()),
                    grade: Set(grade.clone()),
                    quantity_on_hand: Set(item.quantity),
                    quantity_available: Set(item.quantity),
                    quantity_reserved: Set(Decimal::ZERO),
                    version: Set(0),
                    ..Default::default()
                };
                new_stock.insert(txn).await?;
            }

            // 增加库存交易记录
            stock_service
                .record_transaction(
                    "SALES_RETURN".to_string(),
                    item.product_id,
                    return_order.warehouse_id,
                    batch_no.clone(),
                    color_no.clone(),
                    Some(batch_no.clone()), // dye_lot_no
                    grade.clone(),
                    item.quantity, // 正数，表示入库
                    item.quantity_alt,
                    Some("SALES_RETURN".to_string()),
                    Some(return_order.return_no.clone()),
                    Some(return_order.id),
                    None,
                    None,
                    None,
                    None,
                    Some("销售退货入库".to_string()),
                    Some(user_id),
                )
                .await?;
        }

        Ok(())
    }

    /// P2 1-5 修复：状态变更（APPROVED）（从 approve_return 抽取）
    async fn mark_approved_txn(
        txn: &sea_orm::DatabaseTransaction,
        return_order: sales_return::Model,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let mut active_model: sales_return::ActiveModel = return_order.into();
        active_model.status = Set("APPROVED".to_string());
        active_model.approved_by = Set(Some(user_id));
        active_model.approved_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        Ok(return_order)
    }

    /// P2 1-5 修复：红字应收单生成（从 approve_return 抽取）
    ///
    /// P1 5-5/1-3 修复（批次 62）：红字应收单生成移入事务内，失败则整体回滚
    /// 使用 create_credit_memo（支持负金额 + 外部事务 + 幂等检查）
    async fn generate_red_ar_txn(
        db: &Arc<DatabaseConnection>,
        txn: &sea_orm::DatabaseTransaction,
        return_order: &sales_return::Model,
        user_id: i32,
    ) -> Result<(), AppError> {
        let ar_invoice_service = ArInvoiceService::new(db.clone());
        let invoice_date = Utc::now().date_naive();
        let due_date = invoice_date + chrono::Duration::days(30);
        let ar_request = CreateArInvoiceRequest {
            invoice_date: Some(invoice_date),
            due_date: Some(due_date),
            customer_id: Some(return_order.customer_id),
            customer_name: None,
            source_type: Some("SALES_RETURN".to_string()),
            source_bill_id: Some(return_order.id),
            source_bill_no: Some(return_order.return_no.clone()),
            invoice_amount: Some(-return_order.total_amount), // 红字应收单
            batch_no: None,
            color_no: None,
            sales_order_no: None,
        };

        ar_invoice_service
            .create_credit_memo(ar_request, user_id, txn)
            .await?;

        Ok(())
    }

    /// 获取退货单详情
    pub async fn get_return(&self, return_id: i32) -> Result<sales_return::Model, AppError> {
        sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))
    }

    /// 删除退货单
    // 批次 93 P1-7 修复：补 user_id 参数 + lock_exclusive + 状态门移入 txn + 审计 user_id
    pub async fn delete_return(&self, return_id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 93 P1-7 修复：状态门 + delete 移入同一事务，补 lock_exclusive 串行化并发
        // 原实现 find_by_id 在 self.db → 状态门 → begin txn，
        // 状态门在事务外，并发 delete + submit 会竞态绕过 DRAFT 状态门控。
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        // 状态门在 txn 内，基于 lock_exclusive 读出的 model
        if return_order.status != "DRAFT" {
            return Err(AppError::business(format!(
                "退货单状态不允许删除，当前状态：{}",
                return_order.status
            )));
        }

        // 先删除明细
        sales_return_item::Entity::delete_many()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .exec(&txn)
            .await?;

        // 再删除退货单（P0 8-3 修复：补审计日志；批次 93 P1-7：user_id 从 handler AuthContext 注入）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            sales_return::Entity,
            _,
        >(&txn, "sales_return", return_id, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 拒绝退货单
    pub async fn reject_return(
        &self,
        return_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防止并发拒绝同一退货单导致状态不一致
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "SUBMITTED" {
            return Err(AppError::business(format!(
                "退货单状态不允许拒绝，当前状态：{}",
                return_order.status
            )));
        }

        let mut active_model: sales_return::ActiveModel = return_order.into();
        active_model.status = Set("REJECTED".to_string());
        active_model.rejected_reason = Set(Some(reason));
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 执行退货单（完成退货流程）
    pub async fn execute_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防止并发执行同一退货单导致状态不一致
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "APPROVED" {
            return Err(AppError::business(format!(
                "退货单状态不允许执行，当前状态：{}",
                return_order.status
            )));
        }

        let mut active_model: sales_return::ActiveModel = return_order.into();
        active_model.status = Set("COMPLETED".to_string());
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 获取退货单明细列表
    pub async fn list_return_items(
        &self,
        return_id: i32,
    ) -> Result<Vec<sales_return_item::Model>, AppError> {
        let items = sales_return_item::Entity::find()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .order_by_asc(sales_return_item::Column::LineNo)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 更新退货单明细
    pub async fn update_return_item(
        &self,
        item_id: i32,
        quantity: Option<Decimal>,
        unit_price: Option<Decimal>,
        reason: Option<String>,
        user_id: i32,
    ) -> Result<sales_return_item::Model, AppError> {
        let item = sales_return_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货明细 {}", item_id)))?;

        let txn = (*self.db).begin().await?;

        let mut active_model: sales_return_item::ActiveModel = item.into();
        if let Some(qty) = quantity {
            active_model.quantity = Set(qty);
        }
        if let Some(price) = unit_price {
            active_model.unit_price = Set(price);
        }
        if let Some(r) = reason {
            active_model.notes = Set(Some(r));
        }
        active_model.updated_at = Set(Utc::now());

        let item = active_model.insert(&txn).await?;

        // 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(item.return_id, &txn, user_id).await?;

        txn.commit().await?;
        Ok(item)
    }

    /// 删除退货单明细
    // 批次 93 P1-8 修复：find + delete 移入同一事务，补 lock_exclusive 串行化并发
    pub async fn delete_return_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 93 P1-8 修复：find 移入 txn + lock_exclusive，消除 TOCTOU 风险
        // 原实现 find_by_id 在 self.db → begin txn → delete_by_id 在 txn，
        // find 与 delete 跨事务边界，并发删除同一明细可能双写 / return_id 读取过期。
        let txn = (*self.db).begin().await?;

        let item = sales_return_item::Entity::find_by_id(item_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货明细 {}", item_id)))?;

        // 批次 94 P2-6 修复：用 delete_with_audit 记录审计日志（原 delete_by_id 无审计）
        // delete_with_audit 内部 find_by_id + delete + 写审计日志；行已被 lock_exclusive 锁定，重复查询安全
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            sales_return_item::Entity,
            _,
        >(&txn, "sales_return_item", item_id, Some(user_id))
        .await?;

        // 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(item.return_id, &txn, user_id).await?;

        txn.commit().await?;
        Ok(())
    }

    /// 获取列表
    pub async fn list_returns(
        &self,
        return_no: Option<String>,
        status: Option<String>,
        customer_id: Option<i32>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<sales_return::Model>, u64), AppError> {
        let mut query = sales_return::Entity::find();

        if let Some(no) = return_no {
            query = query.filter(sales_return::Column::ReturnNo.contains(&no));
        }

        if let Some(s) = status {
            query = query.filter(sales_return::Column::Status.eq(s));
        }

        if let Some(id) = customer_id {
            query = query.filter(sales_return::Column::CustomerId.eq(id));
        }

        let paginator = query
            .order_by_desc(sales_return::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        // 使用统一分页辅助函数，并行执行分页查询与总数统计
        let (items, total) = paginate_with_total(paginator, page).await?;

        Ok((items, total))
    }
}
