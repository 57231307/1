//! 主备隔离模块 - 统一抽象接口
//!
//! 提供 FailoverCall trait，统一数据库/缓存/MQ/存储等核心功能的主备调用。
//!
//! # 设计原则
//!
//! - 仅主调用不可用时才自动切换至备用
//! - 主调用正常运行时禁用备用
//! - 故障转移后支持自动回切（半开状态探测）
//! - 配置化（不修改代码可调整主备 URL / 超时 / 熔断参数）
//!
//! # 核心类型
//!
//! - [`FailoverCall`]：主备调用 trait
//! - [`FailoverError`]：主备调用错误
//! - [`circuit_breaker::CircuitBreaker`]：熔断器

use std::time::Duration;
use async_trait::async_trait;
use thiserror::Error;
use tracing::{info, warn};

pub mod circuit_breaker;
pub mod database;
pub mod cache;

/// 主备调用错误
#[derive(Debug, Error)]
pub enum FailoverError<E> {
    /// 主调用失败
    #[error("主调用失败: {0}")]
    PrimaryFailed(E),
    /// 主调用超时
    #[error("主调用超时")]
    PrimaryTimeout,
    /// 备用调用失败
    #[error("备用调用失败: {0}")]
    BackupFailed(E),
    /// 备用调用超时
    #[error("备用调用超时")]
    BackupTimeout,
    /// 主备同时失败
    #[error("主备同时失败: 主={0:?}, 备={1:?}")]
    BothFailed(E, E),
    /// 主备同时超时
    #[error("主备同时超时")]
    BothTimeout,
    /// 熔断器打开
    #[error("熔断器已打开")]
    CircuitOpen,
}

/// 主备调用 trait
///
/// # 实现要点
///
/// - `primary_call` 实际调用主服务
/// - `backup_call` 实际调用备用服务
/// - `call` 方法封装主备切换逻辑
///
/// # 示例
///
/// ```ignore
/// use crate::utils::failover::{FailoverCall, FailoverError};
/// use std::time::Duration;
///
/// struct MyFailover { /* ... */ }
///
/// #[async_trait]
/// impl FailoverCall<String, String> for MyFailover {
///     async fn primary_call(&self) -> Result<String, String> { /* ... */ Ok(...) }
///     async fn backup_call(&self) -> Result<String, String> { /* ... */ Ok(...) }
///     fn primary_timeout(&self) -> Duration { Duration::from_secs(3) }
///     fn backup_timeout(&self) -> Duration { Duration::from_secs(5) }
///     fn function_name(&self) -> &str { "my_func" }
/// }
/// ```
#[async_trait]
pub trait FailoverCall<T, E>: Send + Sync {
    /// 主调用
    async fn primary_call(&self) -> Result<T, E>;

    /// 备用调用
    async fn backup_call(&self) -> Result<T, E>;

    /// 主调用超时
    fn primary_timeout(&self) -> Duration;

    /// 备用调用超时
    fn backup_timeout(&self) -> Duration;

    /// 函数名（监控 / 日志 / 状态记录用）
    fn function_name(&self) -> &str;

    /// 熔断器（可选，默认不启用）
    fn circuit_breaker(&self) -> Option<&circuit_breaker::CircuitBreaker> {
        None
    }

    /// 记录主调用成功（用于更新熔断器 / 指标）
    fn record_primary_success(&self) {
        if let Some(cb) = self.circuit_breaker() {
            cb.record_success();
        }
    }

    /// 记录主调用失败
    fn record_primary_failure(&self) {
        if let Some(cb) = self.circuit_breaker() {
            cb.record_failure();
        }
    }

    /// 记录切换事件
    fn record_switch(&self) {
        info!(
            function = self.function_name(),
            "主备切换"
        );
    }

    /// 检查熔断器是否打开
    fn is_circuit_open(&self) -> bool {
        self.circuit_breaker()
            .map(|cb| cb.is_open())
            .unwrap_or(false)
    }

    /// 调用备用服务
    async fn try_backup(&self) -> Result<T, FailoverError<E>> {
        warn!(
            function = self.function_name(),
            "主调用失败，切换至备用"
        );
        self.record_switch();
        match tokio::time::timeout(self.backup_timeout(), self.backup_call()).await {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(FailoverError::BackupFailed(e)),
            Err(_) => Err(FailoverError::BackupTimeout),
        }
    }

    /// 带主备隔离的调用入口
    async fn call(&self) -> Result<T, FailoverError<E>> {
        // 检查熔断器
        if self.is_circuit_open() {
            return self.try_backup().await;
        }

        // 尝试主调用
        match tokio::time::timeout(self.primary_timeout(), self.primary_call()).await {
            Ok(Ok(v)) => {
                self.record_primary_success();
                Ok(v)
            }
            Ok(Err(e)) => {
                self.record_primary_failure();
                Err(FailoverError::PrimaryFailed(e))
            }
            Err(_) => {
                self.record_primary_failure();
                self.try_backup().await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// 测试用 mock 主备调用
    struct MockFailover {
        primary_should_fail: Arc<AtomicU32>,
        backup_should_fail: Arc<AtomicU32>,
        func_name: String,
    }

    #[async_trait]
    impl FailoverCall<String, String> for MockFailover {
        async fn primary_call(&self) -> Result<String, String> {
            let n = self.primary_should_fail.fetch_sub(1, Ordering::AcqRel);
            if n > 0 {
                Err("primary error".to_string())
            } else {
                Ok("primary_ok".to_string())
            }
        }
        async fn backup_call(&self) -> Result<String, String> {
            let n = self.backup_should_fail.fetch_sub(1, Ordering::AcqRel);
            if n > 0 {
                Err("backup error".to_string())
            } else {
                Ok("backup_ok".to_string())
            }
        }
        fn primary_timeout(&self) -> Duration {
            Duration::from_millis(100)
        }
        fn backup_timeout(&self) -> Duration {
            Duration::from_millis(100)
        }
        fn function_name(&self) -> &str {
            &self.func_name
        }
    }

    #[tokio::test]
    async fn test_primary_success() {
        let mock = MockFailover {
            primary_should_fail: Arc::new(AtomicU32::new(0)),
            backup_should_fail: Arc::new(AtomicU32::new(0)),
            func_name: "test".to_string(),
        };
        let result = mock.call().await;
        assert!(matches!(result, Ok(ref v) if v == "primary_ok"));
    }

    #[tokio::test]
    async fn test_primary_fail_use_backup() {
        let mock = MockFailover {
            primary_should_fail: Arc::new(AtomicU32::new(1)),
            backup_should_fail: Arc::new(AtomicU32::new(0)),
            func_name: "test".to_string(),
        };
        let result = mock.call().await;
        assert!(matches!(result, Ok(ref v) if v == "backup_ok"));
    }

    #[tokio::test]
    async fn test_both_fail() {
        let mock = MockFailover {
            primary_should_fail: Arc::new(AtomicU32::new(1)),
            backup_should_fail: Arc::new(AtomicU32::new(1)),
            func_name: "test".to_string(),
        };
        let result = mock.call().await;
        // 主调用失败后切换备用，备用也失败，返回 BackupFailed
        assert!(matches!(result, Err(FailoverError::BackupFailed(_))));
    }
}
