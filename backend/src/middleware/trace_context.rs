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
const X_TRACE_ID_HEADER: &str = "x-trace-id";

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

    // 5. 把 trace_id 写入响应头（X-Trace-Id）
    if let Ok(v) = HeaderValue::from_str(&ctx.trace_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(X_TRACE_ID_HEADER), v);
    }

    let elapsed_ms = start.elapsed().as_millis();
    tracing::info!(
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        method = %method,
        path = %uri_path,
        status = %response.status(),
        elapsed_ms = %elapsed_ms,
        "trace.complete"
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt; // for oneshot()

    async fn hello() -> &'static str {
        "world"
    }

    #[tokio::test]
    async fn test_middleware_generates_trace_id_when_missing() {
        let app = Router::new()
            .route("/", get(hello))
            .layer(from_fn(trace_context_middleware));

        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("build request");
        let response = app.oneshot(req).await.expect("request should succeed");

        assert_eq!(response.status(), 200);
        assert!(response.headers().contains_key(X_TRACE_ID_HEADER));
        let trace_id = response
            .headers()
            .get(X_TRACE_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(trace_id.len(), 32);
    }

    #[tokio::test]
    async fn test_middleware_propagates_traceparent() {
        let app = Router::new()
            .route("/", get(hello))
            .layer(from_fn(trace_context_middleware));

        let req = Request::builder()
            .uri("/")
            .header(
                "traceparent",
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            )
            .body(Body::empty())
            .expect("build request");
        let response = app.oneshot(req).await.expect("request should succeed");

        assert_eq!(response.status(), 200);
        let trace_id = response
            .headers()
            .get(X_TRACE_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap();
        // 透传客户端的 trace_id
        assert_eq!(trace_id, "0af7651916cd43dd8448eb211c80319c");
    }

    #[tokio::test]
    async fn test_middleware_handles_invalid_traceparent() {
        let app = Router::new()
            .route("/", get(hello))
            .layer(from_fn(trace_context_middleware));

        let req = Request::builder()
            .uri("/")
            .header("traceparent", "garbage")
            .body(Body::empty())
            .expect("build request");
        let response = app.oneshot(req).await.expect("request should succeed");

        assert_eq!(response.status(), 200);
        // 无效 header 时 fallback 到新 trace_id
        let trace_id = response
            .headers()
            .get(X_TRACE_ID_HEADER)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(trace_id.len(), 32);
    }
}
