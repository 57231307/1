//! 应付报表 Handler
//!
//! 应付报表 HTTP 接口层，负责处理报表相关的 HTTP 请求

use crate::middleware::auth_context::AuthContext;
use crate::services::ap_report_service::ApReportService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use std::time::Duration;
use tracing::info;

/// 报表缓存 TTL（60 秒，平衡新鲜度与数据库负载）
const REPORT_CACHE_TTL: Duration = Duration::from_secs(60);

/// 查询统计报表参数
#[derive(Debug, Deserialize)]
pub struct ApStatisticsQueryParams {
    pub supplier_id: Option<i32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// 获取应付统计报表
// 批次 248 v14 中风险修复：接入 CacheService 缓存报表结果
pub async fn get_statistics_report(
    Query(params): Query<ApStatisticsQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询应付统计报表，供应商 ID: {:?}, 日期范围：{} 至 {}",
        auth.username, params.supplier_id, params.start_date, params.end_date
    );

    let cache_key = format!(
        "ap:report:statistics:{:?}:{}:{}",
        params.supplier_id, params.start_date, params.end_date
    );
    if let Some(cached) = state.cache_service.get(&cache_key).await {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&cached) {
            return Ok(Json(ApiResponse::success(value)));
        }
    }

    let service = ApReportService::new(state.db.clone());
    let report = service
        .get_statistics_report(params.supplier_id, params.start_date, params.end_date)
        .await?;

    let value = serde_json::to_value(&report)?;
    if let Ok(bytes) = serde_json::to_vec(&value) {
        state
            .cache_service
            .set_with_ttl(cache_key, bytes, REPORT_CACHE_TTL)
            .await;
    }

    info!("用户 {} 查询应付统计报表成功", auth.username);

    Ok(Json(ApiResponse::success(value)))
}

/// 查询日报参数
#[derive(Debug, Deserialize)]
pub struct ApDailyQueryParams {
    pub supplier_id: Option<i32>,
    pub report_date: NaiveDate,
}

/// 获取应付日报
// 批次 248 v14 中风险修复：接入 CacheService 缓存报表结果
pub async fn get_daily_report(
    Query(params): Query<ApDailyQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询应付日报，日期：{}, 供应商 ID: {:?}",
        auth.username, params.report_date, params.supplier_id
    );

    let cache_key = format!(
        "ap:report:daily:{:?}:{}",
        params.supplier_id, params.report_date
    );
    if let Some(cached) = state.cache_service.get(&cache_key).await {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&cached) {
            return Ok(Json(ApiResponse::success(value)));
        }
    }

    let service = ApReportService::new(state.db.clone());
    let report = service
        .get_daily_report(params.report_date, params.supplier_id)
        .await?;

    let value = serde_json::to_value(&report)?;
    if let Ok(bytes) = serde_json::to_vec(&value) {
        state
            .cache_service
            .set_with_ttl(cache_key, bytes, REPORT_CACHE_TTL)
            .await;
    }

    info!("用户 {} 查询应付日报成功", auth.username);

    Ok(Json(ApiResponse::success(value)))
}

/// 查询月报参数
#[derive(Debug, Deserialize)]
pub struct ApMonthlyQueryParams {
    pub supplier_id: Option<i32>,
    pub year: i32,
    pub month: u32,
}

/// 获取应付月报
// 批次 248 v14 中风险修复：接入 CacheService 缓存报表结果
pub async fn get_monthly_report(
    Query(params): Query<ApMonthlyQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询应付月报，年份：{}, 月份：{}, 供应商 ID: {:?}",
        auth.username, params.year, params.month, params.supplier_id
    );

    let cache_key = format!(
        "ap:report:monthly:{:?}:{}:{}",
        params.supplier_id, params.year, params.month
    );
    if let Some(cached) = state.cache_service.get(&cache_key).await {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&cached) {
            return Ok(Json(ApiResponse::success(value)));
        }
    }

    let service = ApReportService::new(state.db.clone());
    let report = service
        .get_monthly_report(params.year, params.month, params.supplier_id)
        .await?;

    let value = serde_json::to_value(&report)?;
    if let Ok(bytes) = serde_json::to_vec(&value) {
        state
            .cache_service
            .set_with_ttl(cache_key, bytes, REPORT_CACHE_TTL)
            .await;
    }

    info!("用户 {} 查询应付月报成功", auth.username);

    Ok(Json(ApiResponse::success(value)))
}

/// 查询账龄分析参数
#[derive(Debug, Deserialize)]
pub struct ApAgingQueryParams {
    pub supplier_id: Option<i32>,
}

/// 获取账龄分析报告
// 批次 248 v14 中风险修复：接入 CacheService 缓存报表结果
pub async fn get_aging_report(
    Query(params): Query<ApAgingQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询账龄分析报告，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let cache_key = format!("ap:report:aging:{:?}", params.supplier_id);
    if let Some(cached) = state.cache_service.get(&cache_key).await {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&cached) {
            return Ok(Json(ApiResponse::success(value)));
        }
    }

    let service = ApReportService::new(state.db.clone());
    let report = service.get_aging_report(params.supplier_id).await?;

    let value = serde_json::to_value(&report)?;
    if let Ok(bytes) = serde_json::to_vec(&value) {
        state
            .cache_service
            .set_with_ttl(cache_key, bytes, REPORT_CACHE_TTL)
            .await;
    }

    info!("用户 {} 查询账龄分析报告成功", auth.username);

    Ok(Json(ApiResponse::success(value)))
}
