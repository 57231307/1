//! 日志文件保留期清理服务：按 90 天保留期自动清理滚动日志文件。
//!
//! V15 P1 20.8-B 修复：原系统使用 tracing_appender 按日滚动生成日志文件
//! （bingxi_backend.log.YYYY-MM-DD、audit/*.log.YYYY-MM-DD 等），但缺乏自动
//! 清理机制，长期运行后磁盘被占满导致服务崩溃。本服务每天扫描日志目录，
//! 删除修改时间超过 retention_days 的文件，确保磁盘可控。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use tokio::time::{interval, Interval};
use tracing::{info, warn};

/// 默认日志保留天数（V15 P1 20.8-B 要求 90 天）
pub const DEFAULT_LOG_RETENTION_DAYS: i32 = 90;

/// 日志清理服务：每天扫描 log_dir 及其子目录，删除超过 retention_days 的滚动日志文件。
pub struct LogCleanupService {
    log_dir: PathBuf,
    retention_days: i32,
}

impl LogCleanupService {
    /// 创建日志清理服务实例。
    pub fn new(log_dir: impl Into<PathBuf>, retention_days: i32) -> Self {
        let retention_days = if retention_days > 0 {
            retention_days
        } else {
            warn!(
                retention_days,
                "LogCleanupService retention_days 非正数，回退为默认 90 天"
            );
            DEFAULT_LOG_RETENTION_DAYS
        };
        Self {
            log_dir: log_dir.into(),
            retention_days,
        }
    }

    /// 启动后台定期清理任务（每天执行一次，panic 隔离确保循环不退出）。
    /// 返回 JoinHandle 供 graceful shutdown 管理。
    pub fn start_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let service = self.clone();
        let handle = tokio::spawn(async move {
            // 启动后立即执行一次清理，避免长期未清理的日志在首次启动后仍占用磁盘
            service.run_once_with_panic_isolation().await;
            let mut tick: Interval = interval(Duration::from_secs(24 * 60 * 60));
            loop {
                tick.tick().await;
                service.run_once_with_panic_isolation().await;
            }
        });
        info!(
            log_dir = %self.log_dir.display(),
            retention_days = self.retention_days,
            "日志清理后台任务已启动（每日扫描，保留 {} 天）",
            self.retention_days
        );
        handle
    }

    /// 执行一次清理并隔离 panic，确保循环任务不会因单次 panic 退出。
    async fn run_once_with_panic_isolation(self: Arc<Self>) {
        let result = AssertUnwindSafe(async {
            match self.cleanup_once().await {
                Ok(deleted) => {
                    if deleted > 0 {
                        info!(deleted, "日志清理完成（已删除 {} 个过期文件）", deleted);
                    }
                }
                Err(e) => warn!(error = %e, "日志清理失败"),
            }
        })
        .catch_unwind()
        .await;
        if let Err(payload) = result {
            let msg = payload
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| payload.downcast_ref::<&'static str>().copied())
                .unwrap_or("<非字符串 panic payload>");
            warn!(
                panic = %msg,
                "⚠ 日志清理 spawn 任务内 panic 已被隔离，清理循环继续运行"
            );
        }
    }

    /// 执行一次清理：递归扫描 log_dir，删除修改时间超过 retention_days 的文件。
    /// 返回已删除文件数。
    pub async fn cleanup_once(&self) -> Result<usize, std::io::Error> {
        if !self.log_dir.exists() {
            info!(
                log_dir = %self.log_dir.display(),
                "日志目录不存在，跳过清理"
            );
            return Ok(0);
        }
        let retention = Duration::from_secs(60 * 60 * 24 * self.retention_days as u64);
        let cutoff = SystemTime::now()
            .checked_sub(retention)
            .unwrap_or_else(SystemTime::UNIX_EPOCH);
        // 在 blocking 线程执行文件系统遍历，避免阻塞 tokio runtime
        let log_dir = self.log_dir.clone();
        tokio::task::spawn_blocking(move || Self::cleanup_dir_recursive(&log_dir, cutoff))
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
    }

    /// 递归清理目录下所有修改时间早于 cutoff 的文件（保留子目录结构）。
    fn cleanup_dir_recursive(dir: &Path, cutoff: SystemTime) -> std::io::Result<usize> {
        let mut deleted: usize = 0;
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e),
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "读取目录条目失败，跳过");
                    continue;
                }
            };
            let path = entry.path();
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    warn!(error = %e, path = %path.display(), "读取文件元数据失败，跳过");
                    continue;
                }
            };
            if meta.is_dir() {
                // 递归清理子目录（audit/ security/ performance/）
                match Self::cleanup_dir_recursive(&path, cutoff) {
                    Ok(n) => deleted += n,
                    Err(e) => warn!(error = %e, path = %path.display(), "递归清理子目录失败"),
                }
            } else if meta.is_file() {
                if Self::should_delete(&meta, cutoff) {
                    if let Err(e) = std::fs::remove_file(&path) {
                        warn!(error = %e, path = %path.display(), "删除过期日志文件失败");
                    } else {
                        deleted += 1;
                    }
                }
            }
        }
        Ok(deleted)
    }

    /// 判定文件是否应被删除：修改时间早于 cutoff 即视为过期。
    /// tracing_appender 滚动文件名形如 `bingxi_backend.log.2024-01-01`，
    /// 修改时间是当日写入完成时间，比文件名日期更准确（避免时区漂移）。
    fn should_delete(meta: &std::fs::Metadata, cutoff: SystemTime) -> bool {
        match meta.modified() {
            Ok(mtime) => mtime < cutoff,
            Err(_) => false, // 无法获取 mtime 时不删除，避免误删活跃文件
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 构造唯一临时目录：/tmp/bingxi_log_cleanup_test_<uuid>
    fn make_temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bingxi_log_cleanup_test_{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).expect("创建临时目录失败");
        dir
    }

    #[test]
    fn cleanup_deletes_files_older_than_cutoff() {
        let dir = make_temp_dir();
        // 创建若干文件（mtime = 当前时间）
        fs::File::create(dir.join("recent.log")).expect("创建文件失败");
        fs::create_dir_all(dir.join("audit")).expect("创建子目录失败");
        fs::File::create(dir.join("audit").join("recent_audit.log")).expect("创建文件失败");

        // cutoff 设为未来 1 小时：当前 mtime < 未来 cutoff，所有文件应被删除
        let future_cutoff = SystemTime::now() + Duration::from_secs(3600);
        let deleted =
            LogCleanupService::cleanup_dir_recursive(&dir, future_cutoff).expect("清理失败");

        assert_eq!(deleted, 2);
        assert!(!dir.join("recent.log").exists());
        assert!(!dir.join("audit").join("recent_audit.log").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cleanup_preserves_files_newer_than_cutoff() {
        let dir = make_temp_dir();
        fs::File::create(dir.join("keep.log")).expect("创建文件失败");

        // cutoff 设为过去 1 小时：当前 mtime > 过去 cutoff，文件应保留
        let past_cutoff = SystemTime::now() - Duration::from_secs(3600);
        let deleted =
            LogCleanupService::cleanup_dir_recursive(&dir, past_cutoff).expect("清理失败");

        assert_eq!(deleted, 0);
        assert!(dir.join("keep.log").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cleanup_missing_dir_returns_zero() {
        let missing = Path::new("/nonexistent/bingxi_log_cleanup_test_missing_dir");
        let cutoff = SystemTime::now();
        let deleted =
            LogCleanupService::cleanup_dir_recursive(missing, cutoff).expect("应返回 0");
        assert_eq!(deleted, 0);
    }
}
