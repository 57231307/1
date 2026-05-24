//! 通用打印 Handler

use axum::{
    extract::{Path, State},
    response::Html,
};
use crate::services::print_service::PrintService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 获取打印数据
pub async fn get_print_data(
    Path(params): Path<(String, i32)>,
    State(_state): State<AppState>,
) -> Result<axum::Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PrintService::new();
    let print_data = service.get_print_data(&params.0, params.1).await?;
    let json_data = serde_json::to_value(&print_data)?;
    
    Ok(axum::Json(ApiResponse::success(json_data)))
}

/// 打印为 HTML
pub async fn print_html(
    Path(params): Path<(String, i32)>,
    State(_state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let service = PrintService::new();
    let print_data = service.get_print_data(&params.0, params.1).await?;
    let html = service.generate_pdf(&print_data)?;
    
    Ok(Html(html))
}
