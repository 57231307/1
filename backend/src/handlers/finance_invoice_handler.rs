use crate::services::finance_invoice_service::FinanceInvoiceService;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};

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

pub async fn list_finance_invoices(
    State(state): State<AppState>,
) -> Result<Json<InvoiceListResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.list_invoices().await {
        Ok(invoices) => {
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

            Ok(Json(InvoiceListResponse {
                invoices: invoice_responses,
                total,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn get_finance_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.get_invoice(id).await {
        Ok(Some(invoice)) => Ok(Json(InvoiceResponse {
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
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, "发票不存在".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn create_finance_invoice(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(state.db.clone());

    let invoice_no = payload.get("invoice_no")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let amount = payload.get("amount")
        .and_then(|v| v.as_f64())
        .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default())
        .unwrap_or_default();

    let tax_amount = payload.get("tax_amount")
        .and_then(|v| v.as_f64())
        .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default())
        .unwrap_or_default();

    let total_amount = payload.get("total_amount")
        .and_then(|v| v.as_f64())
        .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default())
        .unwrap_or_default();

    match service.create_invoice(invoice_no, amount, tax_amount, total_amount).await {
        Ok(invoice) => Ok(Json(InvoiceResponse {
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
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn update_finance_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.update_invoice(id, payload).await {
        Ok(Some(invoice)) => Ok(Json(InvoiceResponse {
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
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, "发票不存在".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn delete_finance_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.delete_invoice(id).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn approve_finance_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.approve_invoice(id).await {
        Ok(Some(invoice)) => Ok(Json(InvoiceResponse {
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
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, "发票不存在".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn verify_invoice(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<InvoiceResponse>, (StatusCode, String)> {
    let service = FinanceInvoiceService::new(state.db.clone());

    match service.verify_invoice(id).await {
        Ok(Some(invoice)) => Ok(Json(InvoiceResponse {
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
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, "发票不存在".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
