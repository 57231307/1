//! 采购入库 Service
//!
//! 采购入库服务层，负责采购入库的核心业务逻辑
//! 包含入库单创建、确认、更新等全流程管理

use crate::models::{purchase_receipt, purchase_receipt_item};
use crate::services::purchase_receipt_dto::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, UpdatePurchaseReceiptRequest,
    UpdateReceiptItemRequest,
};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;

/// 采购入库服务
pub struct PurchaseReceiptService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseReceiptService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成入库单号
    // 格式：GR + 年月日 + 三位序号（GR20260315001）
    crate::impl_generate_no!(
        generate_receipt_no,
        "PR",
        purchase_receipt::Entity,
        purchase_receipt::Column::ReceiptNo
    );

    /// 创建采购入库单（含明细）
    pub async fn create_receipt(
        &self,
        req: CreatePurchaseReceiptRequest,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成入库单号
        let receipt_no = self.generate_receipt_no().await?;

        // 2. 创建入库单主表
        let receipt = purchase_receipt::ActiveModel {
            receipt_no: Set(receipt_no),
            order_id: Set(req.order_id),
            supplier_id: Set(req.supplier_id),
            receipt_date: Set(req.receipt_date),
            warehouse_id: Set(req.warehouse_id),
            department_id: Set(req.department_id),
            receiver_id: Set(Some(user_id)),
            inspector_id: Set(req.inspector_id),
            inspection_status: Set("PENDING".to_string()),
            receipt_status: Set("DRAFT".to_string()),
            notes: Set(req.notes),
            attachment_urls: Set(req.attachment_urls),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 3. 创建入库明细
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);
        let mut total_amount = Decimal::new(0, 0);

        for item_req in req.items {
            let amount =
                item_req.quantity * item_req.unit_price.unwrap_or_else(|| Decimal::new(0, 0));
            total_quantity += item_req.quantity;
            total_quantity_alt += item_req.quantity_alt;

            total_amount += amount;

            purchase_receipt_item::ActiveModel {
                receipt_id: Set(receipt.id),
                order_item_id: Set(item_req.order_item_id),
                product_id: Set(item_req.material_id),
                quantity: Set(item_req.quantity),
                quantity_alt: Set(Some(item_req.quantity_alt)),
                unit_price: Set(Some(
                    item_req.unit_price.unwrap_or_else(|| Decimal::new(0, 0)),
                )),
                amount: Set(Some(amount)),
                notes: Set(item_req.notes),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }

        // 4. 更新入库单总金额和数量
        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();
        receipt_active.total_quantity = Set(total_quantity);
        receipt_active.total_quantity_alt = Set(total_quantity_alt);
        receipt_active.total_amount = Set(total_amount);
        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            receipt_active,
            Some(0),
        )
        .await?;

        // 5. 提交事务
        txn.commit().await?;

        Ok(receipt)
    }

    /// 更新采购入库单（仅草稿状态）
    pub async fn update_receipt(
        &self,
        receipt_id: i32,
        req: UpdatePurchaseReceiptRequest,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        // 1. 查询入库单
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查状态
        if receipt.receipt_status != "DRAFT" {
            return Err(AppError::business(format!(
                "入库单状态不允许修改，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 3. 检查权限
        if receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能修改自己创建的入库单".to_string(),
            ));
        }

        // 4. 更新入库单
        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();

        if let Some(supplier_id) = req.supplier_id {
            receipt_active.supplier_id = Set(supplier_id);
        }
        if let Some(receipt_date) = req.receipt_date {
            receipt_active.receipt_date = Set(receipt_date);
        }
        if let Some(department_id) = req.department_id {
            receipt_active.department_id = Set(Some(department_id));
        }
        if let Some(inspector_id) = req.inspector_id {
            receipt_active.inspector_id = Set(Some(inspector_id));
        }
        if let Some(notes) = req.notes {
            receipt_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            receipt_active.attachment_urls = Set(Some(attachment_urls));
        }

        receipt_active.updated_by = Set(Some(user_id));

        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            receipt_active,
            Some(0),
        )
        .await?;

        Ok(receipt)
    }

    /// 删除采购入库单（仅 DRAFT 状态）
    pub async fn delete_receipt(&self, receipt_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询入库单
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查状态
        if receipt.receipt_status != "DRAFT" {
            return Err(AppError::business(format!(
                "入库单状态不允许删除，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 3. 检查权限
        if receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能删除自己创建的入库单".to_string(),
            ));
        }

        let txn = (*self.db).begin().await?;

        // 4. 先删除明细
        purchase_receipt_item::Entity::delete_many()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .exec(&txn)
            .await?;

        // 5. 删除入库单
        purchase_receipt::Entity::delete_by_id(receipt_id)
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 确认采购入库单
    ///
    /// 批次 16（2026-06-28）：入库单状态门查询加 lock_exclusive，
    /// 防止并发 confirm_receipt 同一入库单导致重复入库 + 重复生成应付账单 + 重复累加采购单已收数量。
    /// 原状态门无锁，两并发 confirm 均通过 DRAFT 检查，第二个 confirm 重复执行库存入库与
    /// order_item received_quantity 累加，commit 后还会重复触发 auto_generate_from_receipt 生成应付账单。
    pub async fn confirm_receipt(
        &self,
        receipt_id: i32,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询入库单（加 lock_exclusive 串行化并发 confirm_receipt）
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查状态
        if receipt.receipt_status != "DRAFT" {
            return Err(AppError::business(format!(
                "入库单状态不允许确认，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 3. 检查是否有明细
        let item_count = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .count(&txn)
            .await?;

        if item_count == 0 {
            return Err(AppError::business("入库单至少需要一行明细".to_string()));
        }

        // 4. 检查是否有关联的采购订单
        if let Some(order_id) = receipt.order_id {
            // 已实现: 更新采购订单的已入库数量
            self.update_order_received_quantity(order_id, receipt_id, &txn)
                .await?;
        }

        // 5. 更新状态
        let now = chrono::Utc::now();
        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();
        receipt_active.receipt_status = Set("CONFIRMED".to_string());
        receipt_active.confirmed_at = Set(Some(now));
        receipt_active.confirmed_by = Set(Some(user_id));
        receipt_active.updated_by = Set(Some(user_id));
        receipt_active.updated_at = Set(now);

        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            receipt_active,
            Some(0),
        )
        .await?;

        // 6. 更新库存（在事务内执行，保证原子性）
        self.update_inventory_txn(&receipt, &txn).await?;

        // 7. 提交事务
        txn.commit().await?;

        // 8. 自动生成应付账款（事务外执行，失败不影响入库）
        let ap_service =
            crate::services::ap_invoice_service::ApInvoiceService::new(self.db.clone());
        if let Err(e) = ap_service
            .auto_generate_from_receipt(receipt.id, user_id)
            .await
        {
            tracing::error!(
                "自动生成应付账单失败 (入库单 {}): {}",
                receipt.receipt_no,
                e
            );
        } else {
            tracing::info!("成功自动生成应付账单 (入库单 {})", receipt.receipt_no);
        }

        Ok(receipt)
    }

    /// 添加入库明细
    pub async fn add_receipt_item(
        &self,
        receipt_id: i32,
        req: CreateReceiptItemRequest,
        user_id: i32,
    ) -> Result<purchase_receipt_item::Model, AppError> {
        // 1. 查询入库单
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查状态
        if receipt.receipt_status != "DRAFT" {
            return Err(AppError::business(format!(
                "入库单状态不允许添加明细，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 3. 检查权限
        if receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能为自己创建的入库单添加明细".to_string(),
            ));
        }

        // 4. 创建明细
        let amount = req.quantity * req.unit_price.unwrap_or_else(|| Decimal::new(0, 0));
        let item = purchase_receipt_item::ActiveModel {
            receipt_id: Set(receipt_id),
            order_item_id: Set(req.order_item_id),
            product_id: Set(req.material_id),
            quantity: Set(req.quantity),
            quantity_alt: Set(Some(req.quantity_alt)),
            unit_price: Set(Some(req.unit_price.unwrap_or_else(|| Decimal::new(0, 0)))),
            amount: Set(Some(amount)),
            notes: Set(req.notes),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;

        // 5. 更新入库单总金额
        self.calculate_receipt_total(receipt_id).await?;

        Ok(item)
    }

    /// 更新入库明细
    pub async fn update_receipt_item(
        &self,
        item_id: i32,
        req: UpdateReceiptItemRequest,
        user_id: i32,
    ) -> Result<purchase_receipt_item::Model, AppError> {
        // 1. 查询明细
        let item = purchase_receipt_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("入库明细 {}", item_id)))?;

        // 2. 查询入库单
        let receipt = purchase_receipt::Entity::find_by_id(item.receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", item.receipt_id)))?;

        // 3. 检查状态
        if receipt.receipt_status != "DRAFT" {
            return Err(AppError::business(format!(
                "入库单状态不允许修改明细，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 4. 检查权限
        if receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能修改自己创建的入库明细".to_string(),
            ));
        }

        // 5. 更新明细
        let mut item_active: purchase_receipt_item::ActiveModel = item.into();

        if let Some(quantity) = req.quantity {
            item_active.quantity = Set(quantity);
        }
        if let Some(quantity_alt) = req.quantity_alt {
            item_active.quantity_alt = Set(Some(quantity_alt));
        }
        if let Some(unit_price) = req.unit_price {
            item_active.unit_price = Set(Some(unit_price));
        }
        if let Some(notes) = req.notes {
            item_active.notes = Set(Some(notes));
        }

        let item = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            item_active,
            Some(0),
        )
        .await?;

        // 6. 更新入库单总金额
        self.calculate_receipt_total(receipt.id).await?;

        Ok(item)
    }

    /// 删除入库明细
    pub async fn delete_receipt_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询明细
        let item = purchase_receipt_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("入库明细 {}", item_id)))?;

        // 2. 查询入库单
        let receipt = purchase_receipt::Entity::find_by_id(item.receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", item.receipt_id)))?;

        // 3. 检查状态
        if receipt.receipt_status != "DRAFT" {
            return Err(AppError::business(format!(
                "入库单状态不允许删除明细，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 4. 检查权限
        if receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能删除自己创建的入库明细".to_string(),
            ));
        }

        // 5. 删除明细
        purchase_receipt_item::Entity::delete_by_id(item_id)
            .exec(&*self.db)
            .await?;

        // 6. 更新入库单总金额
        self.calculate_receipt_total(receipt.id).await?;

        Ok(())
    }

    /// 计算入库单总金额
    pub async fn calculate_receipt_total(&self, receipt_id: i32) -> Result<(), AppError> {
        // 1. 查询所有明细
        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .all(&*self.db)
            .await?;

        // 2. 计算总和
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);
        let mut total_amount = Decimal::new(0, 0);

        for item in items {
            total_quantity += item.quantity;
            total_quantity_alt += item.quantity_alt.unwrap_or_default();
            total_amount += item.amount.unwrap_or_default();
        }

        // 3. 更新入库单
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();
        receipt_active.total_quantity = Set(total_quantity);
        receipt_active.total_quantity_alt = Set(total_quantity_alt);
        receipt_active.total_amount = Set(total_amount);
        receipt_active.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            receipt_active,
            Some(0),
        )
        .await?;

        Ok(())
    }

    /// 获取入库单列表（分页）
    pub async fn list_receipts(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
        order_id: Option<i32>,
    ) -> Result<(Vec<purchase_receipt::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let mut query = purchase_receipt::Entity::find();

        // 添加筛选条件
        if let Some(status) = status {
            query = query.filter(purchase_receipt::Column::ReceiptStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_receipt::Column::SupplierId.eq(supplier_id));
        }
        if let Some(order_id) = order_id {
            query = query.filter(purchase_receipt::Column::OrderId.eq(order_id));
        }

        // 分页查询
        let paginator = query
            .order_by(purchase_receipt::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
    }

    /// 获取入库单详情
    pub async fn get_receipt(&self, receipt_id: i32) -> Result<purchase_receipt::Model, AppError> {
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        Ok(receipt)
    }

    /// 获取入库明细列表
    pub async fn list_receipt_items(
        &self,
        receipt_id: i32,
    ) -> Result<Vec<purchase_receipt_item::Model>, AppError> {
        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .order_by(purchase_receipt_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(items)
    }
}
