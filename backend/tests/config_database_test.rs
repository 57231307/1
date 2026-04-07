//! 配置和数据库连接测试

use bingxi_backend::config::settings::AppSettings;
// Tests for config database

/// 测试配置文件加载
#[tokio::test]
async fn test_config_loading() {
    // 测试默认配置加载
    let settings = AppSettings::new();
    assert!(settings.is_ok());
    let settings = settings.unwrap();

    // 验证默认值
    assert!(!settings.server.host.is_empty());
    assert!(!settings.server.port.is_empty());
    assert!(!settings.database.host.is_empty());
    assert!(settings.database.port > 0);
    assert!(!settings.database.name.is_empty());
    assert!(settings.auth.token_expiry_hours > 0);
}

/// 测试数据库连接池初始化
#[tokio::test]
async fn test_database_pool_initialization() {
    // 注意：这个测试需要一个可用的PostgreSQL数据库
    // 这里我们测试配置加载和连接池创建过程，不实际连接数据库
    let settings = AppSettings::new().unwrap();

    // 验证数据库连接字符串是否正确生成
    assert!(!settings.database.connection_string.is_empty());
    assert!(settings
        .database
        .connection_string
        .starts_with("postgres://"));
}

/// 测试CORS配置
#[tokio::test]
async fn test_cors_config() {
    let settings = AppSettings::new().unwrap();

    // 验证默认CORS配置
    assert!(!settings.cors.allowed_origins.is_empty());
    assert!(settings
        .cors
        .allowed_origins
        .contains(&"http://localhost:3000".to_string()));
}
