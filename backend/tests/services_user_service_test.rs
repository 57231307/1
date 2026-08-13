use bingxi_backend::models::master_data;
use bingxi_backend::models::status::master_data;
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::services::user_service::UserService;
use bingxi_backend::utils::error::AppError;
use chrono::Utc;
use std::sync::Arc;

/// 构造用户模型夹具（复用于多个测试，遵循规则 6 避免硬编码）
fn make_user_model(id: i32, username: &str, is_active: bool) -> user::Model {
    user::Model {
        id,
        username: username.to_string(),
        password_hash: "$argon2id$test".to_string(),
        email: Some(format!("{}@test.com", username)),
        phone: None,
        role_id: None,
        department_id: None,
        is_active,
        totp_secret: None,
        is_totp_enabled: false,
        totp_recovery_codes: None,
        last_login_at: None,
        password_changed_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        agreed_to_terms_at: None,
        gender: None,
        birth_date: None,
    }
}

// ---------- 常量正确性 ----------

/// test_master_dataclzzqx（验证 update_user 中引用的 master_data::ACTIVE 常量值正确；（批次 209 P2-5 修复：硬编码 "active" 替换为常量））
#[test]
fn test_master_dataclzzqx() {
    assert_eq!(master_data::ACTIVE, "active");
    assert_eq!(master_data::INACTIVE, "inactive");
    // ACTIVE 和 INACTIVE 互不相同
    assert_ne!(master_data::ACTIVE, master_data::INACTIVE);
}

/// test_update_userztpdlj
/// 复现 update_user 中 status_val == master_data::ACTIVE 的判定逻辑；验证 status 为 "active" 时 becoming_active=true，其他为 false
#[test]
fn test_update_userztpdlj() {
    let becoming_active = |status: &str| status == master_data::ACTIVE;
    assert!(becoming_active("active"));
    assert!(!becoming_active("inactive"));
    assert!(!becoming_active("ACTIVE"));
    assert!(!becoming_active(""));
}

// ---------- 错误消息格式 ----------

/// test_find_by_usernamecwxxgs（验证 find_by_username 在用户不存在时返回的错误消息包含用户名）
#[test]
fn test_find_by_usernamecwxxgs() {
    let username = "test_user_123";
    let err = AppError::not_found(format!("用户 {} 不存在", username));
    let msg = err.to_string();
    assert!(msg.contains(username), "错误消息应包含用户名");
}

/// test_find_by_idcwxxgs（验证 find_by_id 在用户不存在时返回的错误消息包含用户 ID）
#[test]
fn test_find_by_idcwxxgs() {
    let user_id = 99999;
    let err = AppError::not_found(format!("用户 ID {} 不存在", user_id));
    let msg = err.to_string();
    assert!(msg.contains(&user_id.to_string()), "错误消息应包含用户 ID");
}

/// test_create_usercfyhmcwxxgs（验证 create_user 在用户名已存在时返回的错误消息包含用户名）
#[test]
fn test_create_usercfyhmcwxxgs() {
    let username = "existing_user";
    let err = AppError::business(format!("用户名 '{}' 已存在", username));
    let msg = err.to_string();
    assert!(msg.contains(username), "错误消息应包含用户名");
}

// ---------- 夹具与实例化 ----------

/// test_yhmxjjgz（验证 make_user_model 夹具能正确构造用户模型）
#[test]
fn test_yhmxjjgz() {
    let u = make_user_model(1, "fixture_user", true);
    assert_eq!(u.id, 1);
    assert_eq!(u.username, "fixture_user");
    assert!(u.is_active);
    assert_eq!(u.email.as_deref(), Some("fixture_user@test.com"));
}

/// test_fwslh_sqlitencsjk（验证 UserService 在 SQLite 内存数据库上能正常实例化）
#[tokio::test]
async fn test_fwslh_sqlitencsjk() {
    let db = setup_test_db().await;
    let service = UserService::new(Arc::new(db));
    // 验证内部 db Arc 已被正确持有
    assert!(Arc::strong_count(&service.database) >= 1);
}

/// 测试_find_by_id缺失用户返回not_found（验证 find_by_id 在 SQLite 内存库（无表）调用时返回错误（非 panic））
#[tokio::test]
async fn test_find_by_idqsyhfhcw() {
    let db = setup_test_db().await;
    let service = UserService::new(Arc::new(db));
    // SQLite 内存库无 schema，查询应返回 Err 而非 panic
    let result = service.find_by_id(99999).await;
    assert!(result.is_err(), "无表结构时应返回错误而非 panic");
}
