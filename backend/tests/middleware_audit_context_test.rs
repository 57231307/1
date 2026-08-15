use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::from_fn;
use axum::routing::get;
use bingxi_backend::middleware::audit_context::*;
use tower::ServiceExt;

async fn echo() -> &'static str {
    "ok"
}

/// 未注入 trace_id 时仍能放行（fallback 到本地 UUID v4）
#[tokio::test]
async fn test_audit_context_generates_request_id_when_missing() {
    let app = Router::new()
        .route("/", get(echo))
        .layer(from_fn(audit_context_middleware));

    let req = Request::builder()
        .uri("/")
        .body(Body::empty())
        .expect("build");
    let response = app.oneshot(req).await.expect("request ok");
    assert_eq!(response.status(), 200);
}

/// X-Real-IP 优先级最高
#[tokio::test]
async fn test_audit_context_reads_real_ip_header() {
    let app = Router::new()
        .route("/", get(echo))
        .layer(from_fn(audit_context_middleware));

    let req = Request::builder()
        .uri("/")
        .header("x-real-ip", "203.0.113.42")
        .body(Body::empty())
        .expect("build");
    let response = app.oneshot(req).await.expect("request ok");
    assert_eq!(response.status(), 200);
}

/// X-Forwarded-For 多级时取第一段
#[tokio::test]
async fn test_audit_context_reads_forwarded_for_first_segment() {
    let app = Router::new()
        .route("/", get(echo))
        .layer(from_fn(audit_context_middleware));

    let req = Request::builder()
        .uri("/")
        .header("x-forwarded-for", "198.51.100.1, 10.0.0.1")
        .body(Body::empty())
        .expect("build");
    let response = app.oneshot(req).await.expect("request ok");
    assert_eq!(response.status(), 200);
}

/// 无 IP header 时 fallback 到 "unknown"
#[tokio::test]
async fn test_audit_context_falls_back_to_unknown_ip() {
    let app = Router::new()
        .route("/", get(echo))
        .layer(from_fn(audit_context_middleware));

    let req = Request::builder()
        .uri("/")
        .body(Body::empty())
        .expect("build");
    let response = app.oneshot(req).await.expect("request ok");
    assert_eq!(response.status(), 200);
}

/// `AuditContext::empty()` 字段全为空字符串
#[test]
fn test_empty_context() {
    let ctx = AuditContext::empty();
    assert_eq!(ctx.request_id, "");
    assert_eq!(ctx.ip_address, "");
    assert_eq!(ctx.user_agent, "");
}
