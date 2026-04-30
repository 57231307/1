//! 监控中间件
//! 自动记录请求指标

use crate::services::metrics_service::MetricsService;
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
pub async fn metrics_middleware(
    State(metrics_service): State<Arc<MetricsService>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();

    // 开始请求
    metrics_service.metrics.start_request();

    // 执行请求
    let response = next.run(request).await;

    // 结束请求
    metrics_service.metrics.end_request();

    // 记录请求耗时
    let duration = start.elapsed();
    metrics_service
        .metrics
        .record_http_request(duration.as_secs_f64());

    // 记录错误
    if response.status().is_server_error() {
        metrics_service.metrics.record_error();
    }

    Ok(response)
}

/// 数据库查询监控包装器
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

use crate::services::metrics_service::Metrics;
