//! 辅助核算服务
//! 提供辅助核算数据的查询、统计和分析功能

use crate::services::api::ApiService;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

/// 辅助核算维度
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AssistDimension {
    pub id: i32,
    pub dimension_code: String,
    pub dimension_name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
}

/// 辅助核算记录
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssistRecordQueryParams {
    pub accounting_period: Option<String>,
    pub dimension_code: Option<String>,
    pub business_type: Option<String>,
    pub warehouse_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 辅助核算汇总查询参数
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssistSummaryQueryParams {
    pub accounting_period: String,
    pub dimension_code: Option<String>,
}

/// 业务单查询参数
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct BusinessQueryParams {
    pub business_type: String,
    pub business_no: String,
}

/// 辅助核算服务
pub struct AssistAccountingService;

impl AssistAccountingService {
    /// 获取所有辅助核算维度
    pub async fn list_dimensions() -> Result<Vec<AssistDimension>, String> {
        ApiService::get::<Vec<AssistDimension>>("/assist-accounting/dimensions").await
    }

    /// 查询辅助核算记录
    pub async fn query_records(params: AssistRecordQueryParams) -> Result<AssistRecordListResponse, String> {
        let query_string = build_record_query_string(&params);
        let url = if query_string.is_empty() {
            "/assist-accounting/records".to_string()
        } else {
            format!("/assist-accounting/records?{}", query_string)
        };
        ApiService::get::<AssistRecordListResponse>(&url).await
    }

    /// 按业务单查询辅助核算记录
    #[allow(dead_code)]
    pub async fn get_records_by_business(params: BusinessQueryParams) -> Result<Vec<AssistRecord>, String> {
        let query_string = format!(
            "business_type={}&business_no={}",
            utf8_percent_encode(&params.business_type, NON_ALPHANUMERIC),
            utf8_percent_encode(&params.business_no, NON_ALPHANUMERIC)
        );
        let url = format!("/assist-accounting/records/business?{}", query_string);
        ApiService::get::<Vec<AssistRecord>>(&url).await
    }

    /// 按五维ID查询辅助核算记录
    #[allow(dead_code)]
    pub async fn get_records_by_five_dimension(five_dimension_id: &str) -> Result<Vec<AssistRecord>, String> {
        let url = format!("/assist-accounting/records/five-dimension/{}", utf8_percent_encode(five_dimension_id, NON_ALPHANUMERIC));
        ApiService::get::<Vec<AssistRecord>>(&url).await
    }

    /// 获取辅助核算汇总
    pub async fn get_summary(params: AssistSummaryQueryParams) -> Result<Vec<AssistSummary>, String> {
        let query_string = build_summary_query_string(&params);
        let url = if query_string.is_empty() {
            "/assist-accounting/summary".to_string()
        } else {
            format!("/assist-accounting/summary?{}", query_string)
        };
        ApiService::get::<Vec<AssistSummary>>(&url).await
    }
}

/// 构建记录查询字符串
fn build_record_query_string(params: &AssistRecordQueryParams) -> String {
    let mut parts = vec![];

    if let Some(ref accounting_period) = params.accounting_period {
        if !accounting_period.is_empty() {
            parts.push(format!("accounting_period={}", utf8_percent_encode(accounting_period, NON_ALPHANUMERIC)));
        }
    }
    if let Some(ref dimension_code) = params.dimension_code {
        if !dimension_code.is_empty() {
            parts.push(format!("dimension_code={}", utf8_percent_encode(dimension_code, NON_ALPHANUMERIC)));
        }
    }
    if let Some(ref business_type) = params.business_type {
        if !business_type.is_empty() {
            parts.push(format!("business_type={}", utf8_percent_encode(business_type, NON_ALPHANUMERIC)));
        }
    }
    if let Some(warehouse_id) = params.warehouse_id {
        parts.push(format!("warehouse_id={}", warehouse_id));
    }
    if let Some(page) = params.page {
        parts.push(format!("page={}", page));
    }
    if let Some(page_size) = params.page_size {
        parts.push(format!("page_size={}", page_size));
    }

    parts.join("&")
}

/// 构建汇总查询字符串
fn build_summary_query_string(params: &AssistSummaryQueryParams) -> String {
    let mut parts = vec![];

    if !params.accounting_period.is_empty() {
        parts.push(format!("accounting_period={}", utf8_percent_encode(&params.accounting_period, NON_ALPHANUMERIC)));
    }
    if let Some(ref dimension_code) = params.dimension_code {
        if !dimension_code.is_empty() {
            parts.push(format!("dimension_code={}", utf8_percent_encode(dimension_code, NON_ALPHANUMERIC)));
        }
    }

    parts.join("&")
}