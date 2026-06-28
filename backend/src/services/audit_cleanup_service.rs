use futures::FutureExt;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::time::{interval, Duration};

pub struct AuditCleanupService {
    db: Arc<DatabaseConnection>,
    retention_days: i32,
}

impl AuditCleanupService {
    pub fn new(db: Arc<DatabaseConnection>, retention_days: i32) -> Self {
        Self { db, retention_days }
    }

    /// 启动定期清理任务
    pub fn start_cleanup_task(self: Arc<Self>) {
        let service = self.clone();
        tokio::spawn(async move {
            // 每天执行一次清理
            let mut interval = interval(Duration::from_secs(24 * 60 * 60));
            loop {
                interval.tick().await;
                // 批次 7（2026-06-28）：单次清理 panic 隔离
                // 审计日志清理是长期循环任务，若 panic 会导致 omni_audit_logs / audit_logs
                // 表无限增长，最终拖挂数据库。确保单次 panic 不退出循环。
                let result = AssertUnwindSafe(async {
                    if let Err(e) = service.cleanup_expired_logs().await {
                        tracing::error!("审计日志清理失败: {}", e);
                    }
                })
                .catch_unwind()
                .await;
                if let Err(panic_payload) = result {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ 审计日志清理 spawn 任务内 panic 已被隔离，清理循环继续运行（不退出）"
                    );
                }
            }
        });
    }

    /// 清理过期的审计日志
    pub async fn cleanup_expired_logs(&self) -> Result<u64, sea_orm::DbErr> {
        let sql = format!(
            "DELETE FROM omni_audit_logs WHERE created_at < NOW() - INTERVAL '{} days'",
            self.retention_days
        );

        let result = self
            .db
            .as_ref()
            .execute_unprepared(&sql)
            .await?;

        let deleted_count = result.rows_affected();

        if deleted_count > 0 {
            tracing::info!(
                "已清理 {} 条过期审计日志（保留 {} 天）",
                deleted_count,
                self.retention_days
            );
        }

        // 同时清理 audit_logs 表
        let sql = format!(
            "DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '{} days'",
            self.retention_days
        );

        let result = self
            .db
            .as_ref()
            .execute_unprepared(&sql)
            .await?;

        let deleted_count2 = result.rows_affected();
        if deleted_count2 > 0 {
            tracing::info!(
                "已清理 {} 条过期操作日志（保留 {} 天）",
                deleted_count2,
                self.retention_days
            );
        }

        Ok(deleted_count + deleted_count2)
    }

    /// 获取审计日志统计信息
    pub async fn get_stats(&self) -> Result<AuditStats, sea_orm::DbErr> {
        let sql = "SELECT 
            (SELECT COUNT(*) FROM omni_audit_logs) as total_omni_logs,
            (SELECT COUNT(*) FROM audit_logs) as total_audit_logs,
            (SELECT COUNT(*) FROM omni_audit_logs WHERE created_at > NOW() - INTERVAL '24 hours') as today_omni_logs,
            (SELECT COUNT(*) FROM audit_logs WHERE created_at > NOW() - INTERVAL '24 hours') as today_audit_logs,
            (SELECT MIN(created_at) FROM omni_audit_logs) as oldest_omni_log,
            (SELECT MAX(created_at) FROM omni_audit_logs) as newest_omni_log";

        let result: Option<sea_orm::QueryResult> = self
            .db
            .as_ref()
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql.to_string(),
            ))
            .await?;

        if let Some(row) = result {
            // DB 查询失败应传播错误而非吞掉为 0，避免审计统计与实际不符
            Ok(AuditStats {
                total_omni_logs: row.try_get::<i64>("", "total_omni_logs")?,
                total_audit_logs: row.try_get::<i64>("", "total_audit_logs")?,
                today_omni_logs: row.try_get::<i64>("", "today_omni_logs")?,
                today_audit_logs: row.try_get::<i64>("", "today_audit_logs")?,
                oldest_log: row.try_get::<String>("", "oldest_omni_log").ok(),
                newest_log: row.try_get::<String>("", "newest_omni_log").ok(),
            })
        } else {
            Ok(AuditStats::default())
        }
    }
}

#[derive(Debug, Default)]
pub struct AuditStats {
    pub total_omni_logs: i64,
    pub total_audit_logs: i64,
    pub today_omni_logs: i64,
    pub today_audit_logs: i64,
    pub oldest_log: Option<String>,
    pub newest_log: Option<String>,
}
