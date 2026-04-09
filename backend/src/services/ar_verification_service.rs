use crate::models::ar_verification;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Deserialize)]
pub struct CreateVerificationRequest {
    pub customer_id: i32,
    pub receipt_id: i32,
    pub invoice_id: i32,
    pub verify_amount: f64,
}

#[derive(Serialize)]
pub struct VerificationResponse {
    pub id: i32,
    pub verify_no: String,
    pub customer_id: i32,
    pub receipt_id: i32,
    pub invoice_id: i32,
    pub verify_amount: f64,
    pub status: String,
}

pub struct ArVerificationService;

impl ArVerificationService {
    pub async fn create_verification(
        _db: &DatabaseConnection,
        req: CreateVerificationRequest,
    ) -> Result<VerificationResponse, DbErr> {
        let now = Utc::now().naive_utc();
        let verify = ar_verification::Model {
            id: (now.timestamp() % 10000) as i32,
            verify_no: format!("VER-{}", now.timestamp()),
            customer_id: req.customer_id,
            receipt_id: req.receipt_id,
            invoice_id: req.invoice_id,
            verify_amount: req.verify_amount,
            status: "VERIFIED".to_string(),
            verify_date: now,
            created_at: now,
            updated_at: now,
        };

        Ok(VerificationResponse {
            id: verify.id,
            verify_no: verify.verify_no,
            customer_id: verify.customer_id,
            receipt_id: verify.receipt_id,
            invoice_id: verify.invoice_id,
            verify_amount: verify.verify_amount,
            status: verify.status,
        })
    }

    pub async fn list_verifications(_db: &DatabaseConnection) -> Result<Vec<VerificationResponse>, DbErr> {
        Ok(vec![])
    }
}
