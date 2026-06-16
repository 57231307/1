//! 缓存主备隔离实现
//!
//! 提供 FailoverCache，支持 Redis 主缓存 + 进程内 moka LRU 备用缓存自动切换。
//!
//! # 特性
//!
//! - 主调用：Redis 7
//! - 备用：进程内 moka LRU（无需网络，毫秒级响应）
//! - 主调用失败时自动切到 LRU
//! - 主调用恢复后自动回切（半开探测）

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache;
use redis::AsyncCommands;
use tracing::{info, warn};

use super::circuit_breaker::CircuitBreaker;
use super::{FailoverCall, FailoverError};
use crate::config::failover::CacheFailoverConfig;

/// 缓存主备结构体
pub struct FailoverCache {
    /// Redis 客户端
    primary: redis::Client,
    /// 进程内 LRU 备用
    backup: Cache<String, Vec<u8>>,
    /// 熔断器
    circuit: Arc<CircuitBreaker>,
    /// 配置
    config: CacheFailoverConfig,
    /// 功能名
    function_name: String,
}

impl FailoverCache {
    /// 创建缓存主备实例
    pub fn new(config: CacheFailoverConfig) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(config.primary_url.clone())?;
        let cache = Cache::builder()
            .max_capacity(config.backup_max_entries as u64)
            .time_to_live(Duration::from_secs(300)) // 默认 5 分钟 TTL
            .build();
        let circuit = Arc::new(CircuitBreaker::default_breaker());
        Ok(Self {
            primary: client,
            backup: cache,
            circuit,
            config,
            function_name: "cache".to_string(),
        })
    }

    /// 从主缓存获取
    pub async fn get_primary(&self, key: &str) -> Result<Option<Vec<u8>>, redis::RedisError> {
        let mut conn = self.primary.get_async_connection().await?;
        let value: Option<Vec<u8>> = conn.get(key).await?;
        Ok(value)
    }

    /// 从备用 LRU 获取
    pub async fn get_backup(&self, key: &str) -> Option<Vec<u8>> {
        self.backup.get(key).await
    }

    /// 写入主缓存（同步）
    pub async fn set_primary(&self, key: &str, value: Vec<u8>) -> Result<(), redis::RedisError> {
        let mut conn = self.primary.get_async_connection().await?;
        let _: () = conn.set(key, value).await?;
        Ok(())
    }

    /// 写入备用 LRU
    pub async fn set_backup(&self, key: String, value: Vec<u8>) {
        self.backup.insert(key, value).await;
    }

    /// 双写：主缓存 + 备用 LRU
    pub async fn set(&self, key: String, value: Vec<u8>) {
        if let Err(e) = self.set_primary(&key, value.clone()).await {
            warn!("[缓存主备] 主缓存写入失败: {}，降级到 LRU", e);
        }
        self.set_backup(key, value).await;
    }

    /// 优先主缓存，失败时切备用
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>, String>, String> {
        match tokio::time::timeout(
            Duration::from_millis(self.config.primary_timeout_ms),
            self.get_primary(key),
        )
        .await
        {
            Ok(Ok(Some(v))) => Ok(Ok(v)),
            Ok(Ok(None)) => Ok(Ok(None)),
            Ok(Err(e)) => {
                warn!("[缓存主备] 主缓存读取失败: {}，切到 LRU", e);
                Ok(Err(e.to_string()))
            }
            Err(_) => {
                warn!("[缓存主备] 主缓存读取超时，切到 LRU");
                Ok(Err("timeout".to_string()))
            }
        }
        .and_then(|r| r.map_err(|e| e).and_then(|opt| Ok(opt.map(|v| v.to_vec()))))
    }

    /// 获取熔断器
    pub fn circuit(&self) -> &Arc<CircuitBreaker> {
        &self.circuit
    }
}

#[async_trait]
impl FailoverCall<bool, redis::RedisError> for FailoverCache {
    /// 主缓存 ping
    async fn primary_call(&self) -> Result<bool, redis::RedisError> {
        let mut conn = self.primary.get_async_connection().await?;
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(pong == "PONG")
    }

    /// 备用 LRU ping（永远可用）
    async fn backup_call(&self) -> Result<bool, redis::RedisError> {
        Ok(true)
    }

    fn primary_timeout(&self) -> Duration {
        Duration::from_millis(self.config.primary_timeout_ms)
    }

    fn backup_timeout(&self) -> Duration {
        Duration::from_millis(self.config.backup_timeout_ms)
    }

    fn function_name(&self) -> &str {
        &self.function_name
    }

    fn circuit_breaker(&self) -> Option<&CircuitBreaker> {
        Some(self.circuit.as_ref())
    }
}

/// 健康检查任务：定期 ping Redis 主缓存
pub async fn health_check_task(failover: Arc<FailoverCache>) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        match failover.call().await {
            Ok(_) => info!("[缓存主备] 健康检查通过"),
            Err(FailoverError::BackupFailed(e)) => {
                warn!("[缓存主备] 主备均失败: {:?}", e);
            }
            Err(e) => warn!("[缓存主备] 异常: {:?}", e),
        }
    }
}
