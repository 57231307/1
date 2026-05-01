use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use bingxi_backend::middleware::auth::auth_middleware;
use bingxi_backend::middleware::permission::permission_middleware;
use bingxi_backend::routes::create_router;
use bingxi_backend::services::auth_service::AuthService;
use bingxi_backend::utils::app_state::AppState;
use bingxi_backend::utils::cache::Cache;
use sea_orm::Database;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

async fn setup_app_auth_only() -> (Router, AppState) {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let state = AppState::new(Arc::new(db), "test_secret".to_string());
    let app = create_router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        auth_middleware,
    ));
    (app, state)
}

async fn setup_app_auth_and_permission() -> (Router, AppState) {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let state = AppState::new(Arc::new(db), "test_secret".to_string());
    let app = create_router(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));
    (app, state)
}

#[tokio::test]
async fn test_dashboard_requires_authentication() {
    let (app, _state) = setup_app_auth_only().await;
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/dashboard/overview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_rejects_blacklisted_token() {
    let (app, state) = setup_app_auth_only().await;
    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());
    let token = auth_service.generate_token(42, "test_user", Some(2)).unwrap();

    state
        .cache
        .get_token_blacklist()
        .set(token.clone(), true, Some(Duration::from_secs(3600)));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/erp/auth/refresh")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_permission_no_user_id_1_bypass_without_role() {
    let (app, state) = setup_app_auth_and_permission().await;
    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());
    let token = auth_service.generate_token(1, "admin", None).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/users")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
