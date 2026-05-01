use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use bingxi_backend::middleware::request_validator::request_validator_middleware;
use tower::ServiceExt;

async fn ok_handler() -> &'static str {
    "ok"
}

fn build_app() -> Router {
    Router::new()
        .route("/api/v1/erp/auth/login", get(ok_handler))
        .route("/api/v1/erp/product/1", get(ok_handler))
        .layer(middleware::from_fn(request_validator_middleware))
}

#[tokio::test]
async fn test_request_validator_public_path_bypass() {
    let app = build_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/auth/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_request_validator_forbids_without_header() {
    let app = build_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/product/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_request_validator_allows_with_xml_http_request_header() {
    let app = build_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/product/1")
                .header("X-Requested-With", "XMLHttpRequest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

