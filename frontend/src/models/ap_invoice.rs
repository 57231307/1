/// 应付发票数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ApInvoice {
    pub id: i32,
    pub invoice_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub invoice_date: String,
    pub due_date: Option<String>,
    pub invoice_status: String,
    pub invoice_type: String,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub total_amount: String,
    pub tax_amount: Option<String>,
    pub paid_amount: Option<String>,
    pub outstanding_amount: Option<String>,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub receipt_id: Option<i32>,
    pub receipt_no: Option<String>,
    pub remarks: Option<String>,
    pub approver_id: Option<i32>,
    pub approver_name: Option<String>,
    pub approved_at: Option<String>,
    pub cancel_reason: Option<String>,
    pub cancelled_at: Option<String>,
    pub cancelled_by: Option<i32>,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应付发票明细项
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ApInvoiceItem {
    pub id: i32,
    pub invoice_id: i32,
    pub line_no: i32,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub amount: String,
    pub tax_rate: Option<String>,
    pub tax_amount: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub warehouse_name: Option<String>,
}

/// 应付发票列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApInvoiceListResponse {
    pub data: Vec<ApInvoice>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 应付发票查询参数
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApInvoiceQueryParams {
    pub supplier_id: Option<i32>,
    pub invoice_status: Option<String>,
    pub invoice_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建应付发票请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateApInvoiceRequest {
    pub supplier_id: i32,
    pub invoice_date: String,
    pub due_date: Option<String>,
    pub invoice_type: String,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub total_amount: String,
    pub tax_amount: Option<String>,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub remarks: Option<String>,
    pub items: Vec<ApInvoiceItemRequest>,
}

/// 更新应付发票请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateApInvoiceRequest {
    pub supplier_id: Option<i32>,
    pub invoice_date: Option<String>,
    pub due_date: Option<String>,
    pub invoice_type: Option<String>,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub total_amount: Option<String>,
    pub tax_amount: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<ApInvoiceItemRequest>>,
}

/// 应付发票明细项请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApInvoiceItemRequest {
    pub line_no: i32,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub amount: String,
    pub tax_rate: Option<String>,
    pub tax_amount: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub warehouse_id: Option<i32>,
}

/// 取消应付发票请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CancelInvoiceRequest {
    pub reason: String,
}

/// 自动生成应付发票请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct AutoGenerateRequest {
    pub receipt_id: i32,
}

/// 账龄分析数据
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AgingAnalysisItem {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub current_amount: String,
    pub days_1_30: String,
    pub days_31_60: String,
    pub days_61_90: String,
    pub days_over_90: String,
    pub total_outstanding: String,
}

/// 应付余额汇总
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BalanceSummaryItem {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub invoice_count: i32,
    pub total_amount: String,
    pub paid_amount: String,
    pub outstanding_amount: String,
}
