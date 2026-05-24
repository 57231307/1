//! 通用打印 Handler

use axum::{
    extract::{Path, State},
    response::Html,
};
use crate::services::print_service::PrintService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

async fn render_print_html(doc_type: &str, doc_id: i32) -> Result<Html<String>, AppError> {
    let service = PrintService::new();
    let print_data = service.get_print_data(doc_type, doc_id).await?;
    let html = service.generate_pdf(&print_data)?;
    Ok(Html(html))
}

async fn render_print_json(doc_type: &str, doc_id: i32) -> Result<axum::Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PrintService::new();
    let print_data = service.get_print_data(doc_type, doc_id).await?;
    let json_data = serde_json::to_value(&print_data)?;
    Ok(axum::Json(ApiResponse::success(json_data)))
}

pub async fn sales_order_print_html(Path(doc_id): Path<i32>, State(_): State<AppState>) -> Result<Html<String>, AppError> {
    render_print_html("sales_order", doc_id).await
}

pub async fn sales_contract_print_html(Path(doc_id): Path<i32>, State(_): State<AppState>) -> Result<Html<String>, AppError> {
    render_print_html("sales_contract", doc_id).await
}

pub async fn purchase_order_print_html(Path(doc_id): Path<i32>, State(_): State<AppState>) -> Result<Html<String>, AppError> {
    render_print_html("purchase_order", doc_id).await
}

pub async fn purchase_receipt_print_html(Path(doc_id): Path<i32>, State(_): State<AppState>) -> Result<Html<String>, AppError> {
    render_print_html("purchase_receipt", doc_id).await
}

pub async fn inventory_transfer_print_html(Path(doc_id): Path<i32>, State(_): State<AppState>) -> Result<Html<String>, AppError> {
    render_print_html("inventory_transfer", doc_id).await
}

pub async fn inventory_count_print_html(Path(doc_id): Path<i32>, State(_): State<AppState>) -> Result<Html<String>, AppError> {
    render_print_html("inventory_count", doc_id).await
}

pub async fn voucher_print_html(Path(doc_id): Path<i32>, State(_): State<AppState>) -> Result<Html<String>, AppError> {
    render_print_html("voucher", doc_id).await
}
