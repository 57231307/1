use axum::{Json};
use serde::{Deserialize, Serialize};
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct PageViewRequest {
    pub path: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct PageViewResponse {
    pub success: bool,
}

/// 接收前端页面访问埋点 (低优先级，可选功能)
pub async fn track_page_view(
    Json(req): Json<PageViewRequest>,
) -> Result<Json<ApiResponse<PageViewResponse>>, axum::http::StatusCode> {
    tracing::debug!("Page view tracked: path={}, timestamp={}", req.path, req.timestamp);
    Ok(Json(ApiResponse::success(PageViewResponse { success: true })))
}
