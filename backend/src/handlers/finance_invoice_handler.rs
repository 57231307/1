use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::services::finance_invoice_service::FinanceInvoiceService;
use crate::services::finance_invoice_service::{
    CreateInvoiceRequest, UpdateInvoiceRequest,
};

/// 发票响应
#[derive(Debug, Serialize)]
pub struct InvoiceResponse {
    pub id: i32,
    pub invoice_no: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: String,
    pub invoice_type: String,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub status: String,
    pub invoice_date: Option<chrono::DateTime<chrono::Utc>>,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub paid_date: Option<chrono::DateTime<chrono::Utc>>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 发票列表响应
#[derive(Debug, Serialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<InvoiceResponse>,
    pub total: u64,
}

/// 创建发票请求
#[derive(Debug, Deserialize)]
pub struct CreateInvoicePayload {
    pub invoice_no: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: String,
    pub invoice_type: String,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub status: Option<String>,
    pub invoice_date: Option<chrono::DateTime<chrono::Utc>>,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

/// 更新发票请求
#[derive(Debug, Deserialize)]
pub struct UpdateInvoicePayload {
    pub invoice_no: Option<String>,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub invoice_type: Option<String>,
    pub amount: Option<rust_decimal::Decimal>,
    pub tax_amount: Option<rust_decimal::Decimal>,
    pub total_amount: Option<rust_decimal::Decimal>,
    pub status: Option<String>,
    pub invoice_date: Option<chrono::DateTime<chrono::Utc>>,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub paid_date: Option<chrono::DateTime<chrono::Utc>>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

/// 获取发票列表
pub async fn list_invoices(
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<InvoiceListResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(db.clone());

    match service.list_invoices().await {
        Ok(invoices) => {
            let invoice_responses: Vec<InvoiceResponse> = invoices
                .into_iter()
                .map(|invoice| InvoiceResponse {
                    id: invoice.id,
                    invoice_no: invoice.invoice_no,
                    order_id: invoice.order_id,
                    customer_id: invoice.customer_id,
                    customer_name: invoice.customer_name,
                    invoice_type: invoice.invoice_type,
                    amount: invoice.amount,
                    tax_amount: invoice.tax_amount,
                    total_amount: invoice.total_amount,
                    status: invoice.status,
                    invoice_date: invoice.invoice_date,
                    due_date: invoice.due_date,
                    paid_date: invoice.paid_date,
                    payment_method: invoice.payment_method,
                    notes: invoice.notes,
                    created_at: invoice.created_at,
                    updated_at: invoice.updated_at,
                })
                .collect();

            let total = invoice_responses.len() as u64;

            Ok(Json(InvoiceListResponse {
                invoices: invoice_responses,
                total,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 获取发票详情
pub async fn get_invoice(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(db.clone());

    match service.get_invoice(id).await {
        Ok(invoice) => Ok(Json(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            customer_id: invoice.customer_id,
            customer_name: invoice.customer_name,
            invoice_type: invoice.invoice_type,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            due_date: invoice.due_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        })),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

/// 创建发票
pub async fn create_invoice(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateInvoicePayload>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(db.clone());

    let request = CreateInvoiceRequest {
        invoice_no: payload.invoice_no,
        order_id: payload.order_id,
        customer_id: payload.customer_id,
        customer_name: payload.customer_name,
        invoice_type: payload.invoice_type,
        amount: payload.amount,
        tax_amount: payload.tax_amount,
        total_amount: payload.total_amount,
        status: payload.status,
        invoice_date: payload.invoice_date,
        due_date: payload.due_date,
        payment_method: payload.payment_method,
        notes: payload.notes,
    };

    match service.create_invoice(request).await {
        Ok(invoice) => Ok(Json(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            customer_id: invoice.customer_id,
            customer_name: invoice.customer_name,
            invoice_type: invoice.invoice_type,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            due_date: invoice.due_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 更新发票
pub async fn update_invoice(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateInvoicePayload>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(db.clone());

    let request = UpdateInvoiceRequest {
        invoice_no: payload.invoice_no,
        order_id: payload.order_id,
        customer_id: payload.customer_id,
        customer_name: payload.customer_name,
        invoice_type: payload.invoice_type,
        amount: payload.amount,
        tax_amount: payload.tax_amount,
        total_amount: payload.total_amount,
        status: payload.status,
        invoice_date: payload.invoice_date,
        due_date: payload.due_date,
        paid_date: payload.paid_date,
        payment_method: payload.payment_method,
        notes: payload.notes,
    };

    match service.update_invoice(id, request).await {
        Ok(invoice) => Ok(Json(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            customer_id: invoice.customer_id,
            customer_name: invoice.customer_name,
            invoice_type: invoice.invoice_type,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            due_date: invoice.due_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 删除发票
pub async fn delete_invoice(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(db.clone());

    match service.delete_invoice(id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 审核发票
pub async fn approve_invoice(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(db.clone());

    match service.approve_invoice(id).await {
        Ok(invoice) => Ok(Json(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            customer_id: invoice.customer_id,
            customer_name: invoice.customer_name,
            invoice_type: invoice.invoice_type,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            due_date: invoice.due_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 核销发票
pub async fn verify_invoice(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(db.clone());

    let paid_date = chrono::Utc::now();
    let payment_method = payload.get("payment_method")
        .and_then(|v| v.as_str())
        .unwrap_or("bank_transfer")
        .to_string();

    match service.verify_invoice(id, paid_date, payment_method).await {
        Ok(invoice) => Ok(Json(InvoiceResponse {
            id: invoice.id,
            invoice_no: invoice.invoice_no,
            order_id: invoice.order_id,
            customer_id: invoice.customer_id,
            customer_name: invoice.customer_name,
            invoice_type: invoice.invoice_type,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            total_amount: invoice.total_amount,
            status: invoice.status,
            invoice_date: invoice.invoice_date,
            due_date: invoice.due_date,
            paid_date: invoice.paid_date,
            payment_method: invoice.payment_method,
            notes: invoice.notes,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
