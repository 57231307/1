// 集成测试：FailoverConfig 配置加载

use bingxi_backend::config::failover::FailoverConfig;

#[test]
fn test_default_for_test() {
    let config = FailoverConfig::default_for_test();
    assert_eq!(config.database.circuit_breaker_threshold, 5);
    assert_eq!(config.database.circuit_breaker_duration_s, 30);
    assert_eq!(config.cache.backup_max_entries, 10_000);
    assert_eq!(config.cache.primary_timeout_ms, 1000);
}

#[test]
fn test_default_monitoring_config() {
    let config = FailoverConfig::default_for_test();
    assert!(config.monitoring.metrics_enabled);
    assert_eq!(config.monitoring.log_level, "info");
}

#[test]
#[ignore] // 依赖环境变量，CI 中单独运行
fn test_load_from_env() {
    std::env::set_var("DATABASE_URL_PRIMARY", "postgres://test");
    std::env::set_var("DATABASE_URL_BACKUP", "postgres://test2");
    std::env::set_var("REDIS_URL", "redis://test");
    let result = FailoverConfig::load_from_env();
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.database.primary_url, "postgres://test");
    assert_eq!(config.cache.primary_url, "redis://test");
}

#[test]
fn test_load_from_file_not_exist() {
    let result = FailoverConfig::load_from_file("/tmp/nonexistent.toml");
    assert!(result.is_err());
}
