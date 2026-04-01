//! 应收单 Handler
//!
//! HTTP 接口层

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::middleware::auth_context::AuthContext;
use crate::models::ar_invoice;
use crate::services::ar_invoice_service::{ArInvoiceService, CreateArInvoiceRequest};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use rust_decimal::Decimal;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct ArInvoiceQuery {
    pub customer_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArInvoiceRequestDto {
    pub invoice_date: String,
    pub due_date: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub source_type: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub invoice_amount: Decimal,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub sales_order_no: Option<String>,
}

/// 查询应收单列表
pub async fn list_invoices(
    Query(params): Query<ArInvoiceQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ar_invoice::Model>>>, AppError> {
    info!("用户 {} 查询应收单列表", auth.username);

    let service = ArInvoiceService::new(state.db.clone());
    let (invoices, total) = service
        .get_list(
            params.customer_id,
            params.status,
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
        )
        .await?;

    info!("用户 {} 查询应收单成功，共 {} 条", auth.username, total);

    Ok(Json(ApiResponse::success(invoices)))
}

/// 创建应收单
#[axum::debug_handler]
pub async fn create_invoice(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateArInvoiceRequestDto>,
) -> Result<Json<ApiResponse<ar_invoice::Model>>, AppError> {
    info!(
        "用户 {} 创建应收单，客户 ID: {}",
        auth.username, req.customer_id
    );

    let invoice_date = req.invoice_date.parse().map_err(|e| {
        warn!("用户 {} 应收单日期格式错误：{}", auth.username, e);
        AppError::ValidationError("应收单日期格式错误".to_string())
    })?;

    let due_date = req.due_date.parse().map_err(|e| {
        warn!("用户 {} 到期日格式错误：{}", auth.username, e);
        AppError::ValidationError("到期日格式错误".to_string())
    })?;

    let create_req = CreateArInvoiceRequest {
        invoice_date,
        due_date,
        customer_id: req.customer_id,
        customer_name: req.customer_name,
        source_type: req.source_type,
        source_bill_id: req.source_bill_id,
        source_bill_no: req.source_bill_no,
        invoice_amount: req.invoice_amount,
        batch_no: req.batch_no,
        color_no: req.color_no,
        sales_order_no: req.sales_order_no,
    };

    let service = ArInvoiceService::new(state.db.clone());
    let invoice = service.create(create_req, auth.user_id).await?;
    info!(
        "用户 {} 创建应收单成功：{}",
        auth.username, invoice.invoice_no
    );

    Ok(Json(ApiResponse::success_with_message(
        invoice,
        "应收单创建成功",
    )))
}
