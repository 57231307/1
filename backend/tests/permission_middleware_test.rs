use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::permission_middleware;
use bingxi_backend::models::role_permission;
use bingxi_backend::utils::app_state::AppState;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseBackend, EntityTrait,
    QueryFilter, Set, Statement,
};
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
CREATE TABLE role_permissions (
  id INTEGER PRIMARY KEY,
  role_id INTEGER NOT NULL,
  resource_type TEXT NOT NULL,
  resource_id INTEGER NULL,
  action TEXT NOT NULL,
  allowed BOOLEAN NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
"#
        .to_string(),
    ))
    .await
    .unwrap();
    db
}

async fn ok_handler() -> &'static str {
    "ok"
}

fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/erp/health", get(ok_handler))
        .route("/api/v1/erp/product/1", get(ok_handler))
        .layer(middleware::from_fn_with_state(state, permission_middleware))
}

#[tokio::test]
async fn test_permission_middleware_public_path_bypass() {
    let db = setup_db().await;
    let state = AppState::new(Arc::new(db), "test_secret".to_string());
    let app = build_app(state);

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
async fn test_permission_middleware_missing_auth_context_returns_401() {
    let db = setup_db().await;
    let state = AppState::new(Arc::new(db), "test_secret".to_string());
    let app = build_app(state);

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

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_permission_middleware_super_user_bypass() {
    let db = setup_db().await;
    let state = AppState::new(Arc::new(db), "test_secret".to_string());
    let app = build_app(state);

    let mut request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/erp/product/1")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(AuthContext {
        user_id: 1,
        username: "admin".to_string(),
        role_id: None,
    });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_permission_middleware_forbidden_without_permission() {
    let db = setup_db().await;
    let state = AppState::new(Arc::new(db), "test_secret".to_string());
    let app = build_app(state);

    let mut request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/erp/product/1")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(AuthContext {
        user_id: 2,
        username: "user".to_string(),
        role_id: Some(2),
    });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_permission_middleware_allows_when_permission_exists() {
    let db = setup_db().await;
    let now = Utc::now();

    role_permission::ActiveModel {
        id: Set(1),
        role_id: Set(2),
        resource_type: Set("product".to_string()),
        resource_id: Set(None),
        action: Set("read".to_string()),
        allowed: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .unwrap();

    let state = AppState::new(Arc::new(db), "test_secret".to_string());
    let app = build_app(state);

    let mut request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/erp/product/1")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(AuthContext {
        user_id: 2,
        username: "user".to_string(),
        role_id: Some(2),
    });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_permission_middleware_matches_specific_resource_id_or_global() {
    let db = setup_db().await;
    let now = Utc::now();

    role_permission::ActiveModel {
        id: Set(1),
        role_id: Set(3),
        resource_type: Set("product".to_string()),
        resource_id: Set(Some(1)),
        action: Set("read".to_string()),
        allowed: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .unwrap();

    let found_specific = role_permission::Entity::find()
        .filter(role_permission::Column::RoleId.eq(3))
        .filter(role_permission::Column::ResourceType.eq("product"))
        .filter(role_permission::Column::ResourceId.eq(1))
        .one(&db)
        .await
        .unwrap();
    assert!(found_specific.is_some());

    let state = AppState::new(Arc::new(db), "test_secret".to_string());
    let app = build_app(state);

    let mut request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/erp/product/1")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(AuthContext {
        user_id: 3,
        username: "user".to_string(),
        role_id: Some(3),
    });

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

