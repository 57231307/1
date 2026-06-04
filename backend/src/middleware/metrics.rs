//! 监控中间件
//!
//! 自动记录请求指标，包括：
//! - 基础指标：`http_requests_total` / `http_request_duration_seconds` / `errors_total`
//! - 带标签指标（P3.2 新增）：`http_requests_by_route` / `http_request_duration_by_route`
//!
//! ## 使用方法
//!
//! ```rust,ignore
//! use crate::middleware::metrics::metrics_middleware;
//! use std::sync::Arc;
//! use crate::services::metrics_service::MetricsService;
//!
//! // main.rs 中挂载：
//! .layer(axum::middleware::from_fn_with_state(
//!     state.metrics.clone(),
//!     metrics_middleware,
//! ))
//! ```
//!
//! 注：当前 main.rs 已使用 `TraceLayer` 输出结构化日志，本中间件在挂载后会同时
//! 维护 Prometheus 指标。如需关闭可注释掉相关 `.layer()`。

use crate::services::metrics_service::{Metrics, MetricsService};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

/// 监控中间件
///
/// 自动按 method / route / status 记录带标签指标 + 基础指标。
pub async fn metrics_middleware(
    State(metrics_service): State<Arc<MetricsService>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();

    // 方法 + 路径（路径用 URI 的 path 部分，过长时截断到 128 字符避免 label cardinality 爆炸）
    let method = request.method().as_str().to_string();
    let raw_path = request.uri().path().to_string();
    let route = truncate_route(&raw_path);

    // 开始请求
    metrics_service.metrics.start_request();

    // 执行请求
    let response = next.run(request).await;

    // 结束请求
    metrics_service.metrics.end_request();

    // 记录耗时
    let duration = start.elapsed();
    let duration_secs = duration.as_secs_f64();

    // 基础指标
    metrics_service.metrics.record_http_request(duration_secs);

    // 带标签指标（P3.2 新增）
    let status = response.status();
    metrics_service
        .metrics
        .record_http_by_route(&method, &route, status);
    metrics_service
        .metrics
        .record_http_duration_by_route(&method, &route, duration_secs);

    // 错误计数
    if status.is_server_error() {
        metrics_service.metrics.record_error();
    }

    Ok(response)
}

/// 截断过长的 route 标签，避免 Prometheus cardinality 爆炸
///
/// PromQL 标签值过长会导致指标存储膨胀。统一截断到 128 字符，
/// 超长部分用 `*_truncated_<hash>` 标记。
fn truncate_route(path: &str) -> String {
    const MAX_LEN: usize = 128;
    if path.len() <= MAX_LEN {
        return path.to_string();
    }

    // 简单 hash：取末尾 8 字符的 ASCII 总和（避免引入额外依赖）
    let hash: u32 = path.bytes().map(|b| b as u32).sum::<u32>() % 0xFFFF;
    let prefix = &path[..MAX_LEN];
    format!("{}_trunc_{:04x}", prefix, hash)
}

/// 数据库查询监控包装器
///
/// 在 Drop 时自动记录 `db_query_duration_seconds`。
pub struct DbQueryMonitor<'a> {
    start: Instant,
    metrics: &'a Metrics,
}

impl<'a> DbQueryMonitor<'a> {
    pub fn new(metrics: &'a Metrics) -> Self {
        Self {
            start: Instant::now(),
            metrics,
        }
    }
}

impl<'a> Drop for DbQueryMonitor<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.metrics.record_db_query(duration.as_secs_f64());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_route_short() {
        assert_eq!(truncate_route("/api/v1/erp/users"), "/api/v1/erp/users");
    }

    #[test]
    fn test_truncate_route_long() {
        let long_path = "/".to_string() + &"a".repeat(200);
        let truncated = truncate_route(&long_path);
        assert!(truncated.len() <= 128 + 32);
        assert!(truncated.contains("_trunc_"));
    }
}
