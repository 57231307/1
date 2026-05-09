#![allow(dead_code)]

use crate::models::webhook::{self, Entity as Webhook, ActiveModel as WebhookActiveModel};
use sea_orm::*;
use std::sync::Arc;
use chrono::Utc;

pub struct WebhookService {
    db: Arc<DatabaseConnection>,
}

impl WebhookService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建 Webhook
    pub async fn create_webhook(
        &self,
        tenant_id: i32,
        name: &str,
        url: &str,
        events: &[&str],
        secret: Option<&str>,
    ) -> Result<webhook::Model, DbErr> {
        let now = Utc::now();
        let active_model = WebhookActiveModel {
            tenant_id: Set(tenant_id),
            name: Set(name.to_string()),
            url: Set(url.to_string()),
            events: Set(events.join(",")),
            secret: Set(secret.map(|s| s.to_string())),
            is_active: Set(true),
            last_triggered_at: Set(None),
            last_status: Set(None),
            retry_count: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        active_model.insert(self.db.as_ref()).await
    }

    /// 获取租户的所有 Webhook
    pub async fn list_webhooks(
        &self,
        tenant_id: i32,
    ) -> Result<Vec<webhook::Model>, DbErr> {
        Webhook::find()
            .filter(webhook::Column::TenantId.eq(tenant_id))
            .filter(webhook::Column::IsActive.eq(true))
            .all(self.db.as_ref())
            .await
    }

    /// 触发 Webhook
    pub async fn trigger_webhook(
        &self,
        webhook_id: i32,
        event: &str,
        _payload: &str,
    ) -> Result<(), DbErr> {
        let webhook = Webhook::find_by_id(webhook_id)
            .one(self.db.as_ref())
            .await?
            .ok_or(DbErr::Custom("Webhook 不存在".to_string()))?;

        if !webhook.is_active {
            return Ok(());
        }

        // 检查事件是否匹配
        let events: Vec<&str> = webhook.events.split(',').collect();
        if !events.contains(&event) && !events.contains(&"*") {
            return Ok(());
        }

        // 更新状态
        let mut active_model: WebhookActiveModel = webhook.into();
        active_model.last_triggered_at = Set(Some(Utc::now()));
        active_model.last_status = Set(Some("PENDING".to_string()));
        active_model.updated_at = Set(Utc::now());
        active_model.update(self.db.as_ref()).await?;

        Ok(())
    }

    /// 删除 Webhook
    pub async fn delete_webhook(&self, id: i32) -> Result<(), DbErr> {
        let webhook = Webhook::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or(DbErr::Custom("Webhook 不存在".to_string()))?;

        let mut active_model: WebhookActiveModel = webhook.into();
        active_model.is_active = Set(false);
        active_model.updated_at = Set(Utc::now());
        active_model.update(self.db.as_ref()).await?;

        Ok(())
    }
}
