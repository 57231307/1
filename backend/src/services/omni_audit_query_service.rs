use serde::{Deserialize, Serialize};

/// 审计查询过滤条件（供 Handler 使用）
///
/// P2 8-10 修复：search_logs 接入 AuditQueryFilter 动态构造 WHERE 条件，
/// 替代原固定 `SELECT * ORDER BY id DESC` 查询。
///
/// P2 8-12 修复：新增 `include_sensitive` 字段，控制是否返回 request_body
/// 等敏感字段。审计大屏默认 false；只有 admin 显式传 true 才返回敏感字段。
#[derive(Debug, Deserialize)]
pub struct AuditQueryFilter {
    pub user_id: Option<i32>,
    pub event_type: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    /// 是否包含 request_body 等敏感字段，默认 false
    #[serde(default)]
    pub include_sensitive: bool,
}

/// 审计可视化大屏统计 DTO
#[derive(Debug, Serialize)]
pub struct AuditStats {
    pub total_events_today: i64,
    pub ui_clicks_today: i64,
    pub api_calls_today: i64,
    pub security_alerts_today: i64,
}
