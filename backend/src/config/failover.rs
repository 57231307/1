//! 主备隔离配置模块
//!
//! 加载 `config/failover.toml`，提供数据库 / 缓存的主备配置。

use serde::Deserialize;
use std::path::Path;

/// 主备隔离配置
#[derive(Debug, Deserialize, Clone)]
pub struct FailoverConfig {
    /// 数据库主备配置
    pub database: DatabaseFailoverConfig,
    /// 缓存主备配置
    pub cache: CacheFailoverConfig,
    /// 监控配置
    #[serde(default)]
    pub monitoring: MonitoringFailoverConfig,
}

/// 数据库主备配置
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseFailoverConfig {
    /// 主库 URL
    pub primary_url: String,
    /// 备库 URL
    pub backup_url: String,
    /// 主调用超时（毫秒）
    pub primary_timeout_ms: u64,
    /// 备用调用超时（毫秒）
    pub backup_timeout_ms: u64,
    /// 熔断阈值（连续失败次数）
    pub circuit_breaker_threshold: u32,
    /// 熔断时长（秒）
    pub circuit_breaker_duration_s: u64,
}

/// 缓存主备配置
///
/// BE-D 修复（2026-06-26 第三优先级）：
/// CacheFailoverConfig 的 primary_url / primary_timeout_ms / backup_timeout_ms
/// 字段当前未在业务代码中被读取（缓存主备切换未接入）。
#[allow(dead_code)]
// TODO(tech-debt): 缓存主备故障切换接入后移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
#[derive(Debug, Deserialize, Clone)]
pub struct CacheFailoverConfig {
    /// Redis 主缓存 URL
    pub primary_url: String,
    /// 进程内 LRU 最大条目数
    pub backup_max_entries: usize,
    /// 主调用超时（毫秒）
    pub primary_timeout_ms: u64,
    /// 备用调用超时（毫秒）
    pub backup_timeout_ms: u64,
}

/// 监控配置
#[derive(Debug, Deserialize, Clone)]
pub struct MonitoringFailoverConfig {
    /// 是否启用指标
    #[serde(default = "default_true")]
    #[allow(dead_code)] // TODO(tech-debt): 主备监控配置接入业务后移除
    pub metrics_enabled: bool,
    /// 日志级别
    #[serde(default = "default_log_level")]
    #[allow(dead_code)] // TODO(tech-debt): 主备监控配置接入业务后移除
    pub log_level: String,
}

impl Default for MonitoringFailoverConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            log_level: "info".to_string(),
        }
    }
}

#[allow(dead_code)] // TODO(tech-debt): serde 默认值函数，监控配置业务接入后评估是否保留
fn default_true() -> bool {
    true
}

#[allow(dead_code)] // TODO(tech-debt): serde 默认值函数，监控配置业务接入后评估是否保留
fn default_log_level() -> String {
    "info".to_string()
}

impl FailoverConfig {
    /// 从文件加载配置
    #[allow(dead_code)] // TODO(tech-debt): 配置文件加载业务接入后移除
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("解析 TOML 失败: {}", e))
    }

    /// 从环境变量加载（覆盖）
    pub fn load_from_env() -> Result<Self, String> {
        Ok(Self {
            database: DatabaseFailoverConfig {
                primary_url: std::env::var("DATABASE_URL_PRIMARY")
                    .map_err(|_| "DATABASE_URL_PRIMARY 未设置".to_string())?,
                backup_url: std::env::var("DATABASE_URL_BACKUP")
                    .map_err(|_| "DATABASE_URL_BACKUP 未设置".to_string())?,
                primary_timeout_ms: parse_env_u64("DATABASE_PRIMARY_TIMEOUT_MS", 3000),
                backup_timeout_ms: parse_env_u64("DATABASE_BACKUP_TIMEOUT_MS", 5000),
                circuit_breaker_threshold: parse_env_u64("DATABASE_CB_THRESHOLD", 5) as u32,
                circuit_breaker_duration_s: parse_env_u64("DATABASE_CB_DURATION_S", 30),
            },
            cache: CacheFailoverConfig {
                primary_url: std::env::var("REDIS_URL")
                    .map_err(|_| "REDIS_URL 未设置".to_string())?,
                backup_max_entries: parse_env_u64("CACHE_LRU_MAX_ENTRIES", 10_000) as usize,
                primary_timeout_ms: parse_env_u64("CACHE_PRIMARY_TIMEOUT_MS", 1000),
                backup_timeout_ms: parse_env_u64("CACHE_BACKUP_TIMEOUT_MS", 0),
            },
            monitoring: MonitoringFailoverConfig::default(),
        })
    }

    /// 默认配置（用于测试）
    pub fn default_for_test() -> Self {
        Self {
            database: DatabaseFailoverConfig {
                primary_url: "postgres://localhost/test".to_string(),
                backup_url: "postgres://localhost/test2".to_string(),
                primary_timeout_ms: 3000,
                backup_timeout_ms: 5000,
                circuit_breaker_threshold: 5,
                circuit_breaker_duration_s: 30,
            },
            cache: CacheFailoverConfig {
                primary_url: "redis://localhost:6379".to_string(),
                backup_max_entries: 10_000,
                primary_timeout_ms: 1000,
                backup_timeout_ms: 0,
            },
            monitoring: MonitoringFailoverConfig::default(),
        }
    }
}

fn parse_env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FailoverConfig::default_for_test();
        assert_eq!(config.database.circuit_breaker_threshold, 5);
        assert_eq!(config.cache.backup_max_entries, 10_000);
    }

    #[test]
    fn test_parse_env() {
        std::env::set_var("DATABASE_URL_PRIMARY", "postgres://test");
        std::env::set_var("DATABASE_URL_BACKUP", "postgres://test2");
        std::env::set_var("REDIS_URL", "redis://test");
        let result = FailoverConfig::load_from_env();
        assert!(result.is_ok());
    }
}
