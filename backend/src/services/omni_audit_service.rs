use std::sync::Arc;
use tokio::sync::mpsc;
use sea_orm::{DatabaseConnection, ActiveValue, EntityTrait};
use chrono::Utc;
use hmac::{Hmac, Mac, KeyInit};
use sha2::Sha256;
use serde_json::Value;

use crate::models::{omni_audit_log, audit_alert_rule};

type HmacSha256 = Hmac<Sha256>;
const AUDIT_SECRET_KEY: &[u8] = b"bingxi_erp_audit_super_secret_key_2026";

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
}

impl OmniAuditEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        // 创建容量为 10000 的 channel，防止高并发时阻塞业务线程
        let (sender, mut receiver) = mpsc::channel::<OmniAuditMessage>(10000);

        let db_clone = db.clone();
        
        // 启动后台守护任务 (Daemon Task)
        tokio::spawn(async move {
            tracing::info!("OmniAudit 异步收集引擎已启动");
            while let Some(msg) = receiver.recv().await {
                // 1. 签名计算
                let payload_str = msg.payload.as_ref().map(|p| p.to_string()).unwrap_or_default();
                let sign_material = format!("{}|{}|{}|{}", msg.trace_id, msg.event_type, msg.action, payload_str);
                
                let mut mac = HmacSha256::new_from_slice(AUDIT_SECRET_KEY).expect("HMAC can take key of any size");
                mac.update(sign_material.as_bytes());
                let signature = hex::encode(mac.finalize().into_bytes());

                // 2. 告警规则匹配 (简化版: 若状态为 FAILED 或 DENIED 且类型为 SECURITY_ALERT)
                if msg.status == "FAILED" || msg.status == "DENIED" || msg.event_type == "SECURITY_ALERT" {
                    tracing::warn!("【审计告警】触发告警规则! 用户ID: {}, 事件: {}, 资源: {}, 状态: {}", 
                        msg.user_id, msg.event_name, msg.resource, msg.status);
                    // 实际项目中这里可以推送到 WebSocket 或邮件
                }

                // 3. 落盘存储
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

        Self { sender }
    }

    /// 发送异步审计日志，不阻塞当前线程
    pub fn log(&self, msg: OmniAuditMessage) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            if let Err(e) = sender.send(msg).await {
                tracing::error!("投递审计日志至 Channel 失败: {}", e);
            }
        });
    }
}
