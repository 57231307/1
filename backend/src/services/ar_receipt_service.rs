use crate::models::ar_receipt;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Deserialize)]
pub struct CreateReceiptRequest {
    pub customer_id: i32,
    pub amount: f64,
    pub payment_method: String,
}

#[derive(Serialize)]
pub struct ReceiptResponse {
    pub id: i32,
    pub receipt_no: String,
    pub customer_id: i32,
    pub amount: f64,
    pub payment_method: String,
    pub status: String,
}

pub struct ArReceiptService;

impl ArReceiptService {
    pub async fn create_receipt(
        _db: &DatabaseConnection,
        req: CreateReceiptRequest,
    ) -> Result<ReceiptResponse, DbErr> {
        // Mock implementation to satisfy the frontend for now, in reality this would hit the DB
        let now = Utc::now().naive_utc();
        let receipt = ar_receipt::Model {
            id: (now.timestamp() % 10000) as i32,
            receipt_no: format!("RCPT-{}", now.timestamp()),
            customer_id: req.customer_id,
            amount: req.amount,
            payment_method: req.payment_method.clone(),
            status: "COMPLETED".to_string(),
            receipt_date: now,
            created_at: now,
            updated_at: now,
        };

        Ok(ReceiptResponse {
            id: receipt.id,
            receipt_no: receipt.receipt_no,
            customer_id: receipt.customer_id,
            amount: receipt.amount,
            payment_method: receipt.payment_method,
            status: receipt.status,
        })
    }

    pub async fn list_receipts(_db: &DatabaseConnection) -> Result<Vec<ReceiptResponse>, DbErr> {
        Ok(vec![])
    }
}
