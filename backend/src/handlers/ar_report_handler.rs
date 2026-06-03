use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 报表查询参数
#[derive(Debug, Deserialize)]
pub struct ArReportQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub customer_id: Option<i32>,
    pub period: Option<String>,
}

/// 统计报表响应
#[derive(Debug, Serialize)]
pub struct StatisticsReportResponse {
    pub total_invoices: i64,
    pub total_amount: rust_decimal::Decimal,
    pub paid_amount: rust_decimal::Decimal,
    pub unpaid_amount: rust_decimal::Decimal,
    pub overdue_amount: rust_decimal::Decimal,
    pub collection_rate: f64,
}

/// 获取统计报表
/// GET /api/v1/erp/ar/reports/statistics
pub async fn get_statistics_report(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let report = service
        .get_statistics_report(query.start_date, query.end_date, query.customer_id)
        .await
        .map_err(|e| AppError::internal(format!("获取统计报表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(report)))
}

/// 获取日报表
/// GET /api/v1/erp/ar/reports/daily
pub async fn get_daily_report(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let report = service
        .get_daily_report(query.start_date, query.end_date, query.customer_id)
        .await
        .map_err(|e| AppError::internal(format!("获取日报表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(report)))
}

/// 获取月报表
/// GET /api/v1/erp/ar/reports/monthly
pub async fn get_monthly_report(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let report = service
        .get_monthly_report(query.start_date, query.end_date, query.customer_id)
        .await
        .map_err(|e| AppError::internal(format!("获取月报表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(report)))
}

/// 获取账龄报表
/// GET /api/v1/erp/ar/reports/aging
pub async fn get_aging_report(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let report = service
        .get_aging_report(query.customer_id)
        .await
        .map_err(|e| AppError::internal(format!("获取账龄报表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(report)))
}
