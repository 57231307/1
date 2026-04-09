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
        db: &DatabaseConnection,
        req: CreateReceiptRequest,
    ) -> Result<ReceiptResponse, DbErr> {
        let now = Utc::now().naive_utc();
        let receipt_no = format!("RCPT-{}", now.timestamp());
        
        let new_receipt = ar_receipt::ActiveModel {
            id: NotSet,
            receipt_no: Set(receipt_no.clone()),
            customer_id: Set(req.customer_id),
            amount: Set(req.amount),
            payment_method: Set(req.payment_method.clone()),
            status: Set("COMPLETED".to_string()),
            receipt_date: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
        };
        
        let inserted = new_receipt.insert(db).await?;

        Ok(ReceiptResponse {
            id: inserted.id,
            receipt_no: inserted.receipt_no,
            customer_id: inserted.customer_id,
            amount: inserted.amount,
            payment_method: inserted.payment_method,
            status: inserted.status,
        })
    }

    pub async fn list_receipts(db: &DatabaseConnection) -> Result<Vec<ReceiptResponse>, DbErr> {
        let receipts = ar_receipt::Entity::find()
            .order_by_desc(ar_receipt::Column::CreatedAt)
            .all(db)
            .await?;
            
        Ok(receipts.into_iter().map(|r| ReceiptResponse {
            id: r.id,
            receipt_no: r.receipt_no,
            customer_id: r.customer_id,
            amount: r.amount,
            payment_method: r.payment_method,
            status: r.status,
        }).collect())
    }
}
