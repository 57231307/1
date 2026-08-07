use crate::utils::error::AppError;
use futures::FutureExt;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

pub struct AuditCleanupService {
    db: Arc<DatabaseConnection>,
    retention_days: i32,
    permission_audit_retention_days: i32,
    security_alert_retention_days: i32,
}

impl AuditCleanupService {
    pub fn new(db: Arc<DatabaseConnection>, retention_days: i32) -> Self {
        Self {
            db,
            retention_days,
            permission_audit_retention_days: 2555, // 7 年
            security_alert_retention_days: 2555,   // 7 年
        }
    }

    /// batch-12 P2-8：启动定期清理任务（返回 JoinHandle + 接受 CancellationToken）
    pub fn start_cleanup_task(
        self: Arc<Self>,
        cancellation_token: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let service = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(24 * 60 * 60));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
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
                                "审计日志清理任务 panic 已被隔离，清理循环继续运行"
                            );
                        }
                    }
                    _ = cancellation_token.cancelled() => {
                        tracing::info!("审计日志清理任务收到取消信号，优雅退出");
                        break;
                    }
                }
            }
        })
    }

    /// 清理过期的审计日志（分级保留）
    pub async fn cleanup_expired_logs(&self) -> Result<u64, AppError> {
        let mut total_deleted = 0u64;

        // omni_audit_logs: 保留 retention_days（默认 365 天）
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM omni_audit_logs WHERE created_at < NOW() - ($1 * INTERVAL '1 day')",
            [self.retention_days.into()],
        );
        let result = self.db.as_ref().execute(stmt).await?;
        let deleted_count = result.rows_affected();
        if deleted_count > 0 {
            tracing::info!(
                "已清理 {} 条过期 omni_audit_logs（保留 {} 天）",
                deleted_count,
                self.retention_days
            );
        }
        total_deleted += deleted_count;

        // audit_logs: 保留 retention_days（默认 365 天）
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM audit_logs WHERE created_at < NOW() - ($1 * INTERVAL '1 day')",
            [self.retention_days.into()],
        );
        let result = self.db.as_ref().execute(stmt).await?;
        let deleted_count2 = result.rows_affected();
        if deleted_count2 > 0 {
            tracing::info!(
                "已清理 {} 条过期 audit_logs（保留 {} 天）",
                deleted_count2,
                self.retention_days
            );
        }
        total_deleted += deleted_count2;

        // permission_change_audits: 保留 7 年（2555 天）
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM permission_change_audits WHERE changed_at < NOW() - ($1 * INTERVAL '1 day')",
            [self.permission_audit_retention_days.into()],
        );
        let result = self.db.as_ref().execute(stmt).await?;
        let deleted_count3 = result.rows_affected();
        if deleted_count3 > 0 {
            tracing::info!(
                "已清理 {} 条过期 permission_change_audits（保留 {} 天）",
                deleted_count3,
                self.permission_audit_retention_days
            );
        }
        total_deleted += deleted_count3;

        // security_alert_logs: 保留 7 年（2555 天）
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM security_alert_logs WHERE created_at < NOW() - ($1 * INTERVAL '1 day')",
            [self.security_alert_retention_days.into()],
        );
        let result = self.db.as_ref().execute(stmt).await?;
        let deleted_count4 = result.rows_affected();
        if deleted_count4 > 0 {
            tracing::info!(
                "已清理 {} 条过期 security_alert_logs（保留 {} 天）",
                deleted_count4,
                self.security_alert_retention_days
            );
        }
        total_deleted += deleted_count4;

        Ok(total_deleted)
    }

    /// 获取审计日志统计信息
    pub async fn get_stats(&self) -> Result<AuditStats, AppError> {
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
