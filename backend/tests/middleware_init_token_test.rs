use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
};
use bingxi_backend::middleware::auth_context::*;
use bingxi_backend::middleware::init_token::INIT_TOKEN_ENV;
use bingxi_backend::middleware::init_token::INIT_TOKEN_HEADER;
use bingxi_backend::middleware::init_token::init_token_middleware;
use bingxi_backend::services::cache_service::*;
use tower::ServiceExt;

/// 创建一个最小化测试 Router
fn build_test_app() -> Router {
    async fn handler() -> &'static str {
        "ok"
    }
    Router::new()
        .route("/test", get(handler))
        .layer(middleware::from_fn(init_token_middleware))
}

/// 场景 A：未设置 INIT_TOKEN 环境变量 → 期望 401
#[tokio::test]
async fn test_init_token_missing_env() {
    // 确保环境变量未设置
    unsafe {
        std::env::remove_var(INIT_TOKEN_ENV);
    }

    let app = build_test_app();
    let req = Request::builder()
        .uri("/test")
        .header(INIT_TOKEN_HEADER, "any-token")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// 场景 B：未提供 X-Init-Token 头 → 期望 401
#[tokio::test]
async fn test_init_token_missing_header() {
    unsafe {
        std::env::set_var(INIT_TOKEN_ENV, "test-secret-token");
    }

    let app = build_test_app();
    let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    unsafe {
        std::env::remove_var(INIT_TOKEN_ENV);
    }
}

/// 场景 C：提供错误的 X-Init-Token → 期望 401
#[tokio::test]
async fn test_init_token_wrong() {
    unsafe {
        std::env::set_var(INIT_TOKEN_ENV, "correct-token");
    }

    let app = build_test_app();
    let req = Request::builder()
        .uri("/test")
        .header(INIT_TOKEN_HEADER, "wrong-token")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    unsafe {
        std::env::remove_var(INIT_TOKEN_ENV);
    }
}

/// 场景 D：提供正确的 X-Init-Token → 期望 200
#[tokio::test]
async fn test_init_token_correct() {
    unsafe {
        std::env::set_var(INIT_TOKEN_ENV, "correct-token-abc");
    }

    let app = build_test_app();
    let req = Request::builder()
        .uri("/test")
        .header(INIT_TOKEN_HEADER, "correct-token-abc")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    unsafe {
        std::env::remove_var(INIT_TOKEN_ENV);
    }
}

/// 场景 E：INIT_TOKEN 配置为空字符串 → 期望 401（fail-secure）
#[tokio::test]
async fn test_init_token_empty_env() {
    unsafe {
        std::env::set_var(INIT_TOKEN_ENV, "");
    }

    let app = build_test_app();
    let req = Request::builder()
        .uri("/test")
        .header(INIT_TOKEN_HEADER, "any-token")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    unsafe {
        std::env::remove_var(INIT_TOKEN_ENV);
    }
}
