use chrono::Utc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::{Instant, Duration};
use tracing::{info, warn, error};

/// 请求日志中间件
/// 记录每个请求的详细信息
#[allow(dead_code)]
pub async fn request_logger_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start_time = Instant::now();

    // 记录请求开始
    info!(
        method = %method,
        path = %uri.path(),
        query = ?uri.query(),
        timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        "收到请求"
    );

    // 执行请求
    let response = next.run(req).await;

    // 计算耗时
    let duration = start_time.elapsed();
    let status = response.status();

    // 根据状态码记录日志
    if status.is_success() {
        info!(
            method = %method,
            path = %uri.path(),
            status = %status,
            duration_ms = duration.as_millis(),
            "请求完成"
        );
    } else if status.is_client_error() {
        warn!(
            method = %method,
            path = %uri.path(),
            status = %status,
            duration_ms = duration.as_millis(),
            "客户端错误"
        );
    } else if status.is_server_error() {
        error!(
            method = %method,
            path = %uri.path(),
            status = %status,
            duration_ms = duration.as_millis(),
            "服务器错误"
        );
    }

    Ok(response)
}

/// 慢请求检测中间件
/// 检测并记录超过阈值的慢请求
#[allow(dead_code)]
pub async fn slow_request_detector_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Instant::now();
    let slow_threshold = Duration::from_millis(1000); // 1 秒阈值

    let response = next.run(req).await;

    let duration = start_time.elapsed();
    if duration > slow_threshold {
        warn!(
            method = %response.extensions().get::<axum::http::Method>().unwrap_or(&axum::http::Method::GET),
            path = %response.extensions().get::<axum::http::Uri>().map(|u| u.path()).unwrap_or("/"),
            status = %response.status(),
            duration_ms = duration.as_millis(),
            threshold_ms = slow_threshold.as_millis(),
            "检测到慢请求"
        );
    }

    Ok(response)
}

/// 性能监控中间件
/// 收集请求性能指标
#[allow(dead_code)]
pub async fn performance_monitor_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Instant::now();
    let path = req.uri().path().to_string();

    let response = next.run(req).await;
    let duration = start_time.elapsed();
    let status = response.status().as_u16();

    // 记录性能指标（可以集成到监控系统）
    // 这里只是示例，实际使用可以集成 Prometheus 等监控工具
    info!(
        target: "performance_metrics",
        path = path,
        status = status,
        duration_ms = duration.as_millis(),
        timestamp = %Utc::now().timestamp(),
        "性能指标"
    );

    Ok(response)
}

/// 请求 ID 中间件
/// 为每个请求生成唯一 ID，便于追踪
#[allow(dead_code)]
pub async fn request_id_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 生成请求 ID
    let request_id = uuid::Uuid::new_v4().to_string();

    // 注入到请求头
    req.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    let mut response = next.run(req).await;

    // 在响应中也添加请求 ID
    response.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    Ok(response)
}
