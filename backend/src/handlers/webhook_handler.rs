use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::webhook_service::{WebhookDeliveryResult, WebhookService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub events: String,
    pub is_active: bool,
}

impl From<crate::models::webhook::Model> for WebhookResponse {
    fn from(model: crate::models::webhook::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            url: model.url,
            events: model.events,
            is_active: model.is_active,
        }
    }
}

pub async fn create_webhook(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<Json<ApiResponse<WebhookResponse>>, AppError> {
    let service = WebhookService::new(state.db);
    let events: Vec<&str> = req.events.iter().map(|s| s.as_str()).collect();

    match service
        .create_webhook(&req.name, &req.url, &events, req.secret.as_deref())
        .await
    {
        Ok(webhook) => Ok(Json(ApiResponse::success(WebhookResponse::from(webhook)))),
        Err(e) => {
            tracing::error!("创建 Webhook 失败: {}", e);
            Err(AppError::internal("创建 Webhook 失败"))
        }
    }
}

pub async fn list_webhooks(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<WebhookResponse>>>, AppError> {
    let service = WebhookService::new(state.db);

    match service.list_webhooks().await {
        Ok(webhooks) => {
            let responses: Vec<WebhookResponse> =
                webhooks.into_iter().map(WebhookResponse::from).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("获取 Webhook 列表失败: {}", e);
            Err(AppError::internal("获取 Webhook 列表失败"))
        }
    }
}

pub async fn delete_webhook(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = WebhookService::new(state.db);

    match service.delete_webhook(id).await {
        Ok(()) => Ok(Json(ApiResponse::success_with_message((), "删除成功"))),
        Err(e) => {
            tracing::error!("删除 Webhook 失败: {}", e);
            Err(AppError::internal("删除 Webhook 失败"))
        }
    }
}

// ============================================================================
// 批次 108 P1-8 修复：webhook handler 真实实现（retry/get_logs/test_webhook 3 端点）
//
// 原状态：service::webhook_service.rs 的 test_webhook 方法标 #[allow(dead_code)]，
// retry_webhook / get_webhook_logs 完全不存在；routes/analytics.rs 中
// /webhooks/:id/{retry,logs,test} 3 个路由未挂载。
//
// 修复：新增 3 个 handler 并挂载到 /webhooks 路由，移除 service.test_webhook 的 dead_code 标注。
// - POST /:id/test  → test_webhook（触发一次 test 事件，验证配置正确性）
// - POST /:id/retry → retry_webhook（重试上一次失败的 webhook 调用）
// - GET  /:id/logs  → get_webhook_logs（返回 webhook 执行状态：last_status/retry_count/last_triggered_at）
// ============================================================================

/// 测试 Webhook（POST /webhooks/:id/test）
///
/// 触发一次 test 事件，验证 webhook 配置正确性。
/// 出于 SSRF 安全考虑，响应中不回显目标 URL 返回的内容。
pub async fn test_webhook(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<WebhookDeliveryResult>>, AppError> {
    let service = WebhookService::new(state.db);

    match service.test_webhook(id).await {
        Ok(mut result) => {
            // SSRF 缓解：测试接口不回显目标响应体，防止攻击者读取内网数据
            result.response_body = Some("出于安全原因，已隐藏响应内容".to_string());
            Ok(Json(ApiResponse::success_with_message(
                result,
                "测试消息已发送",
            )))
        }
        Err(e) => {
            tracing::error!("测试 Webhook 失败: {}", e);
            Err(AppError::internal(format!("测试 Webhook 失败: {}", e)))
        }
    }
}

/// 重试 Webhook（POST /webhooks/:id/retry）
///
/// 对上一次失败的 webhook 调用进行重试。当前未持久化历史 payload，
/// 简化方案：触发一次 retry 事件，由 webhook 接收方按 retry 事件处理。
/// 若 webhook 配置的事件列表不包含 retry 或 *，会返回事件不匹配的业务错误。
pub async fn retry_webhook(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<WebhookDeliveryResult>>, AppError> {
    let service = WebhookService::new(state.db);

    // retry 事件 payload：包含 webhook_id 便于接收方识别重试来源
    let retry_payload = serde_json::json!({
        "webhook_id": id,
        "retry": true,
        "message": "Webhook 重试触发"
    })
    .to_string();

    match service.trigger_webhook(id, "retry", &retry_payload).await {
        Ok(mut result) => {
            // SSRF 缓解：重试接口同样不回显目标响应体
            result.response_body = Some("出于安全原因，已隐藏响应内容".to_string());
            if result.success {
                Ok(Json(ApiResponse::success_with_message(
                    result,
                    "Webhook 重试成功",
                )))
            } else {
                Ok(Json(ApiResponse::success_with_message(
                    result,
                    "Webhook 重试已执行，但目标返回非成功状态",
                )))
            }
        }
        Err(e) => {
            tracing::error!("重试 Webhook 失败: {}", e);
            Err(AppError::internal(format!("重试 Webhook 失败: {}", e)))
        }
    }
}

/// Webhook 执行日志（GET /webhooks/:id/logs）
///
/// 返回 webhook 的执行状态信息。当前未独立持久化调用日志（无 webhook_logs 表），
/// 返回 webhooks 表中的 last_* 字段作为执行状态汇总：
/// - last_triggered_at：上次触发时间
/// - last_status：上次执行状态（SENDING/SUCCESS/FAILED/ERROR/FAILED_PERMANENT）
/// - retry_count：连续失败重试次数（上限 MAX_RETRY_COUNT=5）
/// - events：订阅事件列表
#[derive(Debug, Serialize)]
pub struct WebhookLogEntry {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub events: String,
    pub is_active: bool,
    pub last_triggered_at: Option<String>,
    pub last_status: Option<String>,
    pub retry_count: i32,
    pub max_retry_count: i32,
}

pub async fn get_webhook_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<WebhookLogEntry>>, AppError> {
    let service = WebhookService::new(state.db);

    match service.get_webhook(id).await {
        Ok(webhook) => {
            let log = WebhookLogEntry {
                id: webhook.id,
                name: webhook.name,
                url: webhook.url,
                events: webhook.events,
                is_active: webhook.is_active,
                last_triggered_at: webhook.last_triggered_at.map(|t| t.to_rfc3339()),
                last_status: webhook.last_status,
                retry_count: webhook.retry_count,
                max_retry_count: crate::services::webhook_service::MAX_RETRY_COUNT,
            };
            Ok(Json(ApiResponse::success(log)))
        }
        Err(e) => {
            tracing::error!("获取 Webhook 日志失败: {}", e);
            Err(AppError::internal(format!("获取 Webhook 日志失败: {}", e)))
        }
    }
}
