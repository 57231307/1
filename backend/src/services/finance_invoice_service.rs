use crate::models::finance_invoice::Model as InvoiceModel;
use crate::models::finance_invoice::{self, ActiveModel, Entity as FinanceInvoice};
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

pub struct FinanceInvoiceService {
    db: Arc<DatabaseConnection>,
}

impl FinanceInvoiceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list_invoices(&self) -> Result<Vec<InvoiceModel>, AppError> {
        FinanceInvoice::find()
            .order_by(finance_invoice::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    pub async fn get_invoice(&self, id: i32) -> Result<Option<InvoiceModel>, AppError> {
        FinanceInvoice::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    pub async fn create_invoice(
        &self,
        invoice_no: String,
        amount: Decimal,
        tax_amount: Decimal,
        total_amount: Decimal,
    ) -> Result<InvoiceModel, AppError> {
        let active_model = ActiveModel {
            id: NotSet,
            invoice_no: Set(invoice_no),
            order_id: Set(None),
            invoice_date: Set(Utc::now()),
            amount: Set(amount),
            tax_amount: Set(tax_amount),
            total_amount: Set(total_amount),
            status: Set("pending".to_string()),
            paid_date: Set(None),
            payment_method: Set(None),
            notes: Set(None),
            created_by: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        active_model
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    pub async fn update_invoice(
        &self,
        id: i32,
        payload: serde_json::Value,
    ) -> Result<Option<InvoiceModel>, AppError> {
        let invoice = FinanceInvoice::find_by_id(id).one(self.db.as_ref()).await?;

        if let Some(invoice) = invoice {
            let mut active_model: ActiveModel = invoice.into();

            if let Some(status) = payload.get("status").and_then(|v| v.as_str()) {
                active_model.status = Set(status.to_string());
            }
            if let Some(notes) = payload.get("notes").and_then(|v| v.as_str()) {
                active_model.notes = Set(Some(notes.to_string()));
            }

            active_model.updated_at = Set(Utc::now());

            let updated = active_model.update(self.db.as_ref()).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_invoice(&self, id: i32) -> Result<(), AppError> {
        FinanceInvoice::delete_by_id(id)
            .exec(self.db.as_ref())
            .await?;
        Ok(())
    }

    pub async fn approve_invoice(&self, id: i32) -> Result<Option<InvoiceModel>, AppError> {
        let invoice = FinanceInvoice::find_by_id(id).one(self.db.as_ref()).await?;

        if let Some(invoice) = invoice {
            let mut active_model: ActiveModel = invoice.into();
            active_model.status = Set("approved".to_string());
            active_model.updated_at = Set(Utc::now());

            let updated = active_model.update(self.db.as_ref()).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub async fn verify_invoice(&self, id: i32) -> Result<Option<InvoiceModel>, AppError> {
        let invoice = FinanceInvoice::find_by_id(id).one(self.db.as_ref()).await?;

        if let Some(invoice) = invoice {
            let mut active_model: ActiveModel = invoice.into();
            active_model.status = Set("verified".to_string());
            active_model.paid_date = Set(Some(Utc::now()));
            active_model.updated_at = Set(Utc::now());

            let updated = active_model.update(self.db.as_ref()).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }
}
