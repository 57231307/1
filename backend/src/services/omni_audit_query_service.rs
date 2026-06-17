use serde::{Deserialize, Serialize};

/// 审计查询过滤条件（供 Handler 使用）
#[derive(Debug, Deserialize)]
pub struct AuditQueryFilter {
    pub user_id: Option<i32>,
    pub event_type: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 审计可视化大屏统计 DTO
#[derive(Debug, Serialize)]
pub struct AuditStats {
    pub total_events_today: i64,
    pub ui_clicks_today: i64,
    pub api_calls_today: i64,
    pub security_alerts_today: i64,
}
