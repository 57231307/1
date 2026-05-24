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
    Path(doc_id): Path<i32>,
    State(_state): State<AppState>,
) -> Result<axum::Json<ApiResponse<serde_json::Value>>, AppError> {
    // TODO: 需要根据路由确定 doc_type，暂时返回示例
    let service = PrintService::new();
    let print_data = service.get_print_data("sales_order", doc_id).await?;
    let json_data = serde_json::to_value(&print_data)?;
    
    Ok(axum::Json(ApiResponse::success(json_data)))
}

/// 打印为 HTML - 通用实现，需要从上下文获取 doc_type
pub async fn print_html(
    Path(doc_id): Path<i32>,
    State(_state): State<AppState>,
) -> Result<Html<String>, AppError> {
    // TODO: 需要根据路由确定 doc_type，这里使用默认模板
    let service = PrintService::new();
    let print_data = service.get_print_data("order", doc_id).await?;
    let html = service.generate_pdf(&print_data)?;
    
    Ok(Html(html))
}
