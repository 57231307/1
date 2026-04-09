use axum::{extract::State, Json};
use crate::{
    services::ar_verification_service::{ArVerificationService, CreateVerificationRequest, VerificationResponse},
};
use std::sync::Arc;

pub async fn create_verification(
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    Json(req): Json<CreateVerificationRequest>,
) -> Result<Json<VerificationResponse>, (axum::http::StatusCode, String)> {
    match ArVerificationService::create_verification(&db, req).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn list_verifications(
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
) -> Result<Json<Vec<VerificationResponse>>, (axum::http::StatusCode, String)> {
    match ArVerificationService::list_verifications(&db).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
