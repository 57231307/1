use chrono::Utc;
use futures::FutureExt;
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait};
use serde_json::Value;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::models::omni_audit_log;

#[derive(Debug, Clone)]
pub struct OmniAuditMessage {
    pub trace_id: String,
    /// 用户 ID；未登录/匿名场景下为 None（避免脏数据归到 user_id=0 系统用户）
    pub user_id: Option<i32>,
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
}

impl OmniAuditEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Result<Self, String> {
        // v5 审计批次 21：测试环境专用默认密钥
        // 该字符串包含 "test" 关键词，会命中 validate_secret 黑名单：
        // 即使误将此值通过 AUDIT_SECRET_KEY 环境变量注入生产环境，
        // AppSettings::new 中的 validate_secret 校验也会拒绝启动（fail-secure）。
        // 仅在 cfg!(test) 或 ENV=test/development 时作为兜底默认密钥使用。
        const TEST_DEFAULT_KEY: &str = "test-only-audit-secret-do-not-use-in-production-32b";

        // 批次 92 P3-7：用 match 替代 unwrap_or_else，使生产环境缺失密钥时
        // 能直接 return Err 从 OmniAuditEngine::new 返回（闭包内的 return 仅从闭包返回）
        let secret_key = match std::env::var("AUDIT_SECRET_KEY") {
            Ok(k) => k,
            Err(_) => {
                // 安全修复：未设置 AUDIT_SECRET_KEY 时，仅在测试/开发环境回退默认密钥
                // 生产环境必须设置 AUDIT_SECRET_KEY 环境变量，否则返回 Err 由调用方决定终止
                let env = std::env::var("ENV").unwrap_or_default();
                // cfg!(test) 覆盖单元测试场景；ENV=test/development 覆盖集成测试/开发场景
                if cfg!(test) || env == "test" || env == "development" {
                    tracing::warn!(
                        env = %env,
                        "安全警告: 未设置 AUDIT_SECRET_KEY 环境变量，测试/开发环境使用默认密钥。生产环境必须设置强密钥！"
                    );
                    TEST_DEFAULT_KEY.to_string()
                } else {
                    return Err(format!(
                        "未设置 AUDIT_SECRET_KEY 环境变量，生产环境必须设置强密钥（至少32字节）；当前 ENV={}",
                        env
                    ));
                }
            }
        };

        if secret_key.len() < 32 {
            return Err("AUDIT_SECRET_KEY 长度必须至少 32 字节".to_string());
        }

        let (sender, mut receiver) = mpsc::channel::<OmniAuditMessage>(10000);

        let db_clone = db.clone();
        let secret_key_clone = secret_key.clone();

        tokio::spawn(async move {
            tracing::info!("OmniAudit 异步收集引擎已启动");
            while let Some(msg) = receiver.recv().await {
                // 批次 7（2026-06-28）：单次消息处理 panic 隔离
                // 防御性 catch_unwind：当前 spawn 块直接代码已无 .unwrap()/.expect()，
                // 但调用链路（如未来重构引入的 panic）可能导致整个审计引擎 spawn 死亡，
                // 确保单次 panic 不退出循环，继续处理后续消息。
                let result = AssertUnwindSafe(async {
                    let payload_str = msg
                        .payload
                        .as_ref()
                        .map(|p| p.to_string())
                        .unwrap_or_default();
                    let sign_material = format!(
                        "{}|{}|{}|{}",
                        msg.trace_id, msg.event_type, msg.action, payload_str
                    );

                    // 使用 HMAC-SHA256 对关键字段进行签名
                    // 批次 7（2026-06-28）：hmac_sha256_hex 已改为返回 Result，
                    // 签名失败时降级为空字符串，不阻断审计日志写入
                    // P0 8-2 修复（批次 53）：签名持久化至 signature 列，实现防篡改
                    let signature = match crate::utils::hash::hmac_sha256_hex(
                        secret_key_clone.as_bytes(),
                        sign_material.as_bytes(),
                    ) {
                        Ok(sig) => sig,
                        Err(e) => {
                            tracing::error!("HMAC 签名失败: {}（审计日志将不带签名）", e);
                            String::new()
                        }
                    };
                    if msg.status == "FAILED"
                        || msg.status == "DENIED"
                        || msg.event_type == "SECURITY_ALERT"
                    {
                        tracing::warn!(
                            "【审计告警】触发告警规则! 用户ID: {:?}, 事件: {}, 资源: {}, 状态: {}",
                            msg.user_id,
                            msg.event_name,
                            msg.resource,
                            msg.status
                        );
                    }

                    let log = omni_audit_log::ActiveModel {
                        id: ActiveValue::NotSet,
                        trace_id: ActiveValue::Set(Some(msg.trace_id)),
                        span_id: ActiveValue::Set(None),
                        parent_span_id: ActiveValue::Set(None),
                        user_id: ActiveValue::Set(msg.user_id),
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
                        created_at: ActiveValue::Set(Some(
                            // P0 修复（批次 5，2026-06-27）：原 .expect("UTC offset 0 is always valid")
                            // 已改为 DateTime::fixed_offset()，消除 panic 风险
                            Utc::now().fixed_offset(),
                        )),
                        // P0 8-2 修复（批次 53）：持久化 HMAC-SHA256 签名，实现审计日志防篡改
                        signature: ActiveValue::Set(Some(signature)),
                    };

                    // 使用 exec_without_returning 避免 last_insert_id 解析问题
                    if let Err(e) = omni_audit_log::Entity::insert(log)
                        .exec_without_returning(db_clone.as_ref())
                        .await
                    {
                        tracing::error!("写入综合审计日志失败: {}", e);
                    } else {
                        tracing::debug!("审计日志写入成功");
                    }
                })
                .catch_unwind()
                .await;

                // 批次 7：panic 隔离后的结果处理
                if let Err(panic_payload) = result {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ OmniAudit spawn 任务内 panic 已被隔离，审计引擎继续运行（不退出循环）"
                    );
                }
            }
        });

        Ok(Self {
            sender,
        })
    }

    pub fn log(&self, msg: OmniAuditMessage) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            // 批次 8（2026-06-28）：一次性 spawn panic 隔离
            let result = AssertUnwindSafe(async {
                if let Err(e) = sender.send(msg).await {
                    tracing::error!("投递审计日志至 Channel 失败: {}", e);
                }
            })
            .catch_unwind()
            .await;
            if let Err(panic_payload) = result {
                let panic_msg = panic_payload
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
                    .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                    .unwrap_or("<非字符串 panic payload>");
                tracing::error!(
                    panic = %panic_msg,
                    "⚠ 审计日志投递 spawn panic 已被隔离"
                );
            }
        });
    }
}
