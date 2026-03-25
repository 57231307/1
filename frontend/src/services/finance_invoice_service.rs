//! 财务发票服务
//!
//! 与后端财务发票API交互

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 财务发票数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FinanceInvoice {
    pub id: i32,
    pub invoice_no: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: String,
    pub invoice_type: String,
    pub amount: String,
    pub tax_amount: String,
    pub total_amount: String,
    pub status: String,
    pub invoice_date: Option<String>,
    pub due_date: Option<String>,
    pub paid_date: Option<String>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 财务发票列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<FinanceInvoice>,
    pub total: u64,
}

/// 财务发票查询参数
#[derive(Debug, Clone, Serialize)]
pub struct InvoiceQueryParams {
    pub customer_id: Option<i32>,
    pub status: Option<String>,
    pub invoice_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建财务发票请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CreateInvoiceRequest {
    pub invoice_no: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: String,
    pub invoice_type: String,
    pub amount: String,
    pub tax_amount: String,
    pub total_amount: String,
    pub status: Option<String>,
    pub invoice_date: Option<String>,
    pub due_date: Option<String>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

/// 更新财务发票请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInvoiceRequest {
    pub invoice_no: Option<String>,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub invoice_type: Option<String>,
    pub amount: Option<String>,
    pub tax_amount: Option<String>,
    pub total_amount: Option<String>,
    pub status: Option<String>,
    pub invoice_date: Option<String>,
    pub due_date: Option<String>,
    pub paid_date: Option<String>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

/// 核销发票请求
#[derive(Debug, Clone, Serialize)]
pub struct VerifyInvoiceRequest {
    pub payment_method: String,
}

/// 财务发票服务
pub struct FinanceInvoiceService;

impl FinanceInvoiceService {
    /// 查询财务发票列表
    pub async fn list_invoices(params: InvoiceQueryParams) -> Result<InvoiceListResponse, String> {
        let mut query_parts = vec![];

        if let Some(cid) = params.customer_id {
            query_parts.push(format!("customer_id={}", cid));
        }
        if let Some(ref status) = params.status {
            query_parts.push(format!("status={}", status));
        }
        if let Some(ref itype) = params.invoice_type {
            query_parts.push(format!("invoice_type={}", itype));
        }
        if let Some(ref sd) = params.start_date {
            query_parts.push(format!("start_date={}", sd));
        }
        if let Some(ref ed) = params.end_date {
            query_parts.push(format!("end_date={}", ed));
        }
        if let Some(p) = params.page {
            query_parts.push(format!("page={}", p));
        }
        if let Some(ps) = params.page_size {
            query_parts.push(format!("page_size={}", ps));
        }

        let query_string = if query_parts.is_empty() {
            String::new()
        } else {
            format!("?{}", query_parts.join("&"))
        };

        let url = format!("/finance-invoices{}", query_string);
        ApiService::get::<InvoiceListResponse>(&url).await
    }

    /// 获取财务发票详情
    #[allow(dead_code)]
    pub async fn get_invoice(id: i32) -> Result<FinanceInvoice, String> {
        ApiService::get::<FinanceInvoice>(&format!("/finance-invoices/{}", id)).await
    }

    /// 创建财务发票
    #[allow(dead_code)]
    pub async fn create_invoice(req: CreateInvoiceRequest) -> Result<FinanceInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/finance-invoices", &payload).await
    }

    /// 更新财务发票
    #[allow(dead_code)]
    pub async fn update_invoice(id: i32, req: UpdateInvoiceRequest) -> Result<FinanceInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/finance-invoices/{}", id), &payload).await
    }

    /// 删除财务发票
    pub async fn delete_invoice(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/finance-invoices/{}", id)).await
    }

    /// 审核财务发票
    pub async fn approve_invoice(id: i32) -> Result<FinanceInvoice, String> {
        ApiService::post::<FinanceInvoice>(&format!("/finance-invoices/{}/approve", id), &serde_json::json!({})).await
    }

    /// 核销财务发票
    pub async fn verify_invoice(id: i32, payment_method: String) -> Result<FinanceInvoice, String> {
        let req = VerifyInvoiceRequest { payment_method };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/finance-invoices/{}/verify", id), &payload).await
    }
}