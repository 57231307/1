//! 主备隔离 HTTP 端点
//!
//! 提供 4 个端点：
//! - `GET /api/v1/erp/admin/failover/status` — 主备实时状态
//! - `GET /api/v1/erp/admin/failover/metrics` — Prometheus 指标
//! - `POST /api/v1/erp/admin/failover/test/switch` — 手动触发切换
//! - `GET /api/v1/erp/admin/failover/health` — 健康检查

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing::warn;

use crate::services::failover_service::{FailoverMetrics, FailoverService};
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

/// 全局 failover metrics 单例
static GLOBAL_METRICS: OnceCell<Arc<FailoverMetrics>> = OnceCell::new();

/// 主备隔离状态查询响应
#[derive(Debug, serde::Serialize)]
pub struct StatusResponse {
    pub statuses: Vec<crate::models::failover_status::FailoverStatusDto>,
    pub events: Vec<crate::models::failover_event::FailoverEventDto>,
}

/// 获取主备状态
pub async fn get_failover_status(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, (StatusCode, String)> {
    let service = build_service(&state).ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "failover service not initialized".to_string(),
        )
    })?;

    let statuses = service
        .get_statuses()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let events = service
        .get_recent_events(20)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(StatusResponse { statuses, events }))
}

/// 获取 Prometheus 指标
pub async fn get_failover_metrics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = build_service(&state).ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "failover service not initialized".to_string(),
        )
    })?;
    let text = service
        .export_metrics()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        text,
    ))
}

/// 手动切换请求
#[derive(Debug, Deserialize)]
pub struct SwitchRequest {
    /// 功能名：database / cache
    pub function: String,
}

/// 手动触发切换（仅管理员）
pub async fn post_test_switch(
    State(state): State<AppState>,
    Json(req): Json<SwitchRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, String)> {
    if req.function != "database" && req.function != "cache" {
        return Err((
            StatusCode::BAD_REQUEST,
            "function 必须是 database 或 cache".to_string(),
        ));
    }

    let service = build_service(&state).ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "failover service not initialized".to_string(),
        )
    })?;
    let message = service
        .test_switch(&req.function)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(ApiResponse::success(message)))
}

/// 健康检查
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, String)> {
    let service = build_service(&state).ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "failover service not initialized".to_string(),
        )
    })?;
    let health = service
        .health_check()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(ApiResponse::success(json!({
        "database": health.database,
        "cache": health.cache,
    }))))
}

/// 构建 FailoverService（从 AppState 推断）
///
/// 如果 AppState 中没有初始化 FailoverService，则返回 None 并记录警告。
/// FailoverService 是可选的（不影响主流程）。
fn build_service(state: &AppState) -> Option<FailoverService> {
    // 从环境变量加载配置
    let config = match crate::config::failover::FailoverConfig::load_from_env() {
        Ok(c) => c,
        Err(e) => {
            warn!("[failover] 配置加载失败，使用默认配置: {}", e);
            crate::config::failover::FailoverConfig::default_for_test()
        }
    };

    let metrics = get_global_metrics();
    Some(FailoverService::new(
        (*state.db).clone(),
        config,
        metrics,
    ))
}

/// 获取全局 metrics 单例
pub fn get_global_metrics() -> Arc<FailoverMetrics> {
    GLOBAL_METRICS
        .get_or_init(|| Arc::new(FailoverMetrics::new().unwrap_or_default()))
        .clone()
}

/// 初始化全局 metrics（在 main 中调用）
pub fn init_global_metrics() -> Arc<FailoverMetrics> {
    get_global_metrics()
}
