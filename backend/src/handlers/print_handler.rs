//! 通用打印 Handler

use crate::services::print_service::PrintService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    response::Html,
};

async fn render_print_html(doc_type: &str, doc_id: i32) -> Result<Html<String>, AppError> {
    let service = PrintService::new();
    let print_data = service.get_print_data(doc_type, doc_id).await?;
    let html = service.generate_pdf(&print_data)?;
    Ok(Html(html))
}

pub async fn sales_order_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("sales_order", doc_id).await
}

pub async fn sales_contract_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("sales_contract", doc_id).await
}

pub async fn purchase_order_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("purchase_order", doc_id).await
}

pub async fn purchase_receipt_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("purchase_receipt", doc_id).await
}

pub async fn inventory_transfer_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("inventory_transfer", doc_id).await
}

pub async fn inventory_count_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("inventory_count", doc_id).await
}

/// 打印模板列表响应
#[derive(serde::Serialize)]
pub struct PrintTemplateDto {
    pub id: i32,
    pub name: String,
    pub doc_type: String,
    pub template_content: String,
    pub is_default: bool,
    pub created_at: String,
}

/// 获取打印模板列表
pub async fn list_print_templates(
    State(_): State<AppState>,
) -> Result<axum::Json<ApiResponse<Vec<PrintTemplateDto>>>, AppError> {
    // 打印模板功能暂返回空列表，后续可接入数据库
    Ok(axum::Json(ApiResponse::success(vec![])))
}

/// 获取单个打印模板详情
pub async fn get_print_template(
    Path(_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<axum::Json<ApiResponse<PrintTemplateDto>>, AppError> {
    Err(AppError::not_found("打印模板不存在"))
}
