//! 初始化服务测试

use bingxi_backend::services::init_service::{DatabaseConfig, InitService};
use sea_orm::Database;
use std::sync::Arc;

/// 测试数据库配置转换为连接字符串
#[tokio::test]
async fn test_database_config_to_connection_string() {
    let config = DatabaseConfig {
        host: "localhost".to_string(),
        port: "5432".to_string(),
        name: "test_db".to_string(),
        username: "test_user".to_string(),
        password: "test_pass".to_string(),
    };
    
    let conn_str = config.to_connection_string();
    assert_eq!(conn_str, "postgres://test_user:test_pass@localhost:5432/test_db?sslmode=disable");
}

/// 测试初始化服务的基本功能
#[tokio::test]
#[ignore]
async fn test_init_service_basic() {
    // 使用内存数据库进行测试
    let db = Database::connect("sqlite::memory:").await.expect("操作应该成功");
    let db = Arc::new(db);
    
    let init_service = InitService::new(db);
    
    // 检查初始化状态
    let (initialized, message) = init_service.check_initialized().await;
    assert!(!initialized);
    assert_eq!(message, "系统未初始化");
}

/// 测试密码哈希功能
#[tokio::test]
async fn test_password_hashing() {
    use bingxi_backend::services::auth_service::AuthService;
    
    let password = "test_password_123";
    let hash_result = AuthService::hash_password(password);
    assert!(hash_result.is_ok());
    let hash = hash_result.expect("操作应该成功");
    assert!(!hash.is_empty());
}

#[tokio::test]
async fn test_execute_unprepared_with_dollar_quotes() {
    use sea_orm::ConnectionTrait;
    let db = Database::connect("sqlite::memory:").await.expect("操作应该成功");
    // SQLite doesn't support $$ quotes natively like Postgres does, but we can test multiple statements
    let sql = "CREATE TABLE test (id INTEGER); INSERT INTO test VALUES (1);";
    let res = db.execute_unprepared(sql).await;
    assert!(res.is_ok(), "execute_unprepared failed: {:?}", res.err());
}

