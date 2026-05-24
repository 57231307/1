//! 通用打印 Handler
//!
//! 支持所有单据类型的打印功能

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use serde::{Deserialize, Serialize};

use crate::services::print_service::PrintService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 单据类型
#[derive(Debug, Deserialize)]
pub struct PrintPath {
    pub doc_type: String,
    pub id: i32,
}

/// 打印响应
#[derive(Debug, Serialize)]
pub struct PrintResponse {
    pub url: String,
    pub html: Option<String>,
}

/// 获取打印数据
pub async fn get_print_data(
    Path(params): Path<PrintPath>,
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let service = PrintService::new();
    let print_data = service.get_print_data(&params.doc_type, params.id).await?;
    
    Ok(axum::Json(ApiResponse::success(print_data)))
}

/// 打印为 HTML
pub async fn print_html(
    Path(params): Path<PrintPath>,
    State(_state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let service = PrintService::new();
    let print_data = service.get_print_data(&params.doc_type, params.id).await?;
    let html = service.generate_pdf(&print_data)?;
    
    Ok(Html(html))
}

/// 打印为 PDF（返回 URL）
pub async fn print_pdf(
    Path(params): Path<PrintPath>,
    State(_state): State<AppState>,
) -> Result<axum::Json<ApiResponse<PrintResponse>>, AppError> {
    let service = PrintService::new();
    let _print_data = service.get_print_data(&params.doc_type, params.id).await?;
    
    // TODO: 实际生成 PDF
    let response = PrintResponse {
        url: format!("/prints/{}-{}.pdf", params.doc_type, params.id),
        html: None,
    };
    
    Ok(axum::Json(ApiResponse::success(response)))
}
