//! 出口商检处理器
//! V15 P2 B08-12

use crate::container::AppState;
use crate::services::export_inspection_service::{ExportInspectionService, ListParams};
use crate::utils::error::AppError;
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

pub async fn list_inspections(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = ExportInspectionService::new(state.db.clone());
    let (items, total) = service
        .list(ListParams {
            sales_order_id: params.sales_order_id,
            inspection_no: params.inspection_no,
            result: params.result,
            page: params.page,
            page_size: params.page_size,
        })
        .await?;
    Ok(Json(serde_json::json!({ "items": items, "total": total })))
}

pub async fn get_inspection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = ExportInspectionService::new(state.db.clone());
    let item = service.get_by_id(id).await?;
    Ok(Json(serde_json::json!(item)))
}

#[derive(Deserialize)]
pub struct ListQuery {
    sales_order_id: Option<i32>,
    inspection_no: Option<String>,
    result: Option<String>,
    page: Option<u64>,
    page_size: Option<u64>,
}
