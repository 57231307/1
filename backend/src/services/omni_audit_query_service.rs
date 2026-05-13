use crate::models::omni_audit_log;
use crate::utils::error::AppError;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, PaginatorTrait,
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
    pub keyword: Option<String>, // 模糊查询 payload
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

    /// 大屏统计数据
    pub async fn get_dashboard_stats(&self) -> Result<AuditStats, AppError> {
        let today = chrono::Utc::now().date_naive();

        let total = omni_audit_log::Entity::find()
            .filter(omni_audit_log::Column::CreatedAt.gte(today))
            .count(self.db.as_ref())
            .await?;

        let ui_clicks = omni_audit_log::Entity::find()
            .filter(omni_audit_log::Column::CreatedAt.gte(today))
            .filter(omni_audit_log::Column::EventType.eq("UI_CLICK"))
            .count(self.db.as_ref())
            .await?;

        let api_calls = omni_audit_log::Entity::find()
            .filter(omni_audit_log::Column::CreatedAt.gte(today))
            .filter(omni_audit_log::Column::EventType.eq("API_CALL"))
            .count(self.db.as_ref())
            .await?;

        let security_alerts = omni_audit_log::Entity::find()
            .filter(omni_audit_log::Column::CreatedAt.gte(today))
            .filter(omni_audit_log::Column::EventType.eq("SECURITY_ALERT"))
            .count(self.db.as_ref())
            .await?;

        Ok(AuditStats {
            total_events_today: total as i64,
            ui_clicks_today: ui_clicks as i64,
            api_calls_today: api_calls as i64,
            security_alerts_today: security_alerts as i64,
        })
    }

    /// 复杂检索查询
    pub async fn search_logs(
        &self,
        filter: AuditQueryFilter,
    ) -> Result<(Vec<omni_audit_log::Model>, u64), AppError> {
        let mut query = omni_audit_log::Entity::find();

        if let Some(uid) = filter.user_id {
            query = query.filter(omni_audit_log::Column::UserId.eq(uid));
        }

        if let Some(et) = filter.event_type {
            query = query.filter(omni_audit_log::Column::EventType.eq(et));
        }

        if let Some(start) = filter.start_date {
            query = query.filter(omni_audit_log::Column::CreatedAt.gte(start));
        }

        if let Some(end) = filter.end_date {
            query = query.filter(omni_audit_log::Column::CreatedAt.lte(end));
        }

        if let Some(kw) = filter.keyword {
            // Postgres JSONB operator could be used, here fallback to LIKE string cast if needed,
            // or just match on event_name for simplicity across DBs.
            query = query.filter(omni_audit_log::Column::EventName.contains(&kw));
        }

        let total = query.clone().count(self.db.as_ref()).await?;

        let page = filter.page.unwrap_or(1).max(1);
        let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);

        let logs = query
            .order_by_desc(omni_audit_log::Column::CreatedAt)
            .limit(page_size)
            .offset((page - 1) * page_size)
            .all(self.db.as_ref())
            .await?;

        Ok((logs, total))
    }
}
