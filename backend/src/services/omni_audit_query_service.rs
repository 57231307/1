use crate::models::omni_audit_log;
use crate::utils::error::AppError;
use sea_orm::{
    DatabaseConnection, EntityTrait, QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct OmniAuditQueryService {
    db: Arc<DatabaseConnection>,
}

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

#[derive(Debug, Serialize)]
pub struct AuditStats {
    pub total_events_today: i64,
    pub ui_clicks_today: i64,
    pub api_calls_today: i64,
    pub security_alerts_today: i64,
}

impl OmniAuditQueryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_dashboard_stats(&self) -> Result<AuditStats, AppError> {
        let all_logs = omni_audit_log::Entity::find()
            .all(self.db.as_ref())
            .await?;

        let total = all_logs.len() as i64;
        let ui_clicks = all_logs.iter().filter(|l| l.module.as_deref() == Some("UI_CLICK")).count() as i64;
        let api_calls = all_logs.iter().filter(|l| l.module.as_deref() == Some("API_CALL")).count() as i64;
        let security_alerts = all_logs.iter().filter(|l| l.module.as_deref() == Some("SECURITY_ALERT")).count() as i64;

        Ok(AuditStats {
            total_events_today: total,
            ui_clicks_today: ui_clicks,
            api_calls_today: api_calls,
            security_alerts_today: security_alerts,
        })
    }

    pub async fn search_logs(
        &self,
        filter: AuditQueryFilter,
    ) -> Result<(Vec<omni_audit_log::Model>, u64), AppError> {
        let query = omni_audit_log::Entity::find();

        let page_size: u64 = filter.page_size.unwrap_or(20).clamp(1, 100);
        let page = filter.page.unwrap_or(1);
        let offset = if page > 0 { (page - 1) * page_size } else { 0 };

        let logs = query
            .order_by_desc(omni_audit_log::Column::Id)
            .limit(page_size)
            .offset(offset)
            .all(self.db.as_ref())
            .await?;

        let total = logs.len() as u64;

        Ok((logs, total))
    }
}
