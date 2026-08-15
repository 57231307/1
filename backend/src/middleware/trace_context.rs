//! 分布式追踪上下文中间件
//!
//! 职责：
//! 1. 从请求的 `traceparent` header 解析或生成新的 `TraceContext`
//! 2. 把 `TraceContext` 存入 `Request::extensions()` 供 handler / service 读取
//! 3. 创建 root `tracing::Span`，把 trace_id / span_id 等写入 span 字段
//! 4. 在响应头回写 `X-Trace-Id`，便于客户端关联日志
//!
//! 注：handler 主要通过 `Request::extensions()` 取出 ctx。

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use std::time::Instant;

use crate::observability::span::root_span;
use crate::observability::trace_context::extract_or_new;

/// 用于在响应头回写 `X-Trace-Id`，方便客户端日志关联
pub const X_TRACE_ID_HEADER: &str = "x-trace-id";

/// V15 P2 20.1-C：tail-based sampling 慢请求阈值（毫秒）
/// 超过此阈值的请求强制采样（100%），可通过环境变量 `OTEL_SLOW_REQUEST_MS` 配置。
fn slow_request_threshold_ms() -> u64 {
    use std::sync::LazyLock;
    static THRESHOLD: LazyLock<u64> = LazyLock::new(|| {
        std::env::var("OTEL_SLOW_REQUEST_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2000) // 默认 2s（与 P95 告警阈值对齐）
    });
    *THRESHOLD
}

/// 追踪上下文中间件
pub async fn trace_context_middleware(mut request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    // 1. 解析 / 生成 trace 上下文
    let traceparent = request
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok());
    let ctx = extract_or_new(traceparent);

    // 2. 把 ctx 放入 request extensions，供下游 handler/service 读取
    request.extensions_mut().insert(ctx.clone());

    // 3. 创建 root span 并在 span 内执行下游
    let method = request.method().clone();
    let uri_path = request.uri().path().to_string();
    let span = root_span(&ctx, method.as_str(), &uri_path);

    // 4. 在响应头写入 X-Trace-Id（即便 span 内出现 panic，也确保能回写）
    let _guard = span.enter();
    let mut response = next.run(request).await;

    // 5. V15 P2 20.1-C：tail-based sampling — 5xx / 慢请求强制采样
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let status = response.status();
    let is_5xx = status.is_server_error();
    let is_slow = elapsed_ms > slow_request_threshold_ms();

    if is_5xx || is_slow {
        // 强制采样：在响应头中标记 `X-Trace-Sampled: forced`
        // OTel Collector 可据此决定保留此 trace
        let v = HeaderValue::from_static("forced");
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-trace-sampled"), v);
        tracing::warn!(
            trace_id = %ctx.trace_id,
            span_id = %ctx.span_id,
            method = %method,
            path = %uri_path,
            status = %status,
            elapsed_ms = %elapsed_ms,
            is_5xx = is_5xx,
            is_slow = is_slow,
            "trace.tail_sampled"
        );
    }

    // 6. 把 trace_id 写入响应头（X-Trace-Id）
    if let Ok(v) = HeaderValue::from_str(&ctx.trace_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(X_TRACE_ID_HEADER), v);
    }

    tracing::info!(
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        method = %method,
        path = %uri_path,
        status = %status,
        elapsed_ms = %elapsed_ms,
        "trace.complete"
    );

    response
}
