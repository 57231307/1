use axum::{extract::State, Json};
use crate::{
    services::ar_receipt_service::{ArReceiptService, CreateReceiptRequest, ReceiptResponse},
};
use std::sync::Arc;

pub async fn create_receipt(
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    Json(req): Json<CreateReceiptRequest>,
) -> Result<Json<ReceiptResponse>, (axum::http::StatusCode, String)> {
    match ArReceiptService::create_receipt(&db, req).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn list_receipts(
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
) -> Result<Json<Vec<ReceiptResponse>>, (axum::http::StatusCode, String)> {
    match ArReceiptService::list_receipts(&db).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
