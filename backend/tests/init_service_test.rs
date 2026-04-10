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
    let db = Database::connect("sqlite::memory:").await.unwrap();
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
    let hash = hash_result.unwrap();
    assert!(!hash.is_empty());
}

#[test]
fn test_migration_order_no_sales_orders_fk_before_001_init() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("database/migration");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let init_index = entries
        .iter()
        .position(|e| e.file_name() == std::ffi::OsString::from("001_init.sql"))
        .unwrap();

    for entry in entries.iter().take(init_index) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }
        let sql = std::fs::read_to_string(&path).unwrap();
        assert!(
            !sql.contains("REFERENCES sales_orders"),
            "迁移脚本 {:?} 在 001_init.sql 之前引用了 sales_orders",
            path.file_name().unwrap()
        );
    }
}
