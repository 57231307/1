//! 财务发票模型
//!
//! 财务发票相关的数据结构

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
