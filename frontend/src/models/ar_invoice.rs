//! 应收发票模型
//!
//! 应收发票相关的数据结构

use serde::{Deserialize, Serialize};

/// 应收发票数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArInvoice {
    pub id: i32,
    pub invoice_no: String,
    pub invoice_date: String,
    pub due_date: String,
    // 客户信息
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub customer_code: Option<String>,
    // 来源单据
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    // 面料行业字段
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub sales_order_no: Option<String>,
    // 金额
    pub invoice_amount: String,
    pub received_amount: String,
    pub unpaid_amount: String,
    pub tax_amount: Option<String>,
    // 双计量单位
    pub quantity_meters: Option<String>,
    pub quantity_kg: Option<String>,
    pub unit_price: Option<String>,
    // 状态
    pub status: String,
    pub approval_status: String,
    // 审核信息
    pub created_by: i32,
    pub creator_name: Option<String>,
    pub reviewed_by: Option<i32>,
    pub reviewer_name: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应收发票列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ArInvoiceListResponse {
    pub data: Vec<ArInvoice>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 应收发票查询参数
#[derive(Debug, Clone, Serialize)]
pub struct ArInvoiceQueryParams {
    pub customer_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建应收发票请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CreateArInvoiceRequest {
    pub invoice_date: String,
    pub due_date: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub source_type: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub invoice_amount: String,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub sales_order_no: Option<String>,
}

/// 更新应收发票请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateArInvoiceRequest {
    pub invoice_date: Option<String>,
    pub due_date: Option<String>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub invoice_amount: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub sales_order_no: Option<String>,
    pub status: Option<String>,
}
