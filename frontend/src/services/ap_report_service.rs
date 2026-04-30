//! 应付报表服务
//!
//! 与后端应付报表API交互

use crate::models::ap_report::{
    ApAgingResponse, ApDailyResponse, ApMonthlyResponse, ApStatisticsResponse,
};
use crate::services::api::ApiService;

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
