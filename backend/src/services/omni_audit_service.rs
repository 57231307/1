use std::sync::Arc;
use tokio::sync::mpsc;
use sea_orm::{DatabaseConnection, ActiveValue, EntityTrait};
use chrono::Utc;
use hmac::{Hmac, Mac, KeyInit};
use sha2::Sha256;
use serde_json::Value;

use crate::models::{omni_audit_log, audit_alert_rule};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct OmniAuditMessage {
    pub trace_id: String,
    pub user_id: i32,
    pub event_type: String,
    pub event_name: String,
    pub resource: String,
    pub action: String,
    pub payload: Option<Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub duration_ms: i32,
    pub status: String,
    pub error_msg: Option<String>,
}

pub struct OmniAuditEngine {
    sender: mpsc::Sender<OmniAuditMessage>,
    secret_key: Vec<u8>,
}

impl OmniAuditEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Result<Self, String> {
        let secret_key = std::env::var("AUDIT_SECRET_KEY")
            .map_err(|_| "AUDIT_SECRET_KEY 环境变量未设置".to_string())?;

        if secret_key.len() < 32 {
            return Err("AUDIT_SECRET_KEY 长度必须至少 32 字节".to_string());
        }

        let (sender, mut receiver) = mpsc::channel::<OmniAuditMessage>(10000);

        let db_clone = db.clone();
        let secret_key_clone = secret_key.clone();

        tokio::spawn(async move {
            tracing::info!("OmniAudit 异步收集引擎已启动");
            while let Some(msg) = receiver.recv().await {
                let payload_str = msg.payload.as_ref().map(|p| p.to_string()).unwrap_or_default();
                let sign_material = format!("{}|{}|{}|{}", msg.trace_id, msg.event_type, msg.action, payload_str);

                let mut mac = HmacSha256::new_from_slice(secret_key_clone.as_bytes())
                    .expect("HMAC can take key of any size");
                mac.update(sign_material.as_bytes());
                let signature = hex::encode(mac.finalize().into_bytes());

                if msg.status == "FAILED" || msg.status == "DENIED" || msg.event_type == "SECURITY_ALERT" {
                    tracing::warn!("【审计告警】触发告警规则! 用户ID: {}, 事件: {}, 资源: {}, 状态: {}",
                        msg.user_id, msg.event_name, msg.resource, msg.status);
                }

                let log = omni_audit_log::ActiveModel {
                    trace_id: ActiveValue::Set(msg.trace_id),
                    user_id: ActiveValue::Set(msg.user_id),
                    event_type: ActiveValue::Set(msg.event_type),
                    event_name: ActiveValue::Set(msg.event_name),
                    resource: ActiveValue::Set(msg.resource),
                    action: ActiveValue::Set(msg.action),
                    payload: ActiveValue::Set(msg.payload),
                    ip_address: ActiveValue::Set(msg.ip_address),
                    user_agent: ActiveValue::Set(msg.user_agent),
                    duration_ms: ActiveValue::Set(msg.duration_ms),
                    status: ActiveValue::Set(msg.status),
                    error_msg: ActiveValue::Set(msg.error_msg),
                    signature: ActiveValue::Set(signature),
                    created_at: ActiveValue::Set(Utc::now()),
                    ..Default::default()
                };

                if let Err(e) = omni_audit_log::Entity::insert(log).exec(db_clone.as_ref()).await {
                    tracing::error!("写入综合审计日志失败: {}", e);
                }
            }
        });

        Ok(Self { sender, secret_key: secret_key.into_bytes() })
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
