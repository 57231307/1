use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::services::finance_payment_service::FinancePaymentService;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Deserialize)]
pub struct CreatePaymentRequest {
    pub payment_no: String,
    pub payment_type: String,
    pub order_type: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub amount: Decimal,
    pub payment_date: DateTime<Utc>,
    pub payment_method: Option<String>,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    pub id: i32,
    pub payment_no: String,
    pub payment_type: String,
    pub amount: Decimal,
    pub status: String,
    pub payment_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PaymentListResponse {
    pub payments: Vec<PaymentResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

pub async fn get_payment(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<PaymentResponse>, (StatusCode, String)> {
    let service = FinancePaymentService::new(db.clone());

    match service.find_by_id(id).await {
        Ok(payment) => Ok(Json(PaymentResponse {
            id: payment.id,
            payment_no: payment.payment_no,
            payment_type: payment.payment_type,
            amount: payment.amount,
            status: payment.status,
            payment_date: payment.payment_date,
            created_at: payment.created_at,
        })),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

pub async fn create_payment(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<CreatePaymentRequest>,
) -> Result<Json<PaymentResponse>, (StatusCode, String)> {
    let service = FinancePaymentService::new(db.clone());

    match service.create_payment(
        payload.payment_no,
        payload.payment_type,
        payload.order_type,
        payload.order_id,
        payload.customer_id,
        payload.supplier_id,
        payload.amount,
        payload.payment_date,
        payload.payment_method,
        payload.reference_no,
        payload.notes,
        Some(1), // TODO: 从认证信息中获取
    ).await {
        Ok(payment) => Ok(Json(PaymentResponse {
            id: payment.id,
            payment_no: payment.payment_no,
            payment_type: payment.payment_type,
            amount: payment.amount,
            status: payment.status,
            payment_date: payment.payment_date,
            created_at: payment.created_at,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn list_payments(
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<ListPaymentsParams>,
) -> Result<Json<PaymentListResponse>, (StatusCode, String)> {
    let service = FinancePaymentService::new(db.clone());

    match service.list_payments(
        params.page.unwrap_or(0),
        params.page_size.unwrap_or(20),
        params.status,
    ).await {
        Ok((payments, total)) => {
            let payment_responses: Vec<PaymentResponse> = payments
                .into_iter()
                .map(|payment| PaymentResponse {
                    id: payment.id,
                    payment_no: payment.payment_no,
                    payment_type: payment.payment_type,
                    amount: payment.amount,
                    status: payment.status,
                    payment_date: payment.payment_date,
                    created_at: payment.created_at,
                })
                .collect();

            Ok(Json(PaymentListResponse {
                payments: payment_responses,
                total,
                page: params.page.unwrap_or(0),
                page_size: params.page_size.unwrap_or(20),
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListPaymentsParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
}

use axum::extract::Query;
