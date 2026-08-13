use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::from_fn;
use axum::response::Response;
use axum::routing::get;
use bingxi_backend::middleware::auth_context::*;
use bingxi_backend::middleware::trace::X_TRACE_ID_HEADER;
use bingxi_backend::middleware::trace::trace_context_middleware;
use bingxi_backend::services::cache_service::*;
use tower::ServiceExt; // for oneshot()

async fn hello() -> &'static str {
    "world"
}

// P9-1: 测试夹具 helper，封装 X-Trace-Id header 解析
fn extract_trace_id(response: &Response) -> &str {
    // 显式 match 处理 header 缺失场景
    match response.headers().get(X_TRACE_ID_HEADER) {
        Some(v) => match v.to_str() {
            Ok(s) => s,
            Err(_) => panic!("P9-1: 测试夹具 X-Trace-Id 应为合法 ASCII"),
        },
        None => panic!("P9-1: 测试夹具 X-Trace-Id 缺失"),
    }
}

#[tokio::test]
async fn test_middleware_generates_trace_id_when_missing() {
    let app = Router::new()
        .route("/", get(hello))
        .layer(from_fn(trace_context_middleware));

    let req = Request::builder()
        .uri("/")
        .body(Body::empty())
        .expect("P9-1: 测试夹具 请求构建失败");
    let response = app.oneshot(req).await.expect("P9-1: 测试夹具 请求应成功");

    assert_eq!(response.status(), 200);
    assert!(response.headers().contains_key(X_TRACE_ID_HEADER));
    let trace_id = extract_trace_id(&response);
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
        .expect("P9-1: 测试夹具 请求构建失败");
    let response = app.oneshot(req).await.expect("P9-1: 测试夹具 请求应成功");

    assert_eq!(response.status(), 200);
    // P9-1: 用 helper 替代 .get(...).unwrap().to_str().unwrap()
    let trace_id = extract_trace_id(&response);
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
        .expect("P9-1: 测试夹具 请求构建失败");
    let response = app.oneshot(req).await.expect("P9-1: 测试夹具 请求应成功");

    assert_eq!(response.status(), 200);
    // 无效 header 时 fallback 到新 trace_id
    let trace_id = extract_trace_id(&response);
    assert_eq!(trace_id.len(), 32);
}
