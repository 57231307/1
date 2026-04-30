//! 辅助核算模型
//!
//! 辅助核算相关的数据结构

use serde::{Deserialize, Serialize};
use serde_json;

/// 辅助核算维度
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssistDimension {
    pub id: i32,
    pub dimension_code: String,
    pub dimension_name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
}

/// 辅助核算记录
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssistRecord {
    pub id: i32,
    pub business_type: String,
    pub business_no: String,
    pub business_id: i32,
    pub account_subject_id: i32,
    pub debit_amount: serde_json::Value,
    pub credit_amount: serde_json::Value,
    pub five_dimension_id: String,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub warehouse_id: i32,
    pub quantity_meters: serde_json::Value,
    pub quantity_kg: serde_json::Value,
    pub workshop_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_at: String,
}

/// 辅助核算汇总
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssistSummary {
    pub id: i32,
    pub accounting_period: String,
    pub dimension_code: String,
    pub dimension_value_id: i32,
    pub dimension_value_name: String,
    pub account_subject_id: i32,
    pub total_debit: serde_json::Value,
    pub total_credit: serde_json::Value,
    pub total_quantity_meters: serde_json::Value,
    pub total_quantity_kg: serde_json::Value,
    pub record_count: i64,
}

/// 辅助核算记录列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct AssistRecordListResponse {
    pub records: Vec<AssistRecord>,
    #[allow(dead_code)]
    pub total: u64,
    #[allow(dead_code)]
    pub page: u64,
    #[allow(dead_code)]
    pub page_size: u64,
}

/// 辅助核算查询参数
#[derive(Debug, Clone, Serialize)]
pub struct AssistRecordQueryParams {
    pub accounting_period: Option<String>,
    pub dimension_code: Option<String>,
    pub business_type: Option<String>,
    pub warehouse_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 辅助核算汇总查询参数
#[derive(Debug, Clone, Serialize)]
pub struct AssistSummaryQueryParams {
    pub accounting_period: String,
    pub dimension_code: Option<String>,
}

/// 业务单查询参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct BusinessQueryParams {
    pub business_type: String,
    pub business_no: String,
}
