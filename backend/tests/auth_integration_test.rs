//! 认证服务集成测试

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use bingxi_backend::routes::create_router;
use bingxi_backend::services::auth_service::AuthService;
use bingxi_backend::middleware::auth::auth_middleware;
use sea_orm::Database;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_app() -> Router {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let app_state = bingxi_backend::utils::app_state::AppState::new(Arc::new(db), "test_secret".to_string());
    create_router(app_state.clone())
        .layer(axum::middleware::from_fn_with_state(app_state, auth_middleware))
}

/// 测试完整的登录流程
#[tokio::test]
async fn test_complete_login_flow() {
    let app = setup_app().await;

    // 1. 尝试登录 (应该失败，因为用户不存在)
    let login_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/erp/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "username": "test_user",
                        "password": "test_password"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试密码哈希和验证
#[tokio::test]
async fn test_password_hash_and_verify() {
    let password = "test_password_123";

    // 哈希密码
    let hash_result = AuthService::hash_password(password);
    assert!(hash_result.is_ok());
    let hash = hash_result.unwrap();

    // 验证密码
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let auth_service = AuthService::new(Arc::new(db), "test_secret".to_string());

    let verify_result = auth_service.verify_password(password, &hash);
    assert!(verify_result);

    // 验证错误密码
    let wrong_verify_result = auth_service.verify_password("wrong_password", &hash);
    assert!(!wrong_verify_result);
}
