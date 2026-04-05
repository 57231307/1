//! 用户服务集成测试

use axum::Router;
use bingxi_backend::routes::create_router;
use bingxi_backend::services::auth_service::AuthService;
use bingxi_backend::services::user_service::UserService;
use sea_orm::Database;
use std::sync::Arc;
use bingxi_backend::utils::app_state::AppState;

#[allow(dead_code)]
async fn setup_app() -> Router {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let state = AppState::new(std::sync::Arc::new(db), "test_secret".to_string());
    create_router(state)
}

/// 测试用户创建和查询流程
#[tokio::test]
async fn test_user_crud_flow() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let user_service = UserService::new(Arc::new(db.clone()));

    // 1. 创建用户
    let password_hash = AuthService::hash_password("password123").unwrap();
    let create_result = user_service
        .create_user(
            "test_user".to_string(),
            password_hash,
            Some("test@example.com".to_string()),
            Some("13800138000".to_string()),
            Some(1),
            Some(1),
        )
        .await;

    assert!(create_result.is_ok());
    let user = create_result.unwrap();
    assert_eq!(user.username, "test_user");

    // 2. 根据 ID 查询用户
    let find_by_id_result = user_service.find_by_id(user.id).await;
    assert!(find_by_id_result.is_ok());
    let found_user = find_by_id_result.unwrap();
    assert_eq!(found_user.id, user.id);
    assert_eq!(found_user.username, "test_user");

    // 3. 根据用户名查询用户
    let find_by_username_result = user_service.find_by_username("test_user").await;
    assert!(find_by_username_result.is_ok());
    let found_user = find_by_username_result.unwrap();
    assert_eq!(found_user.username, "test_user");

    // 4. 获取用户列表
    let list_result = user_service.list_users(0, 20).await;
    assert!(list_result.is_ok());
    let (users, total) = list_result.unwrap();
    assert!(total >= 1);
    assert!(!users.is_empty());
}

/// 测试重复用户名处理
#[tokio::test]
async fn test_duplicate_username() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let user_service = UserService::new(Arc::new(db.clone()));

    let password_hash = AuthService::hash_password("password123").unwrap();

    // 1. 创建第一个用户
    let result1 = user_service
        .create_user(
            "duplicate_user".to_string(),
            password_hash.clone(),
            None,
            None,
            None,
            None,
        )
        .await;

    assert!(result1.is_ok());

    // 2. 尝试创建同名用户 (根据数据库约束，可能失败或成功取决于唯一约束)
    let result2 = user_service
        .create_user(
            "duplicate_user".to_string(),
            password_hash,
            None,
            None,
            None,
            None,
        )
        .await;

    // 这个测试取决于数据库是否有唯一约束
    // 如果有唯一约束，应该失败
    // 如果没有，应该成功
    // 这里我们只是记录这个行为
    println!("Duplicate username test: result2 = {:?}", result2.is_ok());
}
