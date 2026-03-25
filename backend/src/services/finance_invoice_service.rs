use crate::models::finance_invoice::Model as InvoiceModel;
use crate::models::finance_invoice::{self, ActiveModel, Entity as FinanceInvoice};
use chrono::{DateTime, Utc};
use sea_orm::*;
use serde::Deserialize;
use std::sync::Arc;

/// 创建发票请求
#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub invoice_no: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: String,
    pub invoice_type: String,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub status: Option<String>,
    pub invoice_date: Option<DateTime<Utc>>,
    pub due_date: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

/// 更新发票请求
#[derive(Debug, Deserialize)]
pub struct UpdateInvoiceRequest {
    pub invoice_no: Option<String>,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub invoice_type: Option<String>,
    pub amount: Option<rust_decimal::Decimal>,
    pub tax_amount: Option<rust_decimal::Decimal>,
    pub total_amount: Option<rust_decimal::Decimal>,
    pub status: Option<String>,
    pub invoice_date: Option<DateTime<Utc>>,
    pub due_date: Option<DateTime<Utc>>,
    pub paid_date: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

/// 发票服务
pub struct FinanceInvoiceService {
    db: Arc<DatabaseConnection>,
}

impl FinanceInvoiceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取发票列表
    pub async fn list_invoices(&self) -> Result<Vec<InvoiceModel>, DbErr> {
        FinanceInvoice::find()
            .order_by(finance_invoice::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
    }

    /// 获取发票详情
    pub async fn get_invoice(&self, id: i32) -> Result<InvoiceModel, DbErr> {
        FinanceInvoice::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or(DbErr::RecordNotFound(format!("发票 {} 不存在", id)))
    }

    /// 创建发票
    pub async fn create_invoice(&self, req: CreateInvoiceRequest) -> Result<InvoiceModel, DbErr> {
        let active_model = ActiveModel {
            id: NotSet,
            invoice_no: Set(req.invoice_no),
            order_id: Set(req.order_id),
            customer_id: Set(req.customer_id),
            customer_name: Set(req.customer_name),
            invoice_type: Set(req.invoice_type),
            amount: Set(req.amount),
            tax_amount: Set(req.tax_amount),
            total_amount: Set(req.total_amount),
            status: Set(req.status.unwrap_or_else(|| "pending".to_string())),
            invoice_date: Set(req.invoice_date),
            due_date: Set(req.due_date),
            paid_date: Set(None),
            payment_method: Set(req.payment_method),
            notes: Set(req.notes),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        active_model.insert(self.db.as_ref()).await
    }

    /// 更新发票
    pub async fn update_invoice(
        &self,
        id: i32,
        req: UpdateInvoiceRequest,
    ) -> Result<InvoiceModel, DbErr> {
        let mut invoice: ActiveModel = FinanceInvoice::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or(DbErr::RecordNotFound(format!("发票 {} 不存在", id)))?
            .into();

        if let Some(invoice_no) = req.invoice_no {
            invoice.invoice_no = Set(invoice_no);
        }
        if let Some(order_id) = req.order_id {
            invoice.order_id = Set(Some(order_id));
        }
        if let Some(customer_id) = req.customer_id {
            invoice.customer_id = Set(Some(customer_id));
        }
        if let Some(customer_name) = req.customer_name {
            invoice.customer_name = Set(customer_name);
        }
        if let Some(invoice_type) = req.invoice_type {
            invoice.invoice_type = Set(invoice_type);
        }
        if let Some(amount) = req.amount {
            invoice.amount = Set(amount);
        }
        if let Some(tax_amount) = req.tax_amount {
            invoice.tax_amount = Set(tax_amount);
        }
        if let Some(total_amount) = req.total_amount {
            invoice.total_amount = Set(total_amount);
        }
        if let Some(status) = req.status {
            invoice.status = Set(status);
        }
        if let Some(invoice_date) = req.invoice_date {
            invoice.invoice_date = Set(Some(invoice_date));
        }
        if let Some(due_date) = req.due_date {
            invoice.due_date = Set(Some(due_date));
        }
        if let Some(paid_date) = req.paid_date {
            invoice.paid_date = Set(Some(paid_date));
        }
        if let Some(payment_method) = req.payment_method {
            invoice.payment_method = Set(Some(payment_method));
        }
        if let Some(notes) = req.notes {
            invoice.notes = Set(Some(notes));
        }

        invoice.updated_at = Set(Utc::now());

        invoice.update(self.db.as_ref()).await
    }

    /// 删除发票
    pub async fn delete_invoice(&self, id: i32) -> Result<DeleteResult, DbErr> {
        let invoice: ActiveModel = FinanceInvoice::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or(DbErr::RecordNotFound(format!("发票 {} 不存在", id)))?
            .into();

        invoice.delete(self.db.as_ref()).await
    }

    /// 审核发票
    pub async fn approve_invoice(&self, id: i32) -> Result<InvoiceModel, DbErr> {
        let mut invoice: ActiveModel = FinanceInvoice::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or(DbErr::RecordNotFound(format!("发票 {} 不存在", id)))?
            .into();

        invoice.status = Set("approved".to_string());
        invoice.updated_at = Set(Utc::now());

        invoice.update(self.db.as_ref()).await
    }

    /// 核销发票（带事务）
    pub async fn verify_invoice(
        &self,
        id: i32,
        paid_date: DateTime<Utc>,
        payment_method: String,
    ) -> Result<InvoiceModel, DbErr> {
        // 开启事务
        let txn = (&*self.db).begin().await?;

        let mut invoice: ActiveModel = FinanceInvoice::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(DbErr::RecordNotFound(format!("发票 {} 不存在", id)))?
            .into();

        invoice.status = Set("verified".to_string());
        invoice.paid_date = Set(Some(paid_date));
        invoice.payment_method = Set(Some(payment_method));
        invoice.updated_at = Set(Utc::now());

        let result = invoice.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        Ok(result)
    }
}
