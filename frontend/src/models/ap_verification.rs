//! 应付核销模型
//!
//! 应付核销相关的数据结构

use serde::{Deserialize, Serialize};

/// 应付核销数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApVerification {
    pub id: i32,
    pub verification_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub verification_type: String,
    pub verification_date: String,
    pub total_amount: String,
    pub currency_code: Option<String>,
    pub status: String,
    pub invoice_count: i32,
    pub payment_count: i32,
    pub invoice_ids: Option<Vec<i32>>,
    pub payment_ids: Option<Vec<i32>>,
    pub remarks: Option<String>,
    pub verifier_id: Option<i32>,
    pub verifier_name: Option<String>,
    pub verified_at: Option<String>,
    pub cancel_reason: Option<String>,
    pub cancelled_at: Option<String>,
    pub cancelled_by: Option<i32>,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应付核销列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApVerificationListResponse {
    pub items: Vec<ApVerification>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 应付核销查询参数
#[derive(Debug, Clone, Serialize)]
pub struct ApVerificationQueryParams {
    pub supplier_id: Option<i32>,
    pub verification_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 自动核销请求
#[derive(Debug, Clone, Serialize)]
pub struct AutoVerifyRequest {
    pub supplier_id: i32,
}

/// 手工核销请求
#[derive(Debug, Clone, Serialize)]
pub struct ManualVerifyRequest {
    pub supplier_id: i32,
    pub invoice_ids: Vec<i32>,
    pub payment_ids: Vec<i32>,
    pub verification_amount: String,
    pub remarks: Option<String>,
}

/// 取消核销请求
#[derive(Debug, Clone, Serialize)]
pub struct CancelVerificationRequest {
    pub reason: String,
}

/// 未核销应付发票项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnverifiedInvoiceItem {
    pub id: i32,
    pub invoice_no: String,
    pub invoice_date: String,
    pub due_date: Option<String>,
    pub total_amount: String,
    pub outstanding_amount: String,
    pub invoice_status: String,
}

/// 未核销付款单项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnverifiedPaymentItem {
    pub id: i32,
    pub payment_no: String,
    pub payment_date: String,
    pub payment_amount: String,
    pub outstanding_amount: String,
    pub payment_status: String,
}
