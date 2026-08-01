//! 出口产地证处理器
//! V15 P2 B08-12

use crate::container::AppState;
use crate::services::certificate_of_origin_service::{CertificateOfOriginService, ListParams};
use crate::utils::error::AppError;
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

pub async fn list_certificates(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = CertificateOfOriginService::new(state.db.clone());
    let (items, total) = service
        .list(ListParams {
            inspection_id: params.inspection_id,
            status: params.status,
            page: params.page,
            page_size: params.page_size,
        })
        .await?;
    Ok(Json(serde_json::json!({ "items": items, "total": total })))
}

pub async fn get_certificate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = CertificateOfOriginService::new(state.db.clone());
    let item = service.get_by_id(id).await?;
    Ok(Json(serde_json::json!(item)))
}

#[derive(Deserialize)]
pub struct ListQuery {
    inspection_id: Option<i32>,
    status: Option<String>,
    page: Option<u64>,
    page_size: Option<u64>,
}
