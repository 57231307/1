//! 用户行为追踪数据 90 天保留策略服务（V15 P1 batch-16 缺陷 8.3/8.4）
//!
//! 提供 page_views 与 user_behaviors 表的 90 天保留策略：
//! 1. 按日聚合超过 retention_days 的明细到 page_view_daily_summary / user_behavior_daily_summary
//! 2. 聚合后删除明细记录，避免明细表无限膨胀
//! 3. 后台调度任务每日执行一次（默认 02:00 启动，可配置）
//!
//! 合规依据：《个人信息保护法》第 19 条（数据最小化原则）+ GDPR 第 5 条
//!
//! 参考模板：audit_cleanup_service.rs（同模式的后台清理任务）

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use futures::FutureExt;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde::Serialize;
use tokio::time::{interval, Duration};

use crate::utils::error::AppError;

/// 默认保留期（90 天）
pub const DEFAULT_RETENTION_DAYS: i32 = 90;

/// 默认扫描间隔（24 小时）
const DEFAULT_INTERVAL_SECS: u64 = 24 * 60 * 60;

/// 启动初始延迟（60 秒，避开启动期高峰）
const INITIAL_DELAY_SECS: u64 = 60;

/// 单轮扫描批量大小（避免一次删除过多明细锁表）
const BATCH_SIZE: i64 = 5000;

/// 追踪数据保留策略 Service
///
/// 同时管理 page_views 与 user_behaviors 两张明细表的归档清理。
pub struct TrackingCleanupService {
    db: Arc<DatabaseConnection>,
    retention_days: i32,
}

impl TrackingCleanupService {
    pub fn new(db: Arc<DatabaseConnection>, retention_days: i32) -> Self {
        Self {
            db,
            retention_days: retention_days.max(1),
        }
    }

    /// 启动定期归档清理任务（默认每 24 小时执行一次）
    pub fn start_background_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let enabled = std::env::var("TRACKING_CLEANUP_ENABLED")
                .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
                .unwrap_or(true);
            if !enabled {
                tracing::info!("追踪数据清理：环境变量 TRACKING_CLEANUP_ENABLED=false，跳过启动");
                return;
            }

            let interval_secs = std::env::var("TRACKING_CLEANUP_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(DEFAULT_INTERVAL_SECS);

            // 启动初始延迟，避开启动期高峰
            tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;

            tracing::info!(
                retention_days = self.retention_days,
                interval_secs,
                "追踪数据清理：后台任务已启动（每 {} 秒扫描一次过期明细）",
                interval_secs
            );

            let mut ticker = interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                // 单次清理 panic 隔离：避免单次失败导致整个循环退出
                let result = AssertUnwindSafe(async {
                    if let Err(e) = self.run_once().await {
                        tracing::error!(error = %e, "追踪数据清理失败");
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
                        "⚠ 追踪数据清理 spawn 任务内 panic 已被隔离，清理循环继续运行"
                    );
                }
            }
        })
    }

    /// 单轮清理：依次归档并删除 page_views 与 user_behaviors 过期明细
    pub async fn run_once(&self) -> Result<CleanupStats, AppError> {
        let page_view_archived = self.archive_page_views().await?;
        let user_behavior_archived = self.archive_user_behaviors().await?;

        tracing::info!(
            page_view_archived,
            user_behavior_archived,
            retention_days = self.retention_days,
            "追踪数据归档完成（保留 {} 天）",
            self.retention_days
        );

        Ok(CleanupStats {
            page_view_archived,
            user_behavior_archived,
        })
    }

    /// 缺陷 8.3 修复：归档 page_views 明细到 page_view_daily_summary 后删除
    ///
    /// 按 (stat_date, path) 聚合 total_views / unique_sessions / unique_users，
    /// UPSERT 到 page_view_daily_summary 后批量删除明细。
    async fn archive_page_views(&self) -> Result<i64, AppError> {
        // 1. 聚合过期明细到汇总表（UPSERT 语义：已存在则累加）
        //    使用 INSERT ... ON CONFLICT DO UPDATE 保证幂等
        let archive_sql = r#"
            INSERT INTO page_view_daily_summary (stat_date, path, total_views, unique_sessions, unique_users, created_at)
            SELECT
                DATE(viewed_at) AS stat_date,
                path,
                COUNT(*) AS total_views,
                COUNT(DISTINCT session_id) AS unique_sessions,
                COUNT(DISTINCT user_id) AS unique_users,
                NOW() AS created_at
            FROM page_views
            WHERE viewed_at < NOW() - ($1 * INTERVAL '1 day')
              AND id <= (
                  SELECT MAX(id) FROM page_views
                  WHERE viewed_at < NOW() - ($1 * INTERVAL '1 day')
              )
            GROUP BY DATE(viewed_at), path
            ON CONFLICT (stat_date, path) DO UPDATE SET
                total_views = page_view_daily_summary.total_views + EXCLUDED.total_views,
                unique_sessions = GREATEST(page_view_daily_summary.unique_sessions, EXCLUDED.unique_sessions),
                unique_users = GREATEST(page_view_daily_summary.unique_users, EXCLUDED.unique_users),
                created_at = NOW()
        "#;
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            archive_sql,
            [self.retention_days.into()],
        );
        self.db.as_ref().execute(stmt).await?;

        // 2. 批量删除已归档的明细（按 id 上限分批，避免锁表）
        let delete_sql = r#"
            DELETE FROM page_views
            WHERE id IN (
                SELECT id FROM page_views
                WHERE viewed_at < NOW() - ($1 * INTERVAL '1 day')
                LIMIT $2
            )
        "#;
        let mut total_deleted: i64 = 0;
        loop {
            let stmt = Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                delete_sql,
                [self.retention_days.into(), BATCH_SIZE.into()],
            );
            let result = self.db.as_ref().execute(stmt).await?;
            let deleted = result.rows_affected() as i64;
            total_deleted += deleted;
            if deleted < BATCH_SIZE {
                break;
            }
        }
        Ok(total_deleted)
    }

    /// 缺陷 8.4 修复：归档 user_behaviors 明细到 user_behavior_daily_summary 后删除
    ///
    /// 按 (stat_date, event_type) 聚合 total_count / unique_users / unique_sessions，
    /// UPSERT 到 user_behavior_daily_summary 后批量删除明细。
    async fn archive_user_behaviors(&self) -> Result<i64, AppError> {
        let archive_sql = r#"
            INSERT INTO user_behavior_daily_summary (stat_date, event_type, total_count, unique_users, unique_sessions, created_at)
            SELECT
                DATE(occurred_at) AS stat_date,
                event_type,
                COUNT(*) AS total_count,
                COUNT(DISTINCT user_id) AS unique_users,
                COUNT(DISTINCT session_id) AS unique_sessions,
                NOW() AS created_at
            FROM user_behaviors
            WHERE occurred_at < NOW() - ($1 * INTERVAL '1 day')
              AND id <= (
                  SELECT MAX(id) FROM user_behaviors
                  WHERE occurred_at < NOW() - ($1 * INTERVAL '1 day')
              )
            GROUP BY DATE(occurred_at), event_type
            ON CONFLICT (stat_date, event_type) DO UPDATE SET
                total_count = user_behavior_daily_summary.total_count + EXCLUDED.total_count,
                unique_users = GREATEST(user_behavior_daily_summary.unique_users, EXCLUDED.unique_users),
                unique_sessions = GREATEST(user_behavior_daily_summary.unique_sessions, EXCLUDED.unique_sessions),
                created_at = NOW()
        "#;
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            archive_sql,
            [self.retention_days.into()],
        );
        self.db.as_ref().execute(stmt).await?;

        // 批量删除已归档的明细
        let delete_sql = r#"
            DELETE FROM user_behaviors
            WHERE id IN (
                SELECT id FROM user_behaviors
                WHERE occurred_at < NOW() - ($1 * INTERVAL '1 day')
                LIMIT $2
            )
        "#;
        let mut total_deleted: i64 = 0;
        loop {
            let stmt = Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                delete_sql,
                [self.retention_days.into(), BATCH_SIZE.into()],
            );
            let result = self.db.as_ref().execute(stmt).await?;
            let deleted = result.rows_affected() as i64;
            total_deleted += deleted;
            if deleted < BATCH_SIZE {
                break;
            }
        }
        Ok(total_deleted)
    }

    /// 获取追踪数据保留统计信息（运维监控用）
    pub async fn get_stats(&self) -> Result<TrackingStats, AppError> {
        let sql = r#"
            SELECT
                (SELECT COUNT(*) FROM page_views) AS total_page_views,
                (SELECT COUNT(*) FROM page_views WHERE viewed_at < NOW() - ($1 * INTERVAL '1 day')) AS expired_page_views,
                (SELECT COUNT(*) FROM user_behaviors) AS total_user_behaviors,
                (SELECT COUNT(*) FROM user_behaviors WHERE occurred_at < NOW() - ($1 * INTERVAL '1 day')) AS expired_user_behaviors,
                (SELECT MIN(viewed_at) FROM page_views) AS oldest_page_view,
                (SELECT MAX(viewed_at) FROM page_views) AS newest_page_view,
                (SELECT MIN(occurred_at) FROM user_behaviors) AS oldest_user_behavior,
                (SELECT MAX(occurred_at) FROM user_behaviors) AS newest_user_behavior
        "#;
        let result: Option<sea_orm::QueryResult> = self
            .db
            .as_ref()
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [self.retention_days.into()],
            ))
            .await?;

        if let Some(row) = result {
            Ok(TrackingStats {
                total_page_views: row.try_get::<i64>("", "total_page_views").unwrap_or(0),
                expired_page_views: row.try_get::<i64>("", "expired_page_views").unwrap_or(0),
                total_user_behaviors: row
                    .try_get::<i64>("", "total_user_behaviors")
                    .unwrap_or(0),
                expired_user_behaviors: row
                    .try_get::<i64>("", "expired_user_behaviors")
                    .unwrap_or(0),
                oldest_page_view: row
                    .try_get::<String>("", "oldest_page_view")
                    .ok(),
                newest_page_view: row
                    .try_get::<String>("", "newest_page_view")
                    .ok(),
                oldest_user_behavior: row
                    .try_get::<String>("", "oldest_user_behavior")
                    .ok(),
                newest_user_behavior: row
                    .try_get::<String>("", "newest_user_behavior")
                    .ok(),
            })
        } else {
            Ok(TrackingStats::default())
        }
    }
}

/// 单轮清理结果统计
#[derive(Debug, Default, Clone, Serialize)]
pub struct CleanupStats {
    pub page_view_archived: i64,
    pub user_behavior_archived: i64,
}

/// 追踪数据保留统计信息
#[derive(Debug, Default)]
pub struct TrackingStats {
    pub total_page_views: i64,
    pub expired_page_views: i64,
    pub total_user_behaviors: i64,
    pub expired_user_behaviors: i64,
    pub oldest_page_view: Option<String>,
    pub newest_page_view: Option<String>,
    pub oldest_user_behavior: Option<String>,
    pub newest_user_behavior: Option<String>,
}
