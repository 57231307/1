use crate::models::webhook::{self, ActiveModel as WebhookActiveModel, Entity as Webhook};
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

/// Webhook 最大重试次数上限（超阈值后停增 retry_count 并置 last_error，防 DB 溢出/重试放大；pub crate 供 handler 返回）
pub(crate) const MAX_RETRY_COUNT: i32 = 5;

/// Webhook负载
/// L8 修复（v8 复审）：降为 pub(crate)，仅模块内部使用，不对外暴露
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WebhookPayload {
    pub(crate) event: String,
    pub(crate) timestamp: String,
    pub(crate) data: serde_json::Value,
}

/// Webhook发送结果（保持 pub：作为 Json<ApiResponse<WebhookDeliveryResult>> 泛型参数出现在响应体中，降级会编译错误）
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

    /// M-4 修复（v9 复审）：校验 webhook 所有权
    /// user_id 为 None 的系统级 webhook，所有认证用户可访问（向后兼容）；user_id 为 Some(uid) 的用户私有 webhook，仅所有者可操作；返回 webhook 模型，校验失败返回 PermissionDenied
    async fn verify_ownership(
        &self,
        user_id: i32,
        webhook_id: i32,
    ) -> Result<webhook::Model, AppError> {
        let webhook = Webhook::find_by_id(webhook_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found("Webhook 不存在"))?;

        // 系统级 webhook（user_id 为 NULL）允许所有认证用户访问
        // 用户私有 webhook 仅所有者可操作
        if let Some(owner_id) = webhook.user_id {
            if owner_id != user_id {
                return Err(AppError::permission_denied(format!(
                    "无权操作此 Webhook（webhook_id={}, owner={}, requester={})",
                    webhook_id, owner_id, user_id
                )));
            }
        }

        Ok(webhook)
    }

    /// 创建 Webhook
    /// M-4 修复（v9 复审）：新增 user_id 参数，记录 webhook 所有者
    pub async fn create_webhook(
        &self,
        user_id: i32,
        name: &str,
        url: &str,
        events: &[&str],
        secret: Option<&str>,
    ) -> Result<webhook::Model, AppError> {
        // 低危 #2 修复（SSRF 防护）：创建前校验 URL 不指向内网/loopback/云元数据
        crate::utils::ssrf_guard::validate_url(url)?;

        let now = Utc::now();
        let active_model = WebhookActiveModel {
            name: Set(name.to_string()),
            url: Set(url.to_string()),
            events: Set(events.join(",")),
            secret: Set(secret.map(|s| s.to_string())),
            is_active: Set(true),
            last_triggered_at: Set(None),
            last_status: Set(None),
            retry_count: Set(0),
            // M-4 修复：记录创建者 user_id，用于后续所有权校验
            user_id: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        active_model
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 获取所有 Webhook
    /// M-4 修复（v9 复审）：新增 user_id 参数，仅返回用户私有 + 系统级 webhook
    pub async fn list_webhooks(&self, user_id: i32) -> Result<Vec<webhook::Model>, AppError> {
        // M-4 修复：使用 OR 条件查询 — user_id IS NULL（系统级）OR user_id = 当前用户
        let ownership_condition = Condition::any()
            .add(webhook::Column::UserId.is_null())
            .add(webhook::Column::UserId.eq(user_id));

        Webhook::find()
            .filter(webhook::Column::IsActive.eq(true))
            .filter(ownership_condition)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 触发 Webhook(发送HTTP请求,M-4 修复新增 user_id 所有权校验)
    pub async fn trigger_webhook(
        &self,
        user_id: i32,
        webhook_id: i32,
        event: &str,
        payload: &str,
    ) -> Result<WebhookDeliveryResult, AppError> {
        let webhook = self.verify_ownership(user_id, webhook_id).await?;
        Self::ensure_webhook_active(&webhook)?;
        Self::ensure_event_matches(&webhook.events, event)?;
        self.mark_webhook_sending(&webhook, payload, event).await?;
        let body = Self::build_webhook_body(event, payload);
        if let Err(e) = crate::utils::ssrf_guard::validate_url(&webhook.url) {
            return Ok(Self::ssrf_blocked_result(&e.to_string()));
        }
        let result = self
            .send_http_request(&webhook.url, &body, webhook.secret.as_deref())
            .await;
        self.persist_final_status(webhook, &result).await?;
        result
    }

    /// 校验 webhook 启用状态(批次 109 P1-2:禁用返回 4xx)
    fn ensure_webhook_active(webhook: &webhook::Model) -> Result<(), AppError> {
        if !webhook.is_active {
            return Err(AppError::business("Webhook 已禁用"));
        }
        Ok(())
    }

    /// 校验触发事件匹配订阅(批次 109 P1-2:不匹配返回 4xx)
    fn ensure_event_matches(events: &str, event: &str) -> Result<(), AppError> {
        let list: Vec<&str> = events.split(',').collect();
        if !list.contains(&event) && !list.contains(&"*") {
            return Err(AppError::business(format!(
                "事件不匹配：webhook 订阅事件为 [{}]，触发事件为 {}",
                events, event
            )));
        }
        Ok(())
    }

    /// 标记发送中并持久化 payload+event(批次 251:支持 retry 重投)
    async fn mark_webhook_sending(
        &self,
        webhook: &webhook::Model,
        payload: &str,
        event: &str,
    ) -> Result<(), AppError> {
        let mut active: WebhookActiveModel = webhook.clone().into();
        active.last_triggered_at = Set(Some(Utc::now()));
        active.last_status = Set(Some("SENDING".to_string()));
        active.last_payload = Set(Some(payload.to_string()));
        active.last_event = Set(Some(event.to_string()));
        active.updated_at = Set(Utc::now());
        active.update(self.db.as_ref()).await?;
        Ok(())
    }

    /// 构建 webhook 请求体
    fn build_webhook_body(event: &str, payload: &str) -> String {
        let webhook_payload = WebhookPayload {
            event: event.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            data: serde_json::from_str(payload)
                .unwrap_or_else(|_| serde_json::json!({"raw": payload})),
        };
        serde_json::to_string(&webhook_payload).unwrap_or_else(|_| payload.to_string())
    }

    /// 构造 SSRF 拦截结果(低危 #2:防 DNS Rebinding)
    fn ssrf_blocked_result(reason: &str) -> WebhookDeliveryResult {
        WebhookDeliveryResult {
            success: false,
            status_code: None,
            response_body: None,
            error: Some(format!("SSRF 防护拦截：{}", reason)),
        }
    }

    /// 持久化最终状态(批次 251:retry 计数,成功重置 0)
    async fn persist_final_status(
        &self,
        webhook: webhook::Model,
        result: &Result<WebhookDeliveryResult, AppError>,
    ) -> Result<(), AppError> {
        let current_retry_count = webhook.retry_count;
        let mut final_model: WebhookActiveModel = webhook.into();
        final_model.updated_at = Set(Utc::now());
        match result {
            Ok(delivery) => {
                if delivery.success {
                    final_model.last_status = Set(Some("SUCCESS".to_string()));
                    final_model.retry_count = Set(0);
                } else {
                    final_model.last_status = Set(Some("FAILED".to_string()));
                    Self::apply_retry_increment(&mut final_model, current_retry_count);
                }
            }
            Err(_) => {
                final_model.last_status = Set(Some("ERROR".to_string()));
                Self::apply_retry_increment(&mut final_model, current_retry_count);
            }
        }
        final_model.update(self.db.as_ref()).await?;
        Ok(())
    }

    /// 递增 retry_count,达上限标记永久失败
    fn apply_retry_increment(model: &mut WebhookActiveModel, current: i32) {
        if current >= MAX_RETRY_COUNT {
            error!(
                current_count = current,
                max = MAX_RETRY_COUNT,
                "Webhook 已达最大重试次数上限，标记为永久失败"
            );
            model.last_status = Set(Some("FAILED_PERMANENT".to_string()));
        } else {
            model.retry_count = Set(current + 1);
        }
    }

    /// 发送HTTP请求
    async fn send_http_request(
        &self,
        url: &str,
        body: &str,
        secret: Option<&str>,
    ) -> Result<WebhookDeliveryResult, AppError> {
        // SSRF 缓解：仅允许 HTTPS 协议
        if !url.to_lowercase().starts_with("https://") {
            return Err(AppError::validation("仅允许 HTTPS 协议"));
        }

        let client = Self::build_webhook_client(url)?;
        // V15 P1 20.1-A：注入 traceparent 到 webhook 出站请求
        let traceparent =
            crate::observability::trace_context::traceparent_from_current_span();
        let request = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "BingXi-ERP-Webhook/1.0")
            .header(crate::observability::trace_context::TRACEPARENT_HEADER, traceparent);
        let request = Self::attach_webhook_signature(request, url, body, secret);
        let request = request.body(body.to_string());

        let result = request.send().await;
        Ok(Self::parse_webhook_response(result).await)
    }

    /// 构建 SSRF 防御的 webhook 客户端（HTTPS 校验 + DNS Rebinding 防御 + 禁止重定向）
    /// BE-V-2/TS-S-2 修复（2026-06-25 第二次全面审计）：TOCTOU 根治。；validate_url_and_resolve 返回校验通过的安全 IP 列表，；用 resolve_to_addrs 将 host 固定到已校验 IP，reqwest 不再独立解析 DNS，；彻底消除"校验时解析为公网 IP、连接时解析为内网 IP"的 TOCTOU 窗口。
    fn build_webhook_client(url: &str) -> Result<reqwest::Client, AppError> {
        let (host, safe_addrs) = crate::utils::ssrf_guard::validate_url_and_resolve(url)?;
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .resolve_to_addrs(&host, &safe_addrs)
            .build()
            .map_err(|e| AppError::internal(format!("HTTP 客户端构建失败: {}", e)))
    }

    /// 附加 HMAC-SHA256 签名头（P1-B 修复：出站签名与入站验证使用同一份算法）
    /// 旧实现 SHA256(body || secret) 存在长度扩展攻击风险：攻击者可在不知 secret 的情况下；推算 secret + padding 后的扩展摘要。现改为 HMAC-SHA256(secret, body)。；签名失败时 warn 日志降级，不阻塞 webhook 发送。
    fn attach_webhook_signature(
        mut request: reqwest::RequestBuilder,
        url: &str,
        body: &str,
        secret: Option<&str>,
    ) -> reqwest::RequestBuilder {
        if let Some(secret) = secret {
            match crate::utils::webhook_signature::sign_webhook_payload(body, secret) {
                Ok(signature) => {
                    request =
                        request.header("X-Webhook-Signature", format!("sha256={}", signature));
                }
                Err(e) => {
                    // 规则 12 合规：日志只记录主机名，不记录完整 URL，防止 URL 中的敏感参数泄露
                    let host = url::Url::parse(url)
                        .ok()
                        .and_then(|u| u.host_str().map(|h| h.to_string()))
                        .unwrap_or_else(|| "unknown".to_string());
                    tracing::warn!(error = %e, webhook_host = %host, "Webhook 签名计算失败，跳过签名头");
                }
            }
        }
        request
    }

    /// 解析 webhook 响应为 WebhookDeliveryResult（发送失败也封装为结构体，不返回 Err）
    async fn parse_webhook_response(
        result: Result<reqwest::Response, reqwest::Error>,
    ) -> WebhookDeliveryResult {
        match result {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let response_body = response.text().await.unwrap_or_default();
                let success = (200..300).contains(&status_code);
                WebhookDeliveryResult {
                    success,
                    status_code: Some(status_code),
                    response_body: Some(response_body),
                    error: if success {
                        None
                    } else {
                        Some(format!("HTTP {}", status_code))
                    },
                }
            }
            Err(e) => WebhookDeliveryResult {
                success: false,
                status_code: None,
                response_body: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// 测试 Webhook（批次 108 P1-8：已通过 POST /webhooks/:id/test 接入业务）
    /// 触发一次 test 事件，验证 webhook 配置正确性。；M-4 修复（v9 复审）：新增 user_id 参数，测试前校验所有权
    pub async fn test_webhook(
        &self,
        user_id: i32,
        webhook_id: i32,
    ) -> Result<WebhookDeliveryResult, AppError> {
        // M-4 修复：通过 verify_ownership 校验所有权（trigger_webhook 内部也会校验，
        // 但提前校验可对"webhook 不存在"返回 NotFound 而非 BusinessError）
        self.verify_ownership(user_id, webhook_id).await?;

        let test_payload = serde_json::json!({
            "message": "This is a test webhook delivery",
            "test": true
        })
        .to_string();

        self.trigger_webhook(user_id, webhook_id, "test", &test_payload)
            .await
    }

    /// 获取单个 Webhook 详情（批次 108 P1-8：为 GET /webhooks/:id/logs 提供数据源）
    /// 返回 webhooks 表中的执行状态字段（last_triggered_at / last_status / retry_count）。；当前未独立持久化调用日志（无 webhook_logs 表），返回 webhook 自身的执行状态汇总。；M-4 修复（v9 复审）：新增 user_id 参数，获取前校验所有权
    pub async fn get_webhook(&self, user_id: i32, id: i32) -> Result<webhook::Model, AppError> {
        self.verify_ownership(user_id, id).await
    }

    /// 删除 Webhook
    /// M-4 修复（v9 复审）：新增 user_id 参数，删除前校验所有权
    pub async fn delete_webhook(&self, user_id: i32, id: i32) -> Result<(), AppError> {
        let webhook = self.verify_ownership(user_id, id).await?;

        let mut active_model: WebhookActiveModel = webhook.into();
        active_model.is_active = Set(false);
        active_model.updated_at = Set(Utc::now());
        active_model.update(self.db.as_ref()).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M8 测试：MAX_RETRY_COUNT 常量值为 5
    #[test]
    fn test_max_retry_count_value() {
        assert_eq!(MAX_RETRY_COUNT, 5);
    }

    /// M8 测试：WebhookPayload 序列化/反序列化正确
    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            event: "order.created".to_string(),
            timestamp: "2026-07-11T08:00:00Z".to_string(),
            data: serde_json::json!({"order_id": 12345}),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: WebhookPayload = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.event, "order.created");
        assert_eq!(deserialized.timestamp, "2026-07-11T08:00:00Z");
        assert_eq!(deserialized.data["order_id"], 12345);
    }

    /// M8 测试：WebhookDeliveryResult 默认状态（失败时）
    #[test]
    fn test_webhook_delivery_result_error_case() {
        let result = WebhookDeliveryResult {
            success: false,
            status_code: None,
            response_body: None,
            error: Some("连接超时".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: WebhookDeliveryResult = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.success);
        assert!(deserialized.status_code.is_none());
        assert!(deserialized.error.is_some());
        assert_eq!(deserialized.error.unwrap(), "连接超时");
    }
}
