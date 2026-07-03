use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 报表查询参数
#[derive(Debug, Deserialize)]
pub struct ArReportQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub customer_id: Option<i32>,
}

/// 获取统计报表
/// GET /api/v1/erp/ar/reports/statistics
// 批次 94 P2-7 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn get_statistics_report(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "AR 统计报表查询");
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let report = service
        .get_statistics_report(query.start_date, query.end_date, query.customer_id)
        .await
        .map_err(|e| AppError::internal(format!("获取统计报表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(report)))
}

/// 获取日报表
/// GET /api/v1/erp/ar/reports/daily
// 批次 94 P2-7 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn get_daily_report(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "AR 日报表查询");
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let report = service
        .get_daily_report(query.start_date, query.end_date, query.customer_id)
        .await
        .map_err(|e| AppError::internal(format!("获取日报表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(report)))
}

/// 获取月报表
/// GET /api/v1/erp/ar/reports/monthly
// 批次 94 P2-7 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn get_monthly_report(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "AR 月报表查询");
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let report = service
        .get_monthly_report(query.start_date, query.end_date, query.customer_id)
        .await
        .map_err(|e| AppError::internal(format!("获取月报表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(report)))
}

/// 获取账龄报表
/// GET /api/v1/erp/ar/reports/aging
// 批次 94 P2-7 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn get_aging_report(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "AR 账龄报表查询");
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let report = service
        .get_aging_report(query.customer_id)
        .await
        .map_err(|e| AppError::internal(format!("获取账龄报表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(report)))
}
