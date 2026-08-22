//! 用户行为追踪分析相关 DTO
//!
//! 从 services/tracking_service.rs 迁移而来的纯数据结构，
//! 仅保留 derive 与字段定义，便于跨模块复用与统一管理。

use chrono::{DateTime, Utc};
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};

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
