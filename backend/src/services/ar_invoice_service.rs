use chrono::NaiveDate;
// 应收单 Service
//
// 应收账款业务逻辑层

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use std::sync::Arc;
use tracing::info;

use crate::models::ar_invoice;
use crate::models::sales_delivery;
use crate::utils::error::AppError;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
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
    pub invoice_date: Option<chrono::NaiveDate>,
    pub due_date: Option<chrono::NaiveDate>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub source_type: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub invoice_amount: Option<Decimal>,
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

    /// 从销售出库单自动生成应收单
    #[allow(dead_code)] // TODO(tech-debt): 销售出库流程接入自动开票后移除
    pub async fn auto_generate_from_delivery(
        &self,
        delivery_id: i32,
        user_id: i32,
    ) -> Result<ar_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询销售出库单
        let delivery = sales_delivery::Entity::find_by_id(delivery_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售出库单 {} 不存在", delivery_id)))?;

        // 2. 检查是否已生成应收单
        let exists = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::SourceType.eq("SALES_DELIVERY"))
            .filter(ar_invoice::Column::SourceBillId.eq(delivery_id))
            .one(&txn)
            .await?;

        if exists.is_some() {
            return Err(AppError::bad_request("该出库单已生成应收单"));
        }

        // 3. 获取客户信息
        let customer = crate::models::customer::Entity::find_by_id(delivery.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", delivery.customer_id)))?;

        // 使用默认账期 30 天
        let payment_terms = 30;

        // 4. 生成应收单
        let invoice_no = self.generate_invoice_no().await?;
        let invoice_date = delivery.delivery_date;
        let due_date = invoice_date + Duration::days(payment_terms as i64);

        let invoice = ar_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            invoice_date: Set(invoice_date),
            due_date: Set(due_date),
            customer_id: Set(delivery.customer_id),
            customer_name: Set(Some(customer.customer_name.clone())),
            source_type: Set(Some("SALES_DELIVERY".to_string())),
            source_bill_id: Set(Some(delivery_id)),
            source_bill_no: Set(Some(delivery.delivery_no.clone())),
            invoice_amount: Set(delivery.total_amount),
            received_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(delivery.total_amount),
            batch_no: Set(delivery.remarks.clone()),
            sales_order_no: Set(None),
            status: Set("APPROVED".to_string()),
            approval_status: Set("APPROVED".to_string()),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        info!(
            "从销售出库单自动生成应收单成功：delivery_no={}, invoice_no={}",
            delivery.delivery_no, invoice.invoice_no
        );

        Ok(invoice)
    }

    /// 创建应收单
    pub async fn create(
        &self,
        req: CreateArInvoiceRequest,
        user_id: i32,
    ) -> Result<ar_invoice::Model, AppError> {
        // 验证客户ID
        let customer_id = req
            .customer_id
            .ok_or_else(|| AppError::validation("客户ID不能为空"))?;
        if customer_id <= 0 {
            return Err(AppError::validation("客户ID无效"));
        }

        // 验证发票金额
        let invoice_amount = req
            .invoice_amount
            .ok_or_else(|| AppError::validation("发票金额不能为空"))?;
        if invoice_amount <= Decimal::ZERO {
            return Err(AppError::validation("发票金额必须大于零"));
        }

        info!(
            "创建应收单：customer_id={}, amount={}",
            customer_id, invoice_amount
        );

        // 生成应收单编号
        let invoice_no = self.generate_invoice_no().await?;

        let active_model = ar_invoice::ActiveModel {
            invoice_no: sea_orm::Set(invoice_no),
            invoice_date: sea_orm::Set(
                req.invoice_date
                    .unwrap_or_else(|| chrono::Utc::now().date_naive()),
            ),
            due_date: sea_orm::Set(
                req.due_date
                    .unwrap_or_else(|| chrono::Utc::now().date_naive()),
            ),
            customer_id: sea_orm::Set(customer_id),
            customer_name: sea_orm::Set(req.customer_name),
            source_type: sea_orm::Set(req.source_type),
            source_bill_id: sea_orm::Set(req.source_bill_id),
            source_bill_no: sea_orm::Set(req.source_bill_no),
            invoice_amount: sea_orm::Set(invoice_amount),
            received_amount: sea_orm::Set(Decimal::ZERO),
            unpaid_amount: sea_orm::Set(invoice_amount),
            batch_no: sea_orm::Set(req.batch_no),
            color_no: sea_orm::Set(req.color_no),
            sales_order_no: sea_orm::Set(req.sales_order_no),
            status: sea_orm::Set("DRAFT".to_string()),
            approval_status: sea_orm::Set("PENDING".to_string()),
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
            .offset((page - 1) * page_size)
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
            .ok_or_else(|| AppError::not_found(format!("应收单不存在：{}", id)))?;

        Ok(invoice)
    }

    // 生成应收单编号
    crate::impl_generate_no!(
        generate_invoice_no,
        "AR",
        ar_invoice::Entity,
        ar_invoice::Column::InvoiceNo
    );

    pub async fn update(
        &self,
        id: i32,
        req: UpdateArInvoiceRequest,
        _user_id: i32,
    ) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("应收单不存在"))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::bad_request(
                "非草稿状态的应收单无法修改".to_string(),
            ));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.clone().into();

        if let Some(date) = req.invoice_date {
            active_invoice.invoice_date = sea_orm::ActiveValue::Set(date);
        }
        if let Some(date) = req.due_date {
            active_invoice.due_date = sea_orm::ActiveValue::Set(date);
        }
        if let Some(amt) = req.invoice_amount {
            let new_unpaid = (amt - invoice.received_amount).max(Decimal::ZERO);
            active_invoice.invoice_amount = sea_orm::ActiveValue::Set(amt);
            active_invoice.unpaid_amount = sea_orm::ActiveValue::Set(new_unpaid);
        }

        active_invoice.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            active_invoice,
            Some(0),
        )
        .await?;

        Ok(result)
    }

    pub async fn delete(&self, id: i32, _user_id: i32) -> Result<(), AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("应收单不存在"))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::bad_request(
                "非草稿状态的应收单无法删除".to_string(),
            ));
        }

        ar_invoice::Entity::delete_by_id(id).exec(&*self.db).await?;

        Ok(())
    }

    pub async fn approve(&self, id: i32, user_id: i32) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("应收单不存在"))?;

        if invoice.status != "DRAFT" {
            return Err(AppError::bad_request("只能审批草稿状态的应收单"));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
        active_invoice.status = sea_orm::ActiveValue::Set("APPROVED".to_string());
        active_invoice.approval_status = sea_orm::ActiveValue::Set("APPROVED".to_string());
        active_invoice.reviewed_by = sea_orm::ActiveValue::Set(Some(user_id));
        active_invoice.reviewed_at = sea_orm::ActiveValue::Set(Some(Utc::now()));
        active_invoice.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = active_invoice.update(&*self.db).await?;

        Ok(result)
    }

    /// 标记应收单为已收讫
    pub async fn mark_as_paid(&self, id: i32) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("应收单不存在"))?;

        if invoice.status == "PAID" || invoice.status == "CANCELLED" {
            return Err(AppError::bad_request(format!(
                "应收单状态为{}，不可标记为已收讫",
                invoice.status
            )));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.clone().into();
        active_invoice.status = sea_orm::ActiveValue::Set("PAID".to_string());
        active_invoice.received_amount = sea_orm::ActiveValue::Set(invoice.invoice_amount);
        active_invoice.unpaid_amount = sea_orm::ActiveValue::Set(Decimal::ZERO);
        active_invoice.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            active_invoice,
            Some(0),
        )
        .await?;

        Ok(result)
    }

    pub async fn cancel(
        &self,
        id: i32,
        _reason: String,
        _user_id: i32,
    ) -> Result<ar_invoice::Model, AppError> {
        let invoice = ar_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("应收单不存在"))?;

        if invoice.status == "CANCELLED" {
            return Err(AppError::bad_request("单据已取消"));
        }

        let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
        active_invoice.status = Set("CANCELLED".to_string());
        active_invoice.updated_at = Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            active_invoice,
            Some(0),
        )
        .await?;

        Ok(result)
    }
}
