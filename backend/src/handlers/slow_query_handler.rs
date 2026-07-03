//! 慢查询审计 Handler（P13 批 1 B-慢查询审计）
//!
//! 提供：
//! - GET /api/v1/erp/slow-queries         分页 + 多维筛选（时间范围 / 最小执行时间 / 关键词）
//! - GET /api/v1/erp/slow-queries/stats   TOP 10 聚合统计
//! - POST /api/v1/erp/slow-queries/refresh 手动触发一次采集

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::middleware::auth_context::AuthContext;
use crate::models::slow_query::{self, SlowQueryDto, SlowQueryStatDto};
use crate::services::slow_query_collector::SlowQueryCollector;
use crate::utils::app_state::AppState;
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

/// GET /api/v1/erp/slow-queries
///
/// 分页 + 多维筛选（时间范围 / 最小执行时间 / 关键词）
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
        .fetch_page(page.saturating_sub(1))
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

/// GET /api/v1/erp/slow-queries/stats
///
/// 聚合统计：按 query_text 分组，TOP 10（按最大平均执行时间倒序）
pub async fn get_slow_query_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SlowQueryStatsResponse>>, AppError> {
    // 使用原生 SQL 聚合：按 query_text 分组，取 max(execution_time_ms) / sum(calls) / avg(rows)
    // 仅取近 7 天数据，避免历史数据爆炸
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

/// POST /api/v1/erp/slow-queries/refresh
///
/// 手动触发一次慢查询采集（用于前端"刷新"按钮）
///
/// 返回：插入条数 + 提示信息
pub async fn refresh_slow_queries(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SlowQueryRefreshResponse>>, AppError> {
    // 构造采集器（与 main 启动参数一致：阈值 100ms / limit 100）
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
