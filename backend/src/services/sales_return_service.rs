//! 销售退货 Service
//!
//! 销售退货服务层，负责销售退货的核心业务逻辑
#![allow(dead_code)]

use crate::models::{inventory_stock, product, sales_return, sales_return_item};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
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
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            return_active,
            Some(0),
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
    ) -> Result<sales_return_item::Model, AppError> {
        // 验证退货单存在且为草稿状态
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "DRAFT" {
            return Err(AppError::business(format!(
                "退货单状态不允许添加明细，当前状态：{}",
                return_order.status
            )));
        }

        let txn = (*self.db).begin().await?;

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
        self.update_return_totals(return_id, &txn).await?;

        txn.commit().await?;

        Ok(item)
    }

    /// 更新销售退货单
    pub async fn update_return(
        &self,
        return_id: i32,
        req: UpdateSalesReturnRequest,
        _user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
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
            &*self.db,
            "auto_audit",
            active_model,
            Some(0),
        )
        .await?;

        Ok(return_order)
    }

    /// 提交销售退货单
    pub async fn submit_return(
        &self,
        return_id: i32,
        _user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "DRAFT" {
            return Err(AppError::business(format!(
                "退货单状态不允许提交，当前状态：{}",
                return_order.status
            )));
        }

        // 验证是否包含明细
        let items_count = sales_return_item::Entity::find()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .count(&*self.db)
            .await?;

        if items_count == 0 {
            return Err(AppError::business("退货单没有明细，无法提交".to_string()));
        }

        // 开启事务，更新退货单总金额和状态
        let txn = (*self.db).begin().await?;

        // 更新退货单总金额
        self.update_return_totals(return_id, &txn).await?;

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
            Some(0),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 审批销售退货单
    pub async fn approve_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&txn)
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
            .all(&txn)
            .await?;

        // 更新退货单总金额
        self.update_return_totals(return_id, &txn).await?;

        let stock_service = InventoryStockService::new(self.db.clone());

        // 批量获取商品信息和库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids.clone()))
            .all(&txn)
            .await?;
        let product_map: std::collections::HashMap<i32, product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();

        let stocks = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(return_order.warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(&txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        for item in &items {
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
                    &txn,
                    "auto_audit",
                    stock_update,
                    Some(0),
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
                new_stock.insert(&txn).await?;
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

        let mut active_model: sales_return::ActiveModel = return_order.clone().into();
        active_model.status = Set("APPROVED".to_string());
        active_model.approved_by = Set(Some(user_id));
        active_model.approved_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(0),
        )
        .await?;

        txn.commit().await?;

        // 生成应收单 (红字，因为是退货)
        let ar_invoice_service = ArInvoiceService::new(self.db.clone());
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

        if let Err(e) = ar_invoice_service.create(ar_request, user_id).await {
            tracing::error!(
                "自动生成红字应收单失败 (退货单 {}): {}",
                return_order.return_no,
                e
            );
        } else {
            tracing::info!("成功自动生成红字应收单 (退货单 {})", return_order.return_no);
        }

        Ok(return_order)
    }

    /// 获取退货单详情
    pub async fn get_return(&self, return_id: i32) -> Result<sales_return::Model, AppError> {
        sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))
    }

    /// 删除退货单
    pub async fn delete_return(&self, return_id: i32) -> Result<(), AppError> {
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != "DRAFT" {
            return Err(AppError::business(format!(
                "退货单状态不允许删除，当前状态：{}",
                return_order.status
            )));
        }

        let txn = (*self.db).begin().await?;

        // 先删除明细
        sales_return_item::Entity::delete_many()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .exec(&txn)
            .await?;

        // 再删除退货单
        sales_return::Entity::delete_by_id(return_id)
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 拒绝退货单
    pub async fn reject_return(
        &self,
        return_id: i32,
        reason: String,
        _user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
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
            &*self.db,
            "auto_audit",
            active_model,
            Some(0),
        )
        .await?;

        Ok(return_order)
    }

    /// 执行退货单（完成退货流程）
    pub async fn execute_return(
        &self,
        return_id: i32,
        _user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
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
            &*self.db,
            "auto_audit",
            active_model,
            Some(0),
        )
        .await?;

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
        self.update_return_totals(item.return_id, &txn).await?;

        txn.commit().await?;
        Ok(item)
    }

    /// 删除退货单明细
    pub async fn delete_return_item(&self, item_id: i32) -> Result<(), AppError> {
        let item = sales_return_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货明细 {}", item_id)))?;

        let txn = (*self.db).begin().await?;

        sales_return_item::Entity::delete_by_id(item_id)
            .exec(&txn)
            .await?;

        // 更新退货单总金额
        self.update_return_totals(item.return_id, &txn).await?;

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
