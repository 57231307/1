//! 辅助核算服务
//! 提供辅助核算数据的查询、统计和分析功能

use crate::models::assist_accounting::{
    AssistDimension, AssistRecord, AssistRecordListResponse, AssistRecordQueryParams,
    AssistSummary, AssistSummaryQueryParams, BusinessQueryParams,
};
use crate::services::api::ApiService;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

/// 辅助核算服务
pub struct AssistAccountingService;

impl AssistAccountingService {
    /// 获取所有辅助核算维度
    pub async fn list_dimensions() -> Result<Vec<AssistDimension>, String> {
        ApiService::get::<Vec<AssistDimension>>("/assist-accounting/dimensions").await
    }

    /// 查询辅助核算记录
    pub async fn query_records(
        params: AssistRecordQueryParams,
    ) -> Result<AssistRecordListResponse, String> {
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
    pub async fn get_records_by_business(
        params: BusinessQueryParams,
    ) -> Result<Vec<AssistRecord>, String> {
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
    pub async fn get_records_by_five_dimension(
        five_dimension_id: &str,
    ) -> Result<Vec<AssistRecord>, String> {
        let url = format!(
            "/assist-accounting/records/five-dimension/{}",
            utf8_percent_encode(five_dimension_id, NON_ALPHANUMERIC)
        );
        ApiService::get::<Vec<AssistRecord>>(&url).await
    }

    /// 获取辅助核算汇总
    pub async fn get_summary(
        params: AssistSummaryQueryParams,
    ) -> Result<Vec<AssistSummary>, String> {
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
            parts.push(format!(
                "accounting_period={}",
                utf8_percent_encode(accounting_period, NON_ALPHANUMERIC)
            ));
        }
    }
    if let Some(ref dimension_code) = params.dimension_code {
        if !dimension_code.is_empty() {
            parts.push(format!(
                "dimension_code={}",
                utf8_percent_encode(dimension_code, NON_ALPHANUMERIC)
            ));
        }
    }
    if let Some(ref business_type) = params.business_type {
        if !business_type.is_empty() {
            parts.push(format!(
                "business_type={}",
                utf8_percent_encode(business_type, NON_ALPHANUMERIC)
            ));
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
        parts.push(format!(
            "accounting_period={}",
            utf8_percent_encode(&params.accounting_period, NON_ALPHANUMERIC)
        ));
    }
    if let Some(ref dimension_code) = params.dimension_code {
        if !dimension_code.is_empty() {
            parts.push(format!(
                "dimension_code={}",
                utf8_percent_encode(dimension_code, NON_ALPHANUMERIC)
            ));
        }
    }

    parts.join("&")
}
