#![allow(dead_code)]
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait};
use serde_json::Value;
use sha2::Sha256;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::models::omni_audit_log;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct OmniAuditMessage {
    pub trace_id: String,
    pub user_id: i32,
    pub username: Option<String>,
    pub event_type: String,
    pub event_name: String,
    pub resource: String,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    pub description: Option<String>,
    pub payload: Option<Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_method: Option<String>,
    pub request_path: Option<String>,
    pub request_body: Option<String>,
    pub duration_ms: i32,
    pub status: String,
    pub error_msg: Option<String>,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
}

pub struct OmniAuditEngine {
    sender: mpsc::Sender<OmniAuditMessage>,
    secret_key: Vec<u8>,
}

impl OmniAuditEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Result<Self, String> {
        let secret_key = std::env::var("AUDIT_SECRET_KEY")
            .unwrap_or_else(|_| {
                tracing::warn!(
                    "安全警告: 未设置 AUDIT_SECRET_KEY 环境变量，正在使用默认密钥。请在生产环境中设置强密钥！"
                );
                "default-audit-secret-key-for-test-environments-only-32-bytes".to_string()
            });

        if secret_key.len() < 32 {
            return Err("AUDIT_SECRET_KEY 长度必须至少 32 字节".to_string());
        }

        let (sender, mut receiver) = mpsc::channel::<OmniAuditMessage>(10000);

        let db_clone = db.clone();
        let secret_key_clone = secret_key.clone();

        tokio::spawn(async move {
            tracing::info!("OmniAudit 异步收集引擎已启动");
            while let Some(msg) = receiver.recv().await {
                let payload_str = msg
                    .payload
                    .as_ref()
                    .map(|p| p.to_string())
                    .unwrap_or_default();
                let sign_material = format!(
                    "{}|{}|{}|{}",
                    msg.trace_id, msg.event_type, msg.action, payload_str
                );

                let mut mac = HmacSha256::new_from_slice(secret_key_clone.as_bytes())
                    .expect("HMAC can take key of any size");
                mac.update(sign_material.as_bytes());
                let _signature = hex::encode(mac.finalize().into_bytes());

                if msg.status == "FAILED"
                    || msg.status == "DENIED"
                    || msg.event_type == "SECURITY_ALERT"
                {
                    tracing::warn!(
                        "【审计告警】触发告警规则! 用户ID: {}, 事件: {}, 资源: {}, 状态: {}",
                        msg.user_id,
                        msg.event_name,
                        msg.resource,
                        msg.status
                    );
                }

                let log = omni_audit_log::ActiveModel {
                    id: ActiveValue::NotSet,
                    tenant_id: ActiveValue::Set(Some(1)), // 默认租户
                    trace_id: ActiveValue::Set(Some(msg.trace_id)),
                    span_id: ActiveValue::Set(None),
                    parent_span_id: ActiveValue::Set(None),
                    user_id: ActiveValue::Set(Some(msg.user_id)),
                    username: ActiveValue::Set(msg.username),
                    module: ActiveValue::Set(Some(msg.event_type)),
                    action: ActiveValue::Set(Some(msg.event_name)),
                    resource_type: ActiveValue::Set(msg.resource_type),
                    resource_id: ActiveValue::Set(msg.resource_id),
                    resource_name: ActiveValue::Set(msg.resource_name),
                    description: ActiveValue::Set(msg.description),
                    ip_address: ActiveValue::Set(msg.ip_address),
                    user_agent: ActiveValue::Set(msg.user_agent),
                    request_method: ActiveValue::Set(msg.request_method),
                    request_path: ActiveValue::Set(msg.request_path),
                    request_body: ActiveValue::Set(msg.request_body),
                    response_status: ActiveValue::Set(Some(msg.status.parse::<i32>().unwrap_or(
                        match msg.status.as_str() {
                            "SUCCESS" => 200,
                            "FAILED" => 500,
                            "DENIED" => 403,
                            _ => 0,
                        },
                    ))),
                    duration_ms: ActiveValue::Set(Some(msg.duration_ms)),
                    old_value: ActiveValue::Set(msg.old_value),
                    new_value: ActiveValue::Set(msg.new_value),
                    created_at: ActiveValue::Set(Some(Utc::now().with_timezone(
                        &chrono::FixedOffset::east_opt(0).expect("UTC offset 0 is always valid"),
                    ))),
                };

                if let Err(e) = omni_audit_log::Entity::insert(log)
                    .exec(db_clone.as_ref())
                    .await
                {
                    tracing::error!("写入综合审计日志失败: {}", e);
                }
            }
        });

        Ok(Self {
            sender,
            secret_key: secret_key.into_bytes(),
        })
    }

    pub fn log(&self, msg: OmniAuditMessage) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            if let Err(e) = sender.send(msg).await {
                tracing::error!("投递审计日志至 Channel 失败: {}", e);
            }
        });
    }
}
