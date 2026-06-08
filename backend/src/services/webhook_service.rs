#![allow(dead_code)]

use crate::models::webhook::{self, ActiveModel as WebhookActiveModel, Entity as Webhook};
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Webhook负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

/// Webhook发送结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveryResult {
    pub success: bool,
    pub status_code: Option<u16>,
    pub response_body: Option<String>,
    pub error: Option<String>,
}

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
    ) -> Result<webhook::Model, AppError> {
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

        active_model
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 获取租户的所有 Webhook
    pub async fn list_webhooks(&self, tenant_id: i32) -> Result<Vec<webhook::Model>, AppError> {
        Webhook::find()
            .filter(webhook::Column::TenantId.eq(tenant_id))
            .filter(webhook::Column::IsActive.eq(true))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 触发 Webhook（实际发送HTTP请求）
    pub async fn trigger_webhook(
        &self,
        webhook_id: i32,
        event: &str,
        payload: &str,
    ) -> Result<WebhookDeliveryResult, AppError> {
        let webhook = Webhook::find_by_id(webhook_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("Webhook 不存在"))?;

        if !webhook.is_active {
            return Ok(WebhookDeliveryResult {
                success: false,
                status_code: None,
                response_body: None,
                error: Some("Webhook已禁用".to_string()),
            });
        }

        // 检查事件是否匹配
        let events: Vec<&str> = webhook.events.split(',').collect();
        if !events.contains(&event) && !events.contains(&"*") {
            return Ok(WebhookDeliveryResult {
                success: false,
                status_code: None,
                response_body: None,
                error: Some("事件不匹配".to_string()),
            });
        }

        // 更新状态为发送中
        let mut active_model: WebhookActiveModel = webhook.clone().into();
        active_model.last_triggered_at = Set(Some(Utc::now()));
        active_model.last_status = Set(Some("SENDING".to_string()));
        active_model.updated_at = Set(Utc::now());
        active_model.update(self.db.as_ref()).await?;

        // 构建请求体
        let webhook_payload = WebhookPayload {
            event: event.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            data: serde_json::from_str(payload)
                .unwrap_or_else(|_| serde_json::json!({"raw": payload})),
        };

        let body = serde_json::to_string(&webhook_payload).unwrap_or_else(|_| payload.to_string());

        // 发送HTTP请求
        let result = self
            .send_http_request(&webhook.url, &body, webhook.secret.as_deref())
            .await;

        // 更新最终状态
        let mut final_model: WebhookActiveModel = webhook.into();
        final_model.updated_at = Set(Utc::now());
        match &result {
            Ok(delivery) => {
                final_model.last_status = Set(Some(
                    if delivery.success {
                        "SUCCESS"
                    } else {
                        "FAILED"
                    }
                    .to_string(),
                ));
            }
            Err(_) => {
                final_model.last_status = Set(Some("ERROR".to_string()));
                // 获取当前retry_count值并递增
                let current_count: i32 =
                    if let sea_orm::ActiveValue::Set(v) = &final_model.retry_count {
                        *v
                    } else {
                        0
                    };
                final_model.retry_count = Set(current_count + 1);
            }
        }
        final_model.update(self.db.as_ref()).await?;

        result
    }

    /// 发送HTTP请求
    async fn send_http_request(
        &self,
        url: &str,
        body: &str,
        secret: Option<&str>,
    ) -> Result<WebhookDeliveryResult, AppError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let mut request = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "BingXi-ERP-Webhook/1.0");

        // 如果有签名密钥，添加签名头
        if let Some(secret) = secret {
            use ring::digest::{Context, SHA256};
            let mut context = Context::new(&SHA256);
            context.update(body.as_bytes());
            context.update(secret.as_bytes());
            let hash_result = context.finish();

            let mut signature = String::with_capacity(hash_result.as_ref().len() * 2);
            for byte in hash_result.as_ref() {
                use std::fmt::Write;
                write!(&mut signature, "{:02x}", byte).unwrap();
            }
            request = request.header("X-Webhook-Signature", format!("sha256={}", signature));
        }

        request = request.body(body.to_string());

        match request.send().await {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let response_body = response.text().await.unwrap_or_default();
                let success = (200..300).contains(&status_code);

                Ok(WebhookDeliveryResult {
                    success,
                    status_code: Some(status_code),
                    response_body: Some(response_body),
                    error: if success {
                        None
                    } else {
                        Some(format!("HTTP {}", status_code))
                    },
                })
            }
            Err(e) => Ok(WebhookDeliveryResult {
                success: false,
                status_code: None,
                response_body: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// 测试Webhook
    pub async fn test_webhook(&self, webhook_id: i32) -> Result<WebhookDeliveryResult, AppError> {
        let test_payload = serde_json::json!({
            "message": "This is a test webhook delivery",
            "test": true
        })
        .to_string();

        self.trigger_webhook(webhook_id, "test", &test_payload)
            .await
    }

    /// 删除 Webhook（带租户权限验证）
    pub async fn delete_webhook(&self, id: i32, tenant_id: i32) -> Result<(), AppError> {
        let webhook = Webhook::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("Webhook 不存在"))?;

        if webhook.tenant_id != tenant_id {
            return Err(AppError::permission_denied("无权删除此Webhook"));
        }

        let mut active_model: WebhookActiveModel = webhook.into();
        active_model.is_active = Set(false);
        active_model.updated_at = Set(Utc::now());
        active_model.update(self.db.as_ref()).await?;

        Ok(())
    }

    /// 获取单个Webhook详情
    pub async fn get_webhook(&self, id: i32, tenant_id: i32) -> Result<webhook::Model, AppError> {
        let webhook = Webhook::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("Webhook 不存在"))?;

        if webhook.tenant_id != tenant_id {
            return Err(AppError::permission_denied("无权查看此Webhook"));
        }

        Ok(webhook)
    }

    /// 更新Webhook配置
    pub async fn update_webhook(
        &self,
        id: i32,
        tenant_id: i32,
        name: Option<&str>,
        url: Option<&str>,
        events: Option<&[&str]>,
        secret: Option<Option<&str>>,
    ) -> Result<webhook::Model, AppError> {
        let webhook = Webhook::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("Webhook 不存在"))?;

        if webhook.tenant_id != tenant_id {
            return Err(AppError::permission_denied("无权修改此Webhook"));
        }

        let mut active_model: WebhookActiveModel = webhook.into();
        if let Some(n) = name {
            active_model.name = Set(n.to_string());
        }
        if let Some(u) = url {
            active_model.url = Set(u.to_string());
        }
        if let Some(e) = events {
            active_model.events = Set(e.join(","));
        }
        if let Some(s) = secret {
            active_model.secret = Set(s.map(|s| s.to_string()));
        }
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }
}
