//! 主备隔离 HTTP 端点
//!
//! 提供 4 个端点：
//! - `GET /api/v1/erp/admin/failover/status` — 主备实时状态
//! - `GET /api/v1/erp/admin/failover/metrics` — Prometheus 指标
//! - `POST /api/v1/erp/admin/failover/test/switch` — 手动触发切换
//! - `GET /api/v1/erp/admin/failover/health` — 健康检查

use axum::{extract::State, Json};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing::warn;

use crate::services::failover_service::{FailoverMetrics, FailoverService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
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
) -> Result<Json<StatusResponse>, AppError> {
    let service = build_service(&state)?;

    let statuses = service
        .get_statuses()
        .await
        .map_err(AppError::internal)?;
    let events = service
        .get_recent_events(20)
        .await
        .map_err(AppError::internal)?;

    Ok(Json(StatusResponse { statuses, events }))
}

/// 获取 Prometheus 指标
///
/// 返回 text/plain 格式（Prometheus exposition format），非 JSON。
pub async fn get_failover_metrics(
    State(state): State<AppState>,
) -> Result<axum::response::Response, AppError> {
    let service = build_service(&state)?;
    let text = service.export_metrics().map_err(AppError::internal)?;
    let mut response = axum::response::Response::new(axum::body::Body::from(text));
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    Ok(response)
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
) -> Result<Json<ApiResponse<String>>, AppError> {
    if req.function != "database" && req.function != "cache" {
        return Err(AppError::bad_request("function 必须是 database 或 cache"));
    }

    let service = build_service(&state)?;
    let message = service
        .test_switch(&req.function)
        .await
        .map_err(AppError::internal)?;

    Ok(Json(ApiResponse::success(message)))
}

/// 健康检查
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = build_service(&state)?;
    let health = service.health_check().await.map_err(AppError::internal)?;
    Ok(Json(ApiResponse::success(json!({
        "database": health.database,
        "cache": health.cache,
    }))))
}

/// 构建 FailoverService（从 AppState 推断）
///
/// 如果配置加载失败则使用默认配置，仅在 AppState 未初始化时返回错误。
fn build_service(state: &AppState) -> Result<FailoverService, AppError> {
    // 从环境变量加载配置
    let config = match crate::config::failover::FailoverConfig::load_from_env() {
        Ok(c) => c,
        Err(e) => {
            warn!("[failover] 配置加载失败，使用默认配置: {}", e);
            crate::config::failover::FailoverConfig::default_for_test()
        }
    };

    let metrics = get_global_metrics();
    Ok(FailoverService::new(
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
