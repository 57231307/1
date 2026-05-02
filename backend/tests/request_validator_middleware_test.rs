use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    routing::get,
    Router,
};
use bingxi_backend::middleware::request_validator::request_validator_middleware;
use tower::ServiceExt;

#[tokio::test]
async fn test_request_validator_allows_public_paths_without_header() {
    let app = Router::new()
        .route("/api/v1/erp/health", get(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn(request_validator_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_request_validator_blocks_non_public_paths_without_header() {
    let app = Router::new()
        .route("/api/v1/erp/users", get(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn(request_validator_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_request_validator_allows_non_public_paths_with_ajax_header() {
    let app = Router::new()
        .route("/api/v1/erp/users", get(|| async { StatusCode::OK }))
        .layer(axum::middleware::from_fn(request_validator_middleware));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/users")
                .header("X-Requested-With", "XMLHttpRequest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

