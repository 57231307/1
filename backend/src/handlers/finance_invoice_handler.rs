use crate::services::finance_invoice_service::FinanceInvoiceService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct InvoiceResponse {
    pub id: i32,
    pub invoice_no: String,
    pub order_id: Option<i32>,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub status: String,
    pub invoice_date: chrono::DateTime<chrono::Utc>,
    pub paid_date: Option<chrono::DateTime<chrono::Utc>>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<InvoiceResponse>,
    pub total: u64,
}

/// 创建发票请求 DTO
/// 用于强类型校验 create_finance_invoice 的入参，
/// 替代原先无类型校验的 serde_json::Value。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateFinanceInvoiceDto {
    /// 发票号：必填，长度至少 1
    #[validate(length(min = 1, message = "发票号不能为空"))]
    pub invoice_no: String,
    /// 发票金额：必填，必须为非负数
    #[validate(range(min = 0.0, message = "发票金额不能为负"))]
    pub amount: f64,
    /// 税额：必填，必须为非负数
    #[validate(range(min = 0.0, message = "税额不能为负"))]
    pub tax_amount: f64,
    /// 价税合计：必填，必须为非负数
    #[validate(range(min = 0.0, message = "价税合计不能为负"))]
    pub total_amount: f64,
}

/// 更新发票请求 DTO
/// 用于强类型校验 update_finance_invoice 的入参。
/// 字段全部可选，仅更新提交的字段。
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateFinanceInvoiceDto {
    /// 发票状态：可选；若提供则长度至少 1
    #[validate(length(min = 1, message = "状态不能为空"))]
    pub status: Option<String>,
    /// 备注：可选
    pub notes: Option<String>,
}

pub async fn list_finance_invoices(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<InvoiceListResponse>>, AppError> {
    let service = FinanceInvoiceService::new(state.db.clone());

    let invoices = service
        .list_invoices()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let invoice_responses: Vec<InvoiceResponse> = invoices
        .into_iter()
        .map(|invoice| InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        })
        .collect();

    let total = invoice_responses.len() as u64;

    Ok(Json(ApiResponse::success(InvoiceListResponse {
        invoices: invoice_responses,
        total,
    })))
}

pub async fn get_finance_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<InvoiceResponse>>, AppError> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.get_invoice(id).await {
        Ok(Some(invoice)) => Ok(Json(ApiResponse::success(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        }))),
        Ok(None) => Err(AppError::not_found("发票不存在")),
        Err(e) => Err(AppError::internal(e.to_string())),
    }
}

pub async fn create_finance_invoice(
    State(state): State<AppState>,
    Json(payload): Json<CreateFinanceInvoiceDto>,
) -> Result<Json<ApiResponse<InvoiceResponse>>, AppError> {
    // 强类型校验：替代原先无校验的 serde_json::Value
    payload
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = FinanceInvoiceService::new(state.db.clone());

    // 从 DTO 字段取值（替代原先的 payload.get(...).as_f64()）
    let invoice_no = payload.invoice_no;
    let amount =
        rust_decimal::Decimal::from_f64_retain(payload.amount).unwrap_or_default();
    let tax_amount =
        rust_decimal::Decimal::from_f64_retain(payload.tax_amount).unwrap_or_default();
    let total_amount =
        rust_decimal::Decimal::from_f64_retain(payload.total_amount).unwrap_or_default();

    let invoice = service
        .create_invoice(invoice_no, amount, tax_amount, total_amount)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(InvoiceResponse {
        id: invoice.id,
        invoice_no: invoice.invoice_no,
        order_id: invoice.order_id,
        amount: invoice.amount,
        tax_amount: invoice.tax_amount,
        total_amount: invoice.total_amount,
        status: invoice.status,
        invoice_date: invoice.invoice_date,
        paid_date: invoice.paid_date,
        payment_method: invoice.payment_method,
        notes: invoice.notes,
        created_at: invoice.created_at,
        updated_at: invoice.updated_at,
    })))
}

pub async fn update_finance_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateFinanceInvoiceDto>,
) -> Result<Json<ApiResponse<InvoiceResponse>>, AppError> {
    // 强类型校验：替代原先无校验的 serde_json::Value
    payload
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = FinanceInvoiceService::new(state.db.clone());

    // service.update_invoice 仍接收 serde_json::Value，
    // 将强类型 DTO 序列化为 Value 传入以保持服务层签名兼容。
    let payload_value = serde_json::to_value(&payload)
        .map_err(|e| AppError::internal(format!("DTO 序列化失败：{}", e)))?;

    match service.update_invoice(id, payload_value).await {
        Ok(Some(invoice)) => Ok(Json(ApiResponse::success(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        }))),
        Ok(None) => Err(AppError::not_found("发票不存在")),
        Err(e) => Err(AppError::internal(e.to_string())),
    }
}

pub async fn delete_finance_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = FinanceInvoiceService::new(state.db.clone());

    service
        .delete_invoice(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn approve_finance_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<InvoiceResponse>>, AppError> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.approve_invoice(id).await {
        Ok(Some(invoice)) => Ok(Json(ApiResponse::success(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        }))),
        Ok(None) => Err(AppError::not_found("发票不存在")),
        Err(e) => Err(AppError::internal(e.to_string())),
    }
}

pub async fn verify_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<InvoiceResponse>>, AppError> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.verify_invoice(id).await {
        Ok(Some(invoice)) => Ok(Json(ApiResponse::success(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        }))),
        Ok(None) => Err(AppError::not_found("发票不存在")),
        Err(e) => Err(AppError::internal(e.to_string())),
    }
}
