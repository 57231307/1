//! 采购退货 Service
//!
//! 采购退货服务层，负责采购退货的核心业务逻辑

use crate::models::{product, purchase_return, purchase_return_item};
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

/// 采购退货服务
pub struct PurchaseReturnService {
    db: Arc<DatabaseConnection>,
}

use crate::utils::number_generator::DocumentNumberGenerator;

impl PurchaseReturnService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成退货单号
    /// 格式：RT + 年月日 + 三位序号（RT20260315001）
    pub async fn generate_return_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "RT",
            purchase_return::Entity,
            purchase_return::Column::ReturnNo,
        )
        .await
    }

    /// 创建采购退货单
    pub async fn create_return(
        &self,
        req: CreatePurchaseReturnRequest,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let return_no = self.generate_return_no().await?;

        let return_order = purchase_return::ActiveModel {
            id: Set(0),
            return_no: Set(return_no),
            receipt_id: Set(req.receipt_id),
            order_id: Set(req.order_id),
            supplier_id: Set(req.supplier_id),
            return_date: Set(req.return_date),
            warehouse_id: Set(req.warehouse_id),
            department_id: Set(req.department_id),
            reason_type: Set(Some(req.reason_type)),
            reason_detail: Set(req.reason_detail),
            return_status: Set(Some("draft".to_string())),
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
        _user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购退货单 {}", return_id)))?;

        if return_order.return_status.as_deref() != Some("draft") {
            return Err(AppError::BusinessError(format!(
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
            Some(0),
        )
        .await?;

        Ok(return_order)
    }

    /// 提交采购退货单
    pub async fn submit_return(
        &self,
        return_id: i32,
        _user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购退货单 {}", return_id)))?;

        if return_order.return_status.as_deref() != Some("draft") {
            return Err(AppError::BusinessError(format!(
                "退货单状态不允许提交，当前状态：{:?}",
                return_order.return_status
            )));
        }

        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.return_status = Set(Some("submitted".to_string()));
        return_active.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            return_active,
            Some(0),
        )
        .await?;

        Ok(return_order)
    }

    /// 审批采购退货单
    pub async fn approve_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购退货单 {}", return_id)))?;

        if return_order.return_status.as_deref() != Some("submitted") {
            return Err(AppError::BusinessError(format!(
                "退货单状态不允许审批，当前状态：{:?}",
                return_order.return_status
            )));
        }

        // 检查是否有退货明细
        let item_count = purchase_return_item::Entity::find()
            .filter(purchase_return_item::Column::ReturnId.eq(return_id))
            .count(&*self.db)
            .await?;

        if item_count == 0 {
            return Err(AppError::BusinessError(
                "退货单至少需要一行明细".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.return_status = Set(Some("approved".to_string()));
        return_active.approved_by = Set(Some(user_id));
        return_active.approved_at = Set(Some(Utc::now()));
        return_active.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            return_active,
            Some(0),
        )
        .await?;

        // 1. 扣减库存（在事务内执行，保证原子性）
        let items = purchase_return_item::Entity::find()
            .filter(purchase_return_item::Column::ReturnId.eq(return_id))
            .all(&txn)
            .await?;

        for item in items {
            let warehouse_id = return_order.warehouse_id.unwrap_or(0);
            let existing_stock = crate::services::inventory_stock_service::InventoryStockService::find_by_product_and_warehouse_txn(
                &txn, item.product_id, warehouse_id,
            )
            .await
            ?;

            if let Some(s) = existing_stock {
                if s.quantity_meters < item.quantity {
                    return Err(AppError::BusinessError(format!(
                        "产品 {} 库存不足，当前库存：{}，需要退货：{}",
                        item.product_id, s.quantity_meters, item.quantity
                    )));
                }

                let new_quantity_meters = s.quantity_meters - item.quantity;
                let new_quantity_kg = s.quantity_kg - item.quantity_alt;

                crate::services::inventory_stock_service::InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
                    &txn, s.id, new_quantity_meters, new_quantity_kg, s.version,
                ).await?;

                crate::services::inventory_stock_service::InventoryStockService::record_transaction_txn(
                    &txn,
                    "PURCHASE_RETURN".to_string(),
                    item.product_id,
                    warehouse_id,
                    s.batch_no.clone(),
                    s.color_no.clone(),
                    s.dye_lot_no.clone(),
                    s.grade.clone(),
                    -item.quantity,
                    -item.quantity_alt,
                    Some("purchase_return".to_string()),
                    Some(return_order.return_no.clone()),
                    Some(return_order.id),
                    Some(s.quantity_meters),
                    Some(s.quantity_kg),
                    Some(new_quantity_meters),
                    Some(new_quantity_kg),
                    Some("采购退货扣减库存".to_string()),
                    Some(user_id),
                ).await?;
            } else {
                return Err(AppError::BusinessError(format!(
                    "产品 {} 在仓库 {} 没有库存记录，无法退货",
                    item.product_id, warehouse_id
                )));
            }
        }

        // 2. 提交事务（库存扣减和状态更新在同一事务内）
        txn.commit().await?;

        // 3. 自动生成应付红字账单（冲销）- 在事务外执行，失败不影响库存扣减
        let ap_service =
            crate::services::ap_invoice_service::ApInvoiceService::new(self.db.clone());
        if let Err(e) = ap_service
            .auto_generate_from_return(return_id, user_id)
            .await
        {
            tracing::error!(
                "自动生成应付账单失败 (退货单 {}): {}",
                return_order.return_no,
                e
            );
            // 记录失败但不阻断流程，可以后续手动重试
        } else {
            tracing::info!("成功自动生成应付账单 (退货单 {})", return_order.return_no);
        }

        Ok(return_order)
    }

    /// 拒绝采购退货单
    pub async fn reject_return(
        &self,
        return_id: i32,
        reason: String,
        _user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购退货单 {}", return_id)))?;

        if return_order.return_status.as_deref() != Some("submitted") {
            return Err(AppError::BusinessError(format!(
                "退货单状态不允许拒绝，当前状态：{:?}",
                return_order.return_status
            )));
        }

        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.return_status = Set(Some("rejected".to_string()));
        return_active.reason_detail = Set(Some(reason));
        return_active.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            return_active,
            Some(0),
        )
        .await?;

        Ok(return_order)
    }

    /// 获取退货单列表
    pub async fn list_returns(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<(Vec<purchase_return::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let mut query = purchase_return::Entity::find();

        if let Some(status) = status {
            query = query.filter(purchase_return::Column::ReturnStatus.eq(&status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_return::Column::SupplierId.eq(supplier_id));
        }

        let paginator = query
            .order_by(purchase_return::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
    }

    /// 获取退货单详情
    pub async fn get_return(&self, return_id: i32) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::NotFound(format!("采购退货单 {}", return_id)))?;

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
        use sea_orm::{JoinType, QuerySelect, RelationTrait};
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
    ) -> Result<purchase_return_item::Model, AppError> {
        let txn = self.db.begin().await?;

        // 验证主表状态（只有草稿可以修改明细，实际业务可能放宽，这里简化）
        let return_record = purchase_return::Entity::find_by_id(return_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("退货单 {}", return_id)))?;

        if return_record.return_status.as_deref() != Some("draft") {
            return Err(AppError::BusinessError(
                "只有草稿状态的退货单可以修改明细".to_string(),
            ));
        }

        let quantity = req.quantity_returned;
        let unit_price = req.unit_price;
        let discount_percent = req.discount_percent.unwrap_or(Decimal::ZERO);
        let tax_percent = req.tax_rate.unwrap_or(Decimal::ZERO);

        let subtotal = quantity * unit_price;
        let discount_amount = subtotal * (discount_percent / Decimal::new(100, 0));
        let taxable_amount = subtotal - discount_amount;
        let tax_amount = taxable_amount * (tax_percent / Decimal::new(100, 0));
        let total_amount = taxable_amount + tax_amount;

        let item = purchase_return_item::ActiveModel {
            id: Set(0),
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
        }
        .insert(&txn)
        .await?;

        self.update_return_totals(return_id, &txn).await?;
        txn.commit().await?;

        Ok(item)
    }

    /// 更新退货单明细
    pub async fn update_item(
        &self,
        item_id: i32,
        req: UpdateReturnItemRequest,
    ) -> Result<purchase_return_item::Model, AppError> {
        let txn = self.db.begin().await?;

        let item = purchase_return_item::Entity::find_by_id(item_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("退货明细 {}", item_id)))?;

        let return_record = purchase_return::Entity::find_by_id(item.return_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("退货单 {}", item.return_id)))?;

        if return_record.return_status.as_deref() != Some("draft") {
            return Err(AppError::BusinessError(
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

        let subtotal = quantity * unit_price;
        let discount_amount = subtotal * (discount_percent / Decimal::new(100, 0));
        let taxable_amount = subtotal - discount_amount;
        let tax_amount = taxable_amount * (tax_percent / Decimal::new(100, 0));
        let total_amount = taxable_amount + tax_amount;

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
            Some(0),
        )
        .await?;

        self.update_return_totals(updated_item.return_id, &txn)
            .await?;
        txn.commit().await?;

        Ok(updated_item)
    }

    /// 删除退货单明细
    pub async fn delete_item(&self, item_id: i32) -> Result<(), AppError> {
        let txn = self.db.begin().await?;

        let item = purchase_return_item::Entity::find_by_id(item_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("退货明细 {}", item_id)))?;

        let return_record = purchase_return::Entity::find_by_id(item.return_id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound(format!("退货单 {}", item.return_id)))?;

        if return_record.return_status.as_deref() != Some("draft") {
            return Err(AppError::BusinessError(
                "只有草稿状态的退货单可以修改明细".to_string(),
            ));
        }

        purchase_return_item::Entity::delete_by_id(item_id)
            .exec(&txn)
            .await?;

        self.update_return_totals(item.return_id, &txn).await?;
        txn.commit().await?;

        Ok(())
    }

    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let txn = self.db.begin().await?;

        let ret = purchase_return::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound("Return not found".to_string()))?;
        if ret.return_status.as_deref() != Some("draft") {
            return Err(AppError::BusinessError(
                "Only DRAFT returns can be deleted".to_string(),
            ));
        }

        purchase_return_item::Entity::delete_many()
            .filter(purchase_return_item::Column::ReturnId.eq(id))
            .exec(&txn)
            .await?;

        purchase_return::Entity::delete_by_id(id).exec(&txn).await?;

        txn.commit().await?;
        Ok(())
    }

    /// 更新主单合计金额和数量
    async fn update_return_totals(
        &self,
        return_id: i32,
        txn: &sea_orm::DatabaseTransaction,
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
            .ok_or(AppError::NotFound(format!("退货单 {}", return_id)))?;

        let mut active_return: purchase_return::ActiveModel = return_record.into();
        active_return.total_quantity = Set(Some(total_quantity));
        active_return.total_quantity_alt = Set(Some(total_quantity_alt));
        active_return.total_amount = Set(Some(total_amount));
        active_return.updated_at = Set(Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_return,
            Some(0),
        )
        .await?;

        Ok(())
    }
}
