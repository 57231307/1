//! 慢查询后台采集服务（P13 批 1 B-慢查询审计）
//!
//! 职责：
//! 1. 启动一个后台任务（`start_collect_task`），每 5 分钟（可通过 `interval_secs` 调整）
//!    触发一次采集周期
//! 2. 在每个周期内查询 `pg_stat_statements` 视图，过滤 `mean_exec_time > 100ms` 的 SQL，
//!    写入 `slow_query_log` 表
//! 3. 提供手动触发函数 `collect_once`，供 handler 端"刷新"按钮使用
//!
//! 关键设计：
//! - **降级方案**：CI/容器环境如未预装 `pg_stat_statements` 共享库，采集 SQL 会失败；
//!   失败时仅记录 `tracing::warn!`，不向上传播，不阻断 main 启动
//! - **系统级数据**：慢查询为系统级数据，采集到的记录全局可见
//! - **不阻塞 main**：使用 `tokio::spawn` 启动后台任务，采集失败仅记录日志
//!
//! 关联文档：[2026-06-18-p13-batch1-comprehensive-plan.md §2.2]

use chrono::Utc;
use futures::FutureExt;
use sea_orm::{ActiveValue, ConnectionTrait, DatabaseConnection, EntityTrait, Set, Statement};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use crate::models::slow_query;

/// 慢查询采集服务
///
/// 内部状态：仅持有 `Arc<DatabaseConnection>`，无业务缓存（采集数据全部入表）
pub struct SlowQueryCollector {
    /// 数据库连接（Arc 共享给后台任务）
    db: Arc<DatabaseConnection>,
    /// 采集阈值（毫秒），仅采集 `mean_exec_time > threshold` 的 SQL
    threshold_ms: f64,
    /// 单次采集最大行数（防止极端情况下表爆炸）
    limit_rows: i64,
}

/// 构造 `pg_stat_statements` 查询 SQL（独立函数便于单元测试）
///
/// 输入：threshold_ms（毫秒）/ limit_rows（最大返回行数）
/// 输出：完整 SQL 字符串
///
/// 设计说明：threshold 与 limit 均为数值类型（f64 / i64），
/// 不接受用户输入，无 SQL 注入风险。
pub fn build_query_sql(threshold_ms: f64, limit_rows: i64) -> String {
    format!(
        "SELECT query, mean_exec_time, calls, rows \
         FROM pg_stat_statements \
         WHERE mean_exec_time > {} \
         ORDER BY mean_exec_time DESC \
         LIMIT {}",
        threshold_ms, limit_rows
    )
}

impl SlowQueryCollector {
    /// 创建采集服务实例
    ///
    /// 参数：
    /// - `db`：数据库连接（Arc 包装）
    /// - `threshold_ms`：慢查询阈值（毫秒），默认 100ms（与 plan 一致）
    /// - `limit_rows`：单次采集最大行数，默认 100（与 plan 一致）
    pub fn new(db: Arc<DatabaseConnection>, threshold_ms: f64, limit_rows: i64) -> Self {
        Self {
            db,
            threshold_ms,
            limit_rows,
        }
    }

    /// 启动后台定时采集任务
    ///
    /// 行为：
    /// - 启动后立即执行一次采集（首屏不等待）
    /// - 之后每 `interval_secs` 秒执行一次
    /// - 任何异常仅记录 `tracing::warn!`，不向上传播
    ///
    /// 设计原则：采集任务启动失败不阻断 main（CI 容器可能未预装扩展）
    #[allow(dead_code)] // TODO(tech-debt): 启动入口供 main.rs 调用，预留 API
    pub fn start_collect_task(self: Arc<Self>, interval_secs: u64) {
        let service = self.clone();
        tokio::spawn(async move {
            tracing::info!(
                "慢查询采集任务已启动（间隔 {} 秒，阈值 {}ms）",
                interval_secs,
                service.threshold_ms
            );
            // 批次 7（2026-06-28）：定义 panic 隔离采集闭包
            // 慢查询采集是长期循环任务，若 panic 会导致慢查询审计功能永久失效，
            // 无法定位性能问题。确保单次 panic 不退出循环。
            let run_collect = || async {
                AssertUnwindSafe(async {
                    if let Err(e) = service.collect_once().await {
                        tracing::warn!("慢查询采集失败: {}", e);
                    }
                })
                .catch_unwind()
                .await
            };
            // 首次立即执行（首屏不等待）
            if let Err(panic_payload) = run_collect().await {
                let panic_msg = panic_payload
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
                    .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                    .unwrap_or("<非字符串 panic payload>");
                tracing::warn!(
                    panic = %panic_msg,
                    "慢查询首屏采集 panic 已被隔离（可能是 pg_stat_statements 不可用）"
                );
            }
            // 定时循环
            let mut ticker = interval(Duration::from_secs(interval_secs));
            // 跳过首次立即触发（已手动执行）
            ticker.tick().await;
            loop {
                ticker.tick().await;
                if let Err(panic_payload) = run_collect().await {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ 慢查询采集 spawn 任务内 panic 已被隔离，采集循环继续运行（不退出）"
                    );
                }
            }
        });
    }

    /// 执行一次采集（手动触发 / 定时调用）
    ///
    /// 流程：
    /// 1. 查询 `pg_stat_statements` 视图（按 mean_exec_time 倒序，过滤阈值）
    /// 2. 解析结果，写入 `slow_query_log` 表
    /// 3. 返回写入条数（便于 handler 端反馈）
    ///
    /// 错误处理：所有错误向上传播，由调用方决定是否降级
    #[allow(dead_code)] // TODO(tech-debt): 手动触发入口供 handler 调用，预留 API
    pub async fn collect_once(&self) -> Result<usize, sea_orm::DbErr> {
        // 使用 build_query_sql 拼接 SQL（便于单元测试覆盖）
        let sql = build_query_sql(self.threshold_ms, self.limit_rows);

        let query_result: Vec<sea_orm::QueryResult> = self
            .db
            .as_ref()
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            ))
            .await?;

        let mut inserted = 0usize;
        for row in query_result {
            // 解析字段：query(text) / mean_exec_time(float8) / calls(int8) / rows(int8)
            // 使用 try_get_by_index 防御式读取：缺字段时跳过该行
            let query_text: Option<String> = row.try_get_by_index::<String>(0).ok();
            let mean_exec_time: Option<f64> = row.try_get_by_index::<f64>(1).ok();
            let calls: Option<i64> = row.try_get_by_index::<i64>(2).ok();
            let rows_examined: Option<i64> = row.try_get_by_index::<i64>(3).ok();

            // 跳过空查询（pg_stat_statements 偶尔会插入空字符串占位）
            let query_text = match query_text {
                Some(q) if !q.trim().is_empty() => q,
                _ => continue,
            };
            // 数值字段缺值时记 0（不影响主流程）
            let mean_exec_time = mean_exec_time.unwrap_or(0.0);
            let calls = calls.unwrap_or(0);
            let rows_examined = rows_examined.unwrap_or(0);

            let active = slow_query::ActiveModel {
                id: ActiveValue::NotSet,
                query_text: Set(query_text),
                execution_time_ms: Set(mean_exec_time),
                calls: Set(calls),
                rows_examined: Set(rows_examined),
                // 数据库名：系统级元数据（暂留空，前端 stats 接口会标注"系统"）
                database_name: Set(None),
                captured_at: Set(Utc::now()),
            };

            // 使用 exec_without_returning 避免 last_insert_id 解析问题（参考 omni_audit_service）
            if let Err(e) = slow_query::Entity::insert(active)
                .exec_without_returning(self.db.as_ref())
                .await
            {
                tracing::warn!("写入慢查询日志失败: {}", e);
                // 单条失败不阻断后续插入
                continue;
            }
            inserted += 1;
        }

        if inserted > 0 {
            tracing::info!("本次慢查询采集写入 {} 条记录", inserted);
        }
        Ok(inserted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SQL 拼接：默认阈值与 limit
    #[test]
    fn test_build_query_sql_default() {
        let sql = build_query_sql(100.0, 100);
        assert!(sql.contains("WHERE mean_exec_time > 100"));
        assert!(sql.contains("LIMIT 100"));
        assert!(sql.contains("ORDER BY mean_exec_time DESC"));
        assert!(sql.starts_with("SELECT query, mean_exec_time, calls, rows"));
        assert!(sql.contains("FROM pg_stat_statements"));
    }

    /// SQL 拼接：自定义阈值与 limit
    #[test]
    fn test_build_query_sql_custom() {
        let sql = build_query_sql(250.5, 50);
        assert!(sql.contains("> 250.5"));
        assert!(sql.contains("LIMIT 50"));
    }

    /// SQL 拼接：极值（验证无溢出/格式异常）
    #[test]
    fn test_build_query_sql_extreme_values() {
        // 极小
        let sql_min = build_query_sql(0.001, 1);
        assert!(sql_min.contains("> 0.001"));
        assert!(sql_min.contains("LIMIT 1"));

        // 极大
        let sql_max = build_query_sql(1_000_000.0, 1_000_000);
        assert!(sql_max.contains("> 1000000"));
        assert!(sql_max.contains("LIMIT 1000000"));

        // 零值
        let sql_zero = build_query_sql(0.0, 0);
        assert!(sql_zero.contains("> 0"));
        assert!(sql_zero.contains("LIMIT 0"));
    }
}
