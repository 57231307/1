//! 应付报表服务
//!
//! 与后端应付报表API交互

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 应付统计报表查询参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ApStatisticsQueryParams {
    pub supplier_id: Option<i32>,
    pub start_date: String,
    pub end_date: String,
}

/// 应付日报查询参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ApDailyQueryParams {
    pub supplier_id: Option<i32>,
    pub report_date: String,
}

/// 应付月报查询参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ApMonthlyQueryParams {
    pub supplier_id: Option<i32>,
    pub year: i32,
    pub month: u32,
}

/// 账龄分析查询参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ApAgingQueryParams {
    pub supplier_id: Option<i32>,
}

/// 统计报表数据项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApStatisticsItem {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub total_amount: String,
    pub paid_amount: String,
    pub outstanding_amount: String,
    pub invoice_count: i32,
}

/// 统计报表汇总
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApStatisticsSummary {
    pub total_amount: String,
    pub total_paid: String,
    pub total_outstanding: String,
    pub supplier_count: i32,
    pub invoice_count: i32,
}

/// 统计报表响应
#[derive(Debug, Clone, Deserialize)]
pub struct ApStatisticsResponse {
    pub items: Vec<ApStatisticsItem>,
    pub summary: ApStatisticsSummary,
}

/// 应付日报数据项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApDailyItem {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub invoice_count: i32,
    pub new_amount: String,
    pub paid_amount: String,
    pub outstanding_amount: String,
}

/// 应付日报汇总
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApDailySummary {
    pub total_new_amount: String,
    pub total_paid_amount: String,
    pub total_outstanding: String,
    pub supplier_count: i32,
}

/// 应付日报响应
#[derive(Debug, Clone, Deserialize)]
pub struct ApDailyResponse {
    pub items: Vec<ApDailyItem>,
    pub summary: ApDailySummary,
}

/// 应付月报数据项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApMonthlyItem {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub invoice_count: i32,
    pub month_amount: String,
    pub paid_amount: String,
    pub outstanding_amount: String,
}

/// 应付月报汇总
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApMonthlySummary {
    pub total_month_amount: String,
    pub total_paid: String,
    pub total_outstanding: String,
    pub supplier_count: i32,
}

/// 应付月报响应
#[derive(Debug, Clone, Deserialize)]
pub struct ApMonthlyResponse {
    pub items: Vec<ApMonthlyItem>,
    pub summary: ApMonthlySummary,
}

/// 账龄分析数据项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApAgingItem {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub total_outstanding: String,
    pub current_amount: String,
    pub days_1_30: String,
    pub days_31_60: String,
    pub days_61_90: String,
    pub days_over_90: String,
    pub invoice_count: i32,
}

/// 账龄分析汇总
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApAgingSummary {
    pub total_outstanding: String,
    pub total_current: String,
    pub total_1_30: String,
    pub total_31_60: String,
    pub total_61_90: String,
    pub total_over_90: String,
    pub supplier_count: i32,
}

/// 账龄分析响应
#[derive(Debug, Clone, Deserialize)]
pub struct ApAgingResponse {
    pub items: Vec<ApAgingItem>,
    pub summary: ApAgingSummary,
}

/// 应付报表服务
pub struct ApReportService;

impl ApReportService {
    /// 获取应付统计报表
    pub async fn get_statistics_report(
        supplier_id: Option<i32>,
        start_date: String,
        end_date: String,
    ) -> Result<ApStatisticsResponse, String> {
        let mut query_parts = vec![];

        query_parts.push(format!("start_date={}", start_date));
        query_parts.push(format!("end_date={}", end_date));

        if let Some(sid) = supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }

        let query_string = if query_parts.is_empty() {
            String::new()
        } else {
            format!("?{}", query_parts.join("&"))
        };

        let url = format!("/ap-reports/statistics{}", query_string);
        ApiService::get::<ApStatisticsResponse>(&url).await
    }

    /// 获取应付日报
    pub async fn get_daily_report(
        report_date: String,
        supplier_id: Option<i32>,
    ) -> Result<ApDailyResponse, String> {
        let mut query_parts = vec![];

        query_parts.push(format!("report_date={}", report_date));

        if let Some(sid) = supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }

        let query_string = if query_parts.is_empty() {
            String::new()
        } else {
            format!("?{}", query_parts.join("&"))
        };

        let url = format!("/ap-reports/daily{}", query_string);
        ApiService::get::<ApDailyResponse>(&url).await
    }

    /// 获取应付月报
    pub async fn get_monthly_report(
        year: i32,
        month: u32,
        supplier_id: Option<i32>,
    ) -> Result<ApMonthlyResponse, String> {
        let mut query_parts = vec![];

        query_parts.push(format!("year={}", year));
        query_parts.push(format!("month={}", month));

        if let Some(sid) = supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }

        let query_string = if query_parts.is_empty() {
            String::new()
        } else {
            format!("?{}", query_parts.join("&"))
        };

        let url = format!("/ap-reports/monthly{}", query_string);
        ApiService::get::<ApMonthlyResponse>(&url).await
    }

    /// 获取账龄分析报告
    pub async fn get_aging_report(
        supplier_id: Option<i32>,
    ) -> Result<ApAgingResponse, String> {
        let query_string = if let Some(sid) = supplier_id {
            format!("?supplier_id={}", sid)
        } else {
            String::new()
        };

        let url = format!("/ap-reports/aging{}", query_string);
        ApiService::get::<ApAgingResponse>(&url).await
    }
}
