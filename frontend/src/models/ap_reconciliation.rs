//! 应付对账模型
//!
//! 应付对账相关的数据结构

use serde::{Deserialize, Serialize};

/// 应付对账数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApReconciliation {
    pub id: i32,
    pub reconciliation_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub reconciliation_date: String,
    pub period_start: String,
    pub period_end: String,
    pub reconciliation_status: String,
    pub total_amount: String,
    pub confirmed_amount: Option<String>,
    pub disputed_amount: Option<String>,
    pub paid_amount: Option<String>,
    pub outstanding_amount: Option<String>,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub invoice_count: i32,
    pub confirmed_invoice_count: i32,
    pub disputed_invoice_count: i32,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub remarks: Option<String>,
    pub confirmed_at: Option<String>,
    pub confirmed_by: Option<i32>,
    pub confirmed_by_name: Option<String>,
    pub disputed_at: Option<String>,
    pub disputed_by: Option<i32>,
    pub disputed_by_name: Option<String>,
    pub dispute_reason: Option<String>,
    pub approver_id: Option<i32>,
    pub approver_name: Option<String>,
    pub approved_at: Option<String>,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应付对账明细项
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApReconciliationItem {
    pub id: i32,
    pub reconciliation_id: i32,
    pub invoice_id: Option<i32>,
    pub invoice_no: Option<String>,
    pub invoice_date: Option<String>,
    pub invoice_amount: String,
    pub confirmed_amount: Option<String>,
    pub disputed_amount: Option<String>,
    pub item_status: String,
    pub dispute_reason: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub warehouse_name: Option<String>,
    pub remarks: Option<String>,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应付对账列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApReconciliationListResponse {
    pub items: Vec<ApReconciliation>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 应付对账查询参数
#[derive(Debug, Clone, Serialize)]
pub struct ApReconciliationQueryParams {
    pub supplier_id: Option<i32>,
    pub reconciliation_status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 生成对账单请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct GenerateReconciliationRequest {
    pub supplier_id: i32,
    pub reconciliation_date: String,
    pub period_start: String,
    pub period_end: String,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub remarks: Option<String>,
}

/// 更新对账单请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReconciliationRequest {
    pub reconciliation_date: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub remarks: Option<String>,
}

/// 确认对账单请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ConfirmReconciliationRequest {
    pub confirmed_amount: String,
    pub remarks: Option<String>,
}

/// 争议请求
#[derive(Debug, Clone, Serialize)]
pub struct DisputeRequest {
    pub reason: String,
}

/// 供应商应付汇总数据
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SupplierSummary {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub total_reconciliations: i32,
    pub total_amount: String,
    pub confirmed_amount: String,
    pub disputed_amount: String,
    pub paid_amount: String,
    pub outstanding_amount: String,
}
