//! 用户行为追踪分析服务
//!
//! v11 批次 143 P1-2：真实实现追踪分析功能
//!
//! 提供：
//! - 页面访问记录（持久化到 page_views 表）
//! - 页面访问统计（总量 / 独立会话 / 按日聚合）
//! - 热门页面排行
//! - 用户行为记录（持久化到 user_behaviors 表）
//! - 漏斗分析（按 session_id 统计完成指定步骤序列的会话数）
//! - 用户路径分析（按 session_id 还原访问路径）

use crate::models::{page_view, user_behavior};
use crate::utils::error::AppError;
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseBackend, DatabaseConnection, EntityTrait,
    FromQueryResult, QueryFilter, QueryOrder, Set, Statement, Value,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 页面访问记录请求
#[derive(Debug, Deserialize)]
pub struct PageViewInput {
    pub path: String,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub user_id: Option<i32>,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

/// 用户行为记录请求
#[derive(Debug, Deserialize)]
pub struct BehaviorInput {
    pub event_type: String,
    pub event_target: Option<String>,
    pub event_data: Option<serde_json::Value>,
    pub path: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<i32>,
    pub ip_address: Option<String>,
}

/// 统计查询参数
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// 起始日期（YYYY-MM-DD 或 ISO 8601）
    pub date_from: Option<String>,
    /// 结束日期（YYYY-MM-DD 或 ISO 8601）
    pub date_to: Option<String>,
}

/// 页面访问统计响应
#[derive(Debug, Serialize, FromQueryResult)]
pub struct PageViewStats {
    pub total_views: i64,
    pub unique_sessions: i64,
    pub unique_paths: i64,
}

/// 按日统计响应
#[derive(Debug, Serialize, FromQueryResult)]
pub struct DailyStats {
    pub stat_date: String,
    pub total_views: i64,
    pub unique_sessions: i64,
}

/// 热门页面响应
#[derive(Debug, Serialize, FromQueryResult)]
pub struct PopularPage {
    pub path: String,
    pub view_count: i64,
    pub unique_sessions: i64,
}

/// 漏斗步骤
#[derive(Debug, Deserialize)]
pub struct FunnelQuery {
    /// 漏斗步骤路径序列（按顺序匹配）
    pub steps: Vec<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

/// 漏斗分析响应
#[derive(Debug, Serialize)]
pub struct FunnelAnalysis {
    pub steps: Vec<String>,
    pub step_counts: Vec<i64>,
    pub conversion_rates: Vec<f64>,
}

/// 用户路径查询
#[derive(Debug, Deserialize)]
pub struct UserPathQuery {
    pub session_id: String,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

/// 用户路径节点
#[derive(Debug, Serialize, FromQueryResult)]
pub struct UserPathNode {
    pub path: String,
    pub viewed_at: DateTime<Utc>,
}

/// 会话 ID 查询结果（用于漏斗分析中间步骤）
#[derive(Debug, FromQueryResult)]
struct SessionIdRow {
    session_id: String,
}

#[derive(Debug, Clone)]
pub struct TrackingService {
    db: Arc<DatabaseConnection>,
}

impl TrackingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 记录页面访问
    pub async fn record_page_view(&self, input: PageViewInput) -> Result<(), AppError> {
        let viewed_at: DateTime<Utc> = input
            .timestamp
            .parse::<DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());

        let active = page_view::ActiveModel {
            id: Default::default(),
            session_id: Set(input.session_id),
            user_id: Set(input.user_id),
            path: Set(input.path),
            referrer: Set(input.referrer),
            user_agent: Set(input.user_agent),
            ip_address: Set(input.ip_address),
            viewed_at: Set(viewed_at),
        };
        active.insert(&*self.db).await?;
        Ok(())
    }

    /// 记录用户行为
    pub async fn record_behavior(&self, input: BehaviorInput) -> Result<(), AppError> {
        let active = user_behavior::ActiveModel {
            id: Default::default(),
            session_id: Set(input.session_id),
            user_id: Set(input.user_id),
            event_type: Set(input.event_type),
            event_target: Set(input.event_target),
            event_data: Set(input.event_data),
            path: Set(input.path),
            ip_address: Set(input.ip_address),
            occurred_at: Set(Utc::now()),
        };
        active.insert(&*self.db).await?;
        Ok(())
    }

    /// 获取页面访问统计（总量）
    pub async fn get_page_view_stats(
        &self,
        date_from: Option<DateTime<Utc>>,
        date_to: Option<DateTime<Utc>>,
    ) -> Result<PageViewStats, AppError> {
        let mut sql = String::from(
            "SELECT COUNT(*) as total_views, \
             COUNT(DISTINCT session_id) as unique_sessions, \
             COUNT(DISTINCT path) as unique_paths \
             FROM page_views WHERE 1=1",
        );
        let mut params: Vec<Value> = Vec::new();
        if let Some(from) = date_from {
            sql.push_str(" AND viewed_at >= $1");
            params.push(from.into());
        }
        if let Some(to) = date_to {
            sql.push_str(&format!(" AND viewed_at < ${}", params.len() + 1));
            params.push(to.into());
        }

        let stats = PageViewStats::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            &sql,
            params,
        ))
        .one(&*self.db)
        .await?
        .unwrap_or(PageViewStats {
            total_views: 0,
            unique_sessions: 0,
            unique_paths: 0,
        });
        Ok(stats)
    }

    /// 获取按日统计
    pub async fn get_daily_stats(
        &self,
        date_from: Option<DateTime<Utc>>,
        date_to: Option<DateTime<Utc>>,
    ) -> Result<Vec<DailyStats>, AppError> {
        let mut sql = String::from(
            "SELECT \
             TO_CHAR(DATE(viewed_at), 'YYYY-MM-DD') as stat_date, \
             COUNT(*) as total_views, \
             COUNT(DISTINCT session_id) as unique_sessions \
             FROM page_views WHERE 1=1",
        );
        let mut params: Vec<Value> = Vec::new();
        if let Some(from) = date_from {
            sql.push_str(" AND viewed_at >= $1");
            params.push(from.into());
        }
        if let Some(to) = date_to {
            sql.push_str(&format!(" AND viewed_at < ${}", params.len() + 1));
            params.push(to.into());
        }
        sql.push_str(" GROUP BY DATE(viewed_at) ORDER BY stat_date ASC");

        let stats = DailyStats::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            &sql,
            params,
        ))
        .all(&*self.db)
        .await?;
        Ok(stats)
    }

    /// 获取热门页面
    pub async fn get_popular_pages(
        &self,
        limit: u64,
        date_from: Option<DateTime<Utc>>,
        date_to: Option<DateTime<Utc>>,
    ) -> Result<Vec<PopularPage>, AppError> {
        let mut sql = String::from(
            "SELECT path, COUNT(*) as view_count, \
             COUNT(DISTINCT session_id) as unique_sessions \
             FROM page_views WHERE 1=1",
        );
        let mut params: Vec<Value> = Vec::new();
        if let Some(from) = date_from {
            sql.push_str(" AND viewed_at >= $1");
            params.push(from.into());
        }
        if let Some(to) = date_to {
            sql.push_str(&format!(" AND viewed_at < ${}", params.len() + 1));
            params.push(to.into());
        }
        sql.push_str(" GROUP BY path ORDER BY view_count DESC LIMIT ");
        sql.push_str(&limit.to_string());

        let pages = PopularPage::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            &sql,
            params,
        ))
        .all(&*self.db)
        .await?;
        Ok(pages)
    }

    /// 漏斗分析
    ///
    /// 对每个步骤，统计在指定时间范围内按 session_id 顺序访问过步骤 1..=N 的会话数。
    /// 转化率 = step_counts[i] / step_counts[0] * 100%
    pub async fn get_funnel_analysis(
        &self,
        query: FunnelQuery,
    ) -> Result<FunnelAnalysis, AppError> {
        if query.steps.is_empty() {
            return Err(AppError::validation("漏斗步骤不能为空"));
        }
        let date_from = parse_date(&query.date_from)?;
        let date_to = parse_date(&query.date_to)?;

        let mut step_counts = Vec::with_capacity(query.steps.len());
        for i in 0..query.steps.len() {
            // 统计按时间顺序访问过 steps[0..=i] 的会话数
            let steps_slice = &query.steps[..=i];
            let count = self.count_sessions_with_sequence(steps_slice, date_from, date_to).await?;
            step_counts.push(count);
        }

        let first_count = step_counts[0].max(1) as f64;
        let conversion_rates: Vec<f64> = step_counts
            .iter()
            .map(|&c| ((c as f64 / first_count) * 100.0).round() / 100.0)
            .collect();

        Ok(FunnelAnalysis {
            steps: query.steps,
            step_counts,
            conversion_rates,
        })
    }

    /// 统计在时间范围内访问过指定路径序列的会话数
    ///
    /// 实现策略：对每个路径，查询有访问记录的 session_id 集合，
    /// 取交集（后续步骤的 session 必须也出现在前序步骤中）。
    async fn count_sessions_with_sequence(
        &self,
        steps: &[String],
        date_from: Option<DateTime<Utc>>,
        date_to: Option<DateTime<Utc>>,
    ) -> Result<i64, AppError> {
        if steps.is_empty() {
            return Ok(0);
        }

        // 逐步缩小 session 集合：第一步取所有访问过 steps[0] 的 session，
        // 后续步骤在上一步集合基础上筛选
        let mut current_sessions: Option<Vec<String>> = None;
        for step in steps {
            let mut sql = String::from(
                "SELECT DISTINCT session_id FROM page_views WHERE path = $1 AND session_id IS NOT NULL",
            );
            let mut params: Vec<Value> = vec![step.clone().into()];
            if let Some(from) = date_from {
                sql.push_str(&format!(" AND viewed_at >= ${}", params.len() + 1));
                params.push(from.into());
            }
            if let Some(to) = date_to {
                sql.push_str(&format!(" AND viewed_at < ${}", params.len() + 1));
                params.push(to.into());
            }

            let sessions: Vec<String> = SessionIdRow::find_by_statement(
                Statement::from_sql_and_values(DatabaseBackend::Postgres, &sql, params),
            )
            .all(&*self.db)
            .await?
            .into_iter()
            .map(|r| r.session_id)
            .collect();

            current_sessions = Some(match current_sessions {
                None => sessions,
                Some(prev) => prev
                    .into_iter()
                    .filter(|s| sessions.contains(s))
                    .collect(),
            });
        }

        Ok(current_sessions.map(|s| s.len() as i64).unwrap_or(0))
    }

    /// 获取用户路径
    pub async fn get_user_path(
        &self,
        session_id: &str,
        date_from: Option<DateTime<Utc>>,
        date_to: Option<DateTime<Utc>>,
    ) -> Result<Vec<UserPathNode>, AppError> {
        let mut query = page_view::Entity::find()
            .filter(page_view::Column::SessionId.eq(session_id))
            .order_by_asc(page_view::Column::ViewedAt);
        if let Some(from) = date_from {
            query = query.filter(page_view::Column::ViewedAt.gte(from));
        }
        if let Some(to) = date_to {
            query = query.filter(page_view::Column::ViewedAt.lt(to));
        }
        let views = query.all(&*self.db).await?;
        Ok(views
            .into_iter()
            .map(|v| UserPathNode {
                path: v.path,
                viewed_at: v.viewed_at,
            })
            .collect())
    }
}

/// 解析日期字符串为 DateTime<Utc>
fn parse_date(s: &Option<String>) -> Result<Option<DateTime<Utc>>, AppError> {
    match s {
        Some(s) => {
            // 尝试解析为完整日期时间，失败则按日期解析并补充 00:00:00 UTC
            let dt = s
                .parse::<DateTime<Utc>>()
                .or_else(|_| {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap(/* 不变量：0,0,0 永远合法 */).and_utc())
                })
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?;
            Ok(Some(dt))
        }
        None => Ok(None),
    }
}
