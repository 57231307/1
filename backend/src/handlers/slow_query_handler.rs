//! 慢查询审计 Handler（P13 批 1 B-慢查询审计）
//!
//! 提供：
//! - GET /api/v1/erp/slow-queries         分页 + 多维筛选（时间范围 / 最小执行时间 / 关键词）
//! - GET /api/v1/erp/slow-queries/stats   TOP 10 聚合统计
//! - POST /api/v1/erp/slow-queries/refresh 手动触发一次采集

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, Statement,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::slow_query::{self, SlowQueryDto, SlowQueryStatDto};
use crate::services::slow_query_collector::SlowQueryCollector;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::sql_escape::safe_like_pattern;

/// 列表查询参数（全部可选）
// P1-14 修复（2026-06-25）：路由已挂载至 system::routes()，函数标记已移除。
// 结构体字段经 serde Deserialize 派生使用。
#[derive(Debug, Default, Deserialize)]
pub struct SlowQueryListParams {
    /// 起始时间（RFC3339 / ISO8601）
    pub start_time: Option<String>,
    /// 截止时间（RFC3339 / ISO8601）
    pub end_time: Option<String>,
    /// 最小执行时间（毫秒），仅返回 >= 此值的记录
    pub min_duration: Option<f64>,
    /// 关键词搜索（模糊匹配 query_text）
    pub keyword: Option<String>,
    /// 当前页（从 1 开始）
    pub page: Option<u64>,
    /// 每页条数
    pub page_size: Option<u64>,
}

/// 列表返回包装
// P1-14 修复（2026-06-25）：路由已挂载，字段经 serde Serialize 派生使用。
#[derive(Debug, Serialize)]
pub struct SlowQueryListResponse {
    pub items: Vec<SlowQueryDto>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 统计接口返回包装
// P1-14 修复（2026-06-25）：路由已挂载，字段经 serde Serialize 派生使用。
#[derive(Debug, Serialize)]
pub struct SlowQueryStatsResponse {
    /// TOP 10 列表（按最大平均执行时间倒序）
    pub top10: Vec<SlowQueryStatDto>,
    /// 慢查询总条数
    pub total_count: u64,
    /// 采集时间范围描述（"近 7 天"等）
    pub time_range: String,
}

/// 手动刷新接口返回
// P1-14 修复（2026-06-25）：路由已挂载，字段经 serde Serialize 派生使用。
#[derive(Debug, Serialize)]
pub struct SlowQueryRefreshResponse {
    /// 本次采集写入条数
    pub inserted: usize,
    /// 提示信息
    pub message: String,
}

/// GET /api/v1/erp/slow-queries；分页 + 多维筛选（时间范围 / 最小执行时间 / 关键词）
pub async fn list_slow_queries(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<SlowQueryListParams>,
) -> Result<Json<ApiResponse<SlowQueryListResponse>>, AppError> {
    // 防御式分页参数：unwrap_or(1).max(1) 显式调用 Ord::max 避免 ExprTrait 歧义
    let page = std::cmp::Ord::max(params.page.unwrap_or(1), 1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let mut q = slow_query::Entity::find();

    // 时间范围筛选
    if let Some(start) = &params.start_time {
        if let Ok(ts) = start.parse::<DateTime<Utc>>() {
            q = q.filter(slow_query::Column::CapturedAt.gte(ts.naive_utc()));
        }
    }
    if let Some(end) = &params.end_time {
        if let Ok(ts) = end.parse::<DateTime<Utc>>() {
            q = q.filter(slow_query::Column::CapturedAt.lte(ts.naive_utc()));
        }
    }

    // 最小执行时间
    if let Some(min_dur) = params.min_duration {
        q = q.filter(slow_query::Column::ExecutionTimeMs.gte(min_dur));
    }

    // 关键词模糊搜索
    if let Some(kw) = &params.keyword {
        if !kw.trim().is_empty() {
            // 批次 94 P2-3 修复：LIKE 模式注入，转义 % _ \ 特殊字符
            let pattern = safe_like_pattern(kw);
            q = q.filter(slow_query::Column::QueryText.like(pattern));
        }
    }

    let paginator = q
        .order_by_desc(slow_query::Column::ExecutionTimeMs)
        .paginate(state.db.as_ref(), page_size);

    let total = paginator
        .num_items()
        .await
        .map_err(|e| AppError::internal(format!("统计慢查询失败: {}", e)))?;
    let logs = paginator
        // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
        .fetch_page(page.clamp(1, 1000).saturating_sub(1))
        .await
        .map_err(|e| AppError::internal(format!("查询慢查询失败: {}", e)))?;

    let items: Vec<SlowQueryDto> = logs.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(SlowQueryListResponse {
        items,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/slow-queries/stats；聚合统计：按 query_text 分组，TOP 10（按最大平均执行时间倒序）
pub async fn get_slow_query_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SlowQueryStatsResponse>>, AppError> {
    // 使用原生 SQL 聚合：按 query_text 分组，取 max(execution_time_ms) / sum(calls) / avg(rows)
    // 仅取近 7 天数据，避免历史数据爆炸
    // B03-P2-5 修复：SQL 为静态常量，无用户输入拼接，from_string 安全（无注入风险）
    let sql = "SELECT query_text, \
                      MAX(execution_time_ms) as max_exec_time_ms, \
                      SUM(calls) as total_calls, \
                      AVG(rows_examined) as avg_rows, \
                      COUNT(*) as sample_count \
               FROM slow_query_log \
               WHERE captured_at >= NOW() - INTERVAL '7 days' \
               GROUP BY query_text \
               ORDER BY max_exec_time_ms DESC \
               LIMIT 10";

    let query_result = state
        .db
        .as_ref()
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询慢查询统计失败: {}", e)))?;

    let mut top10: Vec<SlowQueryStatDto> = Vec::with_capacity(query_result.len());
    for row in query_result {
        let query_text: Option<String> = row.try_get_by_index(0).ok();
        let max_exec_time_ms: Option<f64> = row.try_get_by_index(1).ok();
        let total_calls: Option<i64> = row.try_get_by_index(2).ok();
        let avg_rows: Option<f64> = row.try_get_by_index(3).ok();
        let sample_count: Option<i64> = row.try_get_by_index(4).ok();

        if let Some(qt) = query_text {
            top10.push(SlowQueryStatDto {
                query_text: qt,
                max_exec_time_ms: max_exec_time_ms.unwrap_or(0.0),
                total_calls: total_calls.unwrap_or(0),
                avg_rows: avg_rows.unwrap_or(0.0),
                sample_count: sample_count.unwrap_or(0),
            });
        }
    }

    // 总条数（近 7 天）
    // B03-P2-5 修复：count_sql 为静态常量，无用户输入拼接，from_string 安全（无注入风险）
    let count_sql =
        "SELECT COUNT(*) FROM slow_query_log WHERE captured_at >= NOW() - INTERVAL '7 days'";
    let count_row = state
        .db
        .as_ref()
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            count_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("统计慢查询总数失败: {}", e)))?;
    let total_count: i64 = count_row
        .and_then(|r| r.try_get_by_index(0).ok())
        .unwrap_or(0);

    Ok(Json(ApiResponse::success(SlowQueryStatsResponse {
        top10,
        // 显式调用 std::cmp::Ord::max 避免与 migration::ExprTrait::max 冲突
        total_count: std::cmp::Ord::max(total_count, 0) as u64,
        time_range: "近 7 天".to_string(),
    })))
}

/// batch-17 P3: 慢查询摘要数据
#[derive(Debug, Serialize)]
pub struct SlowQuerySummary {
    pub total_queries: i64,
    pub queries_today: i64,
    pub avg_execution_time: f64,
    pub max_execution_time: f64,
    pub most_frequent_query: Option<String>,
    pub optimization_status: OptimizationStatusSummary,
}

/// 优化状态摘要
#[derive(Debug, Serialize)]
pub struct OptimizationStatusSummary {
    pub pending: i64,
    pub in_progress: i64,
    pub optimized: i64,
    pub ignored: i64,
}

/// GET /api/v1/erp/slow-queries/summary - 慢查询摘要
pub async fn get_slow_query_summary(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SlowQuerySummary>>, AppError> {
    // 查询总数
    let total_sql = "SELECT COUNT(*) as total FROM slow_queries";
    let total_result = state
        .db
        .as_ref()
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            total_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询慢查询总数失败: {}", e)))?;
    let total_queries = total_result
        .and_then(|r| r.try_get::<i64>("", "total").ok())
        .unwrap_or(0);

    // 查询今日新增
    let today_sql = "SELECT COUNT(*) as today FROM slow_queries WHERE captured_at >= CURRENT_DATE";
    let today_result = state
        .db
        .as_ref()
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            today_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询今日慢查询失败: {}", e)))?;
    let queries_today = today_result
        .and_then(|r| r.try_get::<i64>("", "today").ok())
        .unwrap_or(0);

    // 查询平均执行时间
    let avg_sql = "SELECT COALESCE(AVG(mean_exec_time), 0) as avg_time FROM slow_queries";
    let avg_result = state
        .db
        .as_ref()
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            avg_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询平均执行时间失败: {}", e)))?;
    let avg_execution_time = avg_result
        .and_then(|r| r.try_get::<f64>("", "avg_time").ok())
        .unwrap_or(0.0);

    // 查询最大执行时间
    let max_sql = "SELECT COALESCE(MAX(max_exec_time), 0) as max_time FROM slow_queries";
    let max_result = state
        .db
        .as_ref()
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            max_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询最大执行时间失败: {}", e)))?;
    let max_execution_time = max_result
        .and_then(|r| r.try_get::<f64>("", "max_time").ok())
        .unwrap_or(0.0);

    // 查询最频繁的查询
    let frequent_sql = "SELECT query_text FROM slow_queries GROUP BY query_text ORDER BY COUNT(*) DESC LIMIT 1";
    let frequent_result = state
        .db
        .as_ref()
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            frequent_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询最频繁查询失败: {}", e)))?;
    let most_frequent_query = frequent_result
        .and_then(|r| r.try_get::<String>("", "query_text").ok());

    // 查询优化状态统计
    let status_sql = "SELECT \
        COUNT(CASE WHEN optimization_status IS NULL OR optimization_status = 'pending' THEN 1 END) as pending, \
        COUNT(CASE WHEN optimization_status = 'in_progress' THEN 1 END) as in_progress, \
        COUNT(CASE WHEN optimization_status = 'optimized' THEN 1 END) as optimized, \
        COUNT(CASE WHEN optimization_status = 'ignored' THEN 1 END) as ignored \
        FROM slow_queries";
    let status_result = state
        .db
        .as_ref()
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            status_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询优化状态失败: {}", e)))?;

    let optimization_status = OptimizationStatusSummary {
        pending: status_result
            .as_ref()
            .and_then(|r| r.try_get::<i64>("", "pending").ok())
            .unwrap_or(0),
        in_progress: status_result
            .as_ref()
            .and_then(|r| r.try_get::<i64>("", "in_progress").ok())
            .unwrap_or(0),
        optimized: status_result
            .as_ref()
            .and_then(|r| r.try_get::<i64>("", "optimized").ok())
            .unwrap_or(0),
        ignored: status_result
            .as_ref()
            .and_then(|r| r.try_get::<i64>("", "ignored").ok())
            .unwrap_or(0),
    };

    let summary = SlowQuerySummary {
        total_queries,
        queries_today,
        avg_execution_time,
        max_execution_time,
        most_frequent_query,
        optimization_status,
    };

    Ok(Json(ApiResponse::success(summary)))
}

/// POST /api/v1/erp/slow-queries/refresh；手动触发一次慢查询采集（用于前端"刷新"按钮）；返回：插入条数 + 提示信息
pub async fn refresh_slow_queries(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SlowQueryRefreshResponse>>, AppError> {
    // batch-17 P3: 使用默认阈值配置（100ms / 100 rows）
    // TODO: 从 AppState.settings 读取配置，需要将 settings 存入 AppState
    let collector = Arc::new(SlowQueryCollector::new(state.db.clone(), 100.0, 100));

    let inserted = collector.collect_once().await.map_err(|e| {
        // pg_stat_statements 不可用时返回友好提示
        let msg = e.to_string();
        if msg.contains("does not exist") || msg.contains("pg_stat_statements") {
            AppError::internal("pg_stat_statements 扩展不可用，请联系管理员启用".to_string())
        } else {
            AppError::internal(format!("手动采集慢查询失败: {}", e))
        }
    })?;

    let message = if inserted == 0 {
        "本次未发现新的慢查询（最近 5 分钟内无 mean_exec_time > 100ms 的查询）".to_string()
    } else {
        format!("本次采集写入 {} 条慢查询记录", inserted)
    };

    Ok(Json(ApiResponse::success(SlowQueryRefreshResponse {
        inserted,
        message,
    })))
}

/// V15 P2 20.5-C：更新慢查询优化状态请求
#[derive(Debug, Deserialize)]
pub struct UpdateOptimizationRequest {
    /// 优化状态（pending/in_progress/resolved/wont_fix）
    pub optimization_status: Option<String>,
    /// 负责人
    pub assigned_to: Option<String>,
    /// Jira 工单号
    pub jira_ticket: Option<String>,
}

/// V15 P2 20.5-C：更新慢查询优化状态
///
/// PUT /api/v1/erp/slow-queries/:id/optimization
///
/// 允许管理员更新慢查询的优化状态、负责人和 Jira 工单号。
pub async fn update_slow_query_optimization(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i64>,
    Json(req): Json<UpdateOptimizationRequest>,
) -> Result<Json<ApiResponse<SlowQueryDto>>, AppError> {
    // 校验 optimization_status 合法性
    if let Some(status) = &req.optimization_status {
        let valid_statuses = ["pending", "in_progress", "resolved", "wont_fix"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(AppError::bad_request(format!(
                "optimization_status 必须是 {:?} 之一",
                valid_statuses
            )));
        }
    }

    // 查询现有记录
    let existing = slow_query::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询慢查询记录失败: {}", e)))?
        .ok_or_else(|| AppError::not_found(format!("慢查询记录 {} 不存在", id)))?;

    // 更新字段
    let mut active: slow_query::ActiveModel = existing.into();
    if let Some(status) = req.optimization_status {
        active.optimization_status = Set(Some(status));
    }
    if let Some(assigned) = req.assigned_to {
        active.assigned_to = Set(Some(assigned));
    }
    if let Some(ticket) = req.jira_ticket {
        active.jira_ticket = Set(Some(ticket));
    }

    let updated = active
        .update(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("更新慢查询优化状态失败: {}", e)))?;

    Ok(Json(ApiResponse::success(updated.into())))
}

/// batch-17 P3: 慢查询周报查询参数
#[derive(Debug, Deserialize)]
pub struct WeeklyReportQuery {
    pub weeks: Option<u32>,
}

/// batch-17 P3: 慢查询周报数据
#[derive(Debug, Serialize)]
pub struct SlowQueryWeeklyReport {
    pub week_start: String,
    pub week_end: String,
    pub total_queries: i64,
    pub new_queries: i64,
    pub optimized_queries: i64,
    pub avg_execution_time: f64,
    pub top_queries: Vec<SlowQueryStatDto>,
}

/// GET /api/v1/erp/slow-queries/report/weekly - 慢查询周报
pub async fn get_weekly_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<WeeklyReportQuery>,
) -> Result<Json<ApiResponse<SlowQueryWeeklyReport>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "慢查询周报查询");

    let weeks = query.weeks.unwrap_or(1);
    let now = chrono::Utc::now();
    let week_start = now - chrono::Duration::weeks(weeks as i64);

    // 查询本周慢查询统计
    let total_queries = slow_query::Entity::find()
        .filter(slow_query::Column::CapturedAt.gte(week_start))
        .count(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询慢查询总数失败: {}", e)))?;

    // 查询新增慢查询（本周首次出现的）
    let new_queries = slow_query::Entity::find()
        .filter(slow_query::Column::CapturedAt.gte(week_start))
        .filter(slow_query::Column::OptimizationStatus.is_null())
        .count(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询新增慢查询失败: {}", e)))?;

    // 查询已优化的慢查询
    let optimized_queries = slow_query::Entity::find()
        .filter(slow_query::Column::CapturedAt.gte(week_start))
        .filter(slow_query::Column::OptimizationStatus.eq("optimized"))
        .count(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询已优化慢查询失败: {}", e)))?;

    // 查询平均执行时间
    let avg_sql = format!(
        "SELECT COALESCE(AVG(mean_exec_time), 0) as avg_time FROM slow_queries WHERE captured_at >= '{}'",
        week_start.format("%Y-%m-%d %H:%M:%S")
    );
    let avg_result = state
        .db
        .as_ref()
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            avg_sql,
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询平均执行时间失败: {}", e)))?;
    let avg_execution_time = avg_result
        .map(|r| r.try_get::<f64>("", "avg_time").unwrap_or(0.0))
        .unwrap_or(0.0);

    // 查询 TOP 10 慢查询
    let top_queries_raw = slow_query::Entity::find()
        .filter(slow_query::Column::CapturedAt.gte(week_start))
        .order_by_desc(slow_query::Column::ExecutionTimeMs)
        .limit(10)
        .all(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询 TOP 慢查询失败: {}", e)))?;

    let top_queries: Vec<SlowQueryStatDto> = top_queries_raw
        .into_iter()
        .map(|q| SlowQueryStatDto {
            query_text: q.query_text,
            max_exec_time_ms: q.execution_time_ms,
            total_calls: q.calls,
            avg_rows: q.rows_examined as f64,
            sample_count: 1,
        })
        .collect();

    let report = SlowQueryWeeklyReport {
        week_start: week_start.format("%Y-%m-%d").to_string(),
        week_end: now.format("%Y-%m-%d").to_string(),
        total_queries,
        new_queries,
        optimized_queries,
        avg_execution_time,
        top_queries,
    };

    Ok(Json(ApiResponse::success(report)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 列表查询参数默认值
    #[test]
    fn test_list_params_default() {
        let p = SlowQueryListParams::default();
        assert!(p.start_time.is_none());
        assert!(p.end_time.is_none());
        assert!(p.min_duration.is_none());
        assert!(p.keyword.is_none());
        assert!(p.page.is_none());
        assert!(p.page_size.is_none());
    }
}
