use chrono::NaiveDate;
// 应收单 Service
//
// 应收账款业务逻辑层

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use std::sync::Arc;
use tracing::info;

use crate::models::ar_invoice;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use rust_decimal::Decimal;
use chrono::Utc;
use sea_orm::ActiveValue::Set;

use serde::Deserialize;

/// 更新应收发票请求
#[derive(Debug, Deserialize)]
pub struct UpdateArInvoiceRequest {
    pub invoice_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub invoice_amount: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct CreateArInvoiceRequest {
    pub invoice_date: chrono::NaiveDate,
    pub due_date: chrono::NaiveDate,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub source_type: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub invoice_amount: Decimal,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub sales_order_no: Option<String>,
}

/// 应收单 Service
pub struct ArInvoiceService {
    db: Arc<DatabaseConnection>,
}

impl ArInvoiceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建应收单
    pub async fn create(
        &self,
        req: CreateArInvoiceRequest,
        user_id: i32,
    ) -> Result<ar_invoice::Model, AppError> {
        info!(
            "创建应收单：customer_id={}, amount={}",
            req.customer_id, req.invoice_amount
        );

        // 生成应收单编号
        let invoice_no = self.generate_invoice_no().await?;

        let active_model = ar_invoice::ActiveModel {
            invoice_no: sea_orm::Set(invoice_no),
            invoice_date: sea_orm::Set(req.invoice_date),
            due_date: sea_orm::Set(req.due_date),
            customer_id: sea_orm::Set(req.customer_id),
            customer_name: sea_orm::Set(req.customer_name),
            source_type: sea_orm::Set(req.source_type),
            source_bill_id: sea_orm::Set(req.source_bill_id),
            source_bill_no: sea_orm::Set(req.source_bill_no),
            invoice_amount: sea_orm::Set(req.invoice_amount),
            received_amount: sea_orm::Set(Decimal::ZERO),
            unpaid_amount: sea_orm::Set(req.invoice_amount),
            batch_no: sea_orm::Set(req.batch_no),
            color_no: sea_orm::Set(req.color_no),
            sales_order_no: sea_orm::Set(req.sales_order_no),
            status: sea_orm::Set("draft".to_string()),
            created_by: sea_orm::Set(user_id),
            ..Default::default()
        };

        let result = active_model.insert(&*self.db).await?;
        info!("应收单创建成功：no={}", result.invoice_no);

        Ok(result)
    }

    /// 查询应收单列表
    pub async fn get_list(
        &self,
        customer_id: Option<i32>,
        status: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ar_invoice::Model>, u64), AppError> {
        info!("查询应收单列表");

        let mut query = ar_invoice::Entity::find();

        if let Some(cid) = customer_id {
            query = query.filter(ar_invoice::Column::CustomerId.eq(cid));
        }

        if let Some(s) = status {
            query = query.filter(ar_invoice::Column::Status.eq(s));
        }

        let total = query.clone().count(&*self.db).await?;
        let invoices = query
            .order_by(ar_invoice::Column::InvoiceDate, Order::Desc)
            .offset(page - 1)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        info!("应收单列表查询成功，共 {} 条", total);
        Ok((invoices, total))
    }

    /// 查询应收单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ar_invoice::Model, AppError> {
        info!("查询应收单详情 ID: {}", id);

        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("应收单不存在：{}", id)))?;

        Ok(invoice)
    }

    /// 生成应收单编号
    async fn generate_invoice_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &self.db,
            "AR",
            ar_invoice::Entity,
            ar_invoice::Column::InvoiceNo,
        )
        .await
    }

    pub async fn update(&self, id: i32, req: UpdateArInvoiceRequest, _user_id: i32) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("应收单不存在".to_string()))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::BadRequest("非草稿状态的应收单无法修改".to_string()));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.clone().into();
        
        if let Some(date) = req.invoice_date {
            active_invoice.invoice_date = sea_orm::ActiveValue::Set(date);
        }
        if let Some(date) = req.due_date {
            active_invoice.due_date = sea_orm::ActiveValue::Set(date);
        }
        if let Some(amt) = req.invoice_amount {
            active_invoice.invoice_amount = sea_orm::ActiveValue::Set(amt);
            active_invoice.unpaid_amount = sea_orm::ActiveValue::Set(amt - invoice.received_amount);
        }
        
        active_invoice.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = active_invoice.update(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result)
    }

    pub async fn delete(&self, id: i32, _user_id: i32) -> Result<(), AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("应收单不存在".to_string()))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::BadRequest("非草稿状态的应收单无法删除".to_string()));
        }

        ar_invoice::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn approve(&self, id: i32, user_id: i32) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("应收单不存在".to_string()))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::BadRequest("只能审批草稿状态的应收单".to_string()));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
        active_invoice.status = sea_orm::ActiveValue::Set("APPROVED".to_string());
        active_invoice.approval_status = sea_orm::ActiveValue::Set("APPROVED".to_string());
        active_invoice.reviewed_by = sea_orm::ActiveValue::Set(Some(user_id));
        active_invoice.reviewed_at = sea_orm::ActiveValue::Set(Some(Utc::now()));
        active_invoice.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = active_invoice.update(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result)
    }

    pub async fn cancel(&self, id: i32, _reason: String, _user_id: i32) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("应收单不存在".to_string()))?;

        if invoice.status == "CANCELLED" {
            return Err(AppError::BadRequest("单据已取消".to_string()));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
        active_invoice.status = Set("CANCELLED".to_string());
        active_invoice.updated_at = Set(Utc::now());

        let result = active_invoice.update(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result)
    }
}
