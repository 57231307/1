// 集成测试：FailoverCall trait + 主备切换逻辑
// 注意：沙箱 OOM 跑不了 cargo test，本测试依赖 CI 验证。

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bingxi_backend::utils::failover::{FailoverCall, FailoverError};

/// 测试用 mock 主备调用
struct MockFailover {
    primary_should_fail: Arc<AtomicU32>,
    backup_should_fail: Arc<AtomicU32>,
    primary_delay_ms: u64,
    backup_delay_ms: u64,
    func_name: String,
}

#[async_trait]
impl FailoverCall<String, String> for MockFailover {
    async fn primary_call(&self) -> Result<String, String> {
        tokio::time::sleep(Duration::from_millis(self.primary_delay_ms)).await;
        let n = self.primary_should_fail.fetch_sub(1, Ordering::AcqRel);
        if n > 0 {
            Err("primary error".to_string())
        } else {
            Ok("primary_ok".to_string())
        }
    }
    async fn backup_call(&self) -> Result<String, String> {
        tokio::time::sleep(Duration::from_millis(self.backup_delay_ms)).await;
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
async fn test_failover_trait_primary_success() {
    let mock = MockFailover {
        primary_should_fail: Arc::new(AtomicU32::new(0)),
        backup_should_fail: Arc::new(AtomicU32::new(0)),
        primary_delay_ms: 0,
        backup_delay_ms: 0,
        func_name: "test_primary_success".to_string(),
    };
    let result = mock.call().await;
    assert!(matches!(result, Ok(ref v) if v == "primary_ok"));
}

#[tokio::test]
async fn test_failover_trait_primary_fail_uses_backup() {
    let mock = MockFailover {
        primary_should_fail: Arc::new(AtomicU32::new(1)),
        backup_should_fail: Arc::new(AtomicU32::new(0)),
        primary_delay_ms: 0,
        backup_delay_ms: 0,
        func_name: "test_primary_fail".to_string(),
    };
    let result = mock.call().await;
    assert!(matches!(result, Ok(ref v) if v == "backup_ok"));
}

#[tokio::test]
async fn test_failover_trait_primary_timeout_uses_backup() {
    let mock = MockFailover {
        primary_should_fail: Arc::new(AtomicU32::new(0)),
        backup_should_fail: Arc::new(AtomicU32::new(0)),
        primary_delay_ms: 200, // 超过 primary_timeout 100ms
        backup_delay_ms: 0,
        func_name: "test_primary_timeout".to_string(),
    };
    let result = mock.call().await;
    // 主调用超时 → 切换到备用
    assert!(matches!(result, Ok(ref v) if v == "backup_ok"));
}

#[tokio::test]
async fn test_failover_trait_both_fail() {
    let mock = MockFailover {
        primary_should_fail: Arc::new(AtomicU32::new(1)),
        backup_should_fail: Arc::new(AtomicU32::new(1)),
        primary_delay_ms: 0,
        backup_delay_ms: 0,
        func_name: "test_both_fail".to_string(),
    };
    let result = mock.call().await;
    // 主调用失败 → 切备用 → 备用也失败 → BackupFailed
    assert!(matches!(result, Err(FailoverError::BackupFailed(_))));
}
