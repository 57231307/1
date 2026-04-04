use serde::{Deserialize, Serialize};
use serde_json::Number;

/// 凭证数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Voucher {
    pub id: i32,
    pub voucher_no: String,
    pub voucher_type: String,
    pub voucher_date: String,
    pub status: String,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub total_debit: String,
    pub total_credit: String,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub submitter_id: Option<i32>,
    pub submitter_name: Option<String>,
    pub reviewer_id: Option<i32>,
    pub reviewer_name: Option<String>,
    pub poster_id: Option<i32>,
    pub poster_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 凭证明细项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VoucherItem {
    pub id: i32,
    pub voucher_id: i32,
    pub line_no: i32,
    pub subject_code: String,
    pub subject_name: String,
    pub debit: String,
    pub credit: String,
    pub summary: Option<String>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub quantity_meters: Option<String>,
    pub quantity_kg: Option<String>,
}

/// 凭证列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct VoucherListResponse {
    pub data: Vec<Voucher>,
    pub total: u64,
}

/// 凭证查询参数
#[derive(Debug, Clone, Serialize)]
pub struct VoucherQueryParams {
    pub voucher_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建凭证请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateVoucherRequest {
    pub voucher_type: String,
    pub voucher_date: String,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub items: Vec<VoucherItemRequest>,
}

/// 凭证明细请求
#[derive(Debug, Clone, Serialize)]
pub struct VoucherItemRequest {
    pub line_no: i32,
    pub subject_code: String,
    pub subject_name: String,
    pub debit: Number,
    pub credit: Number,
    pub summary: Option<String>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub quantity_meters: Option<Number>,
    pub quantity_kg: Option<Number>,
}
