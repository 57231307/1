use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct CreateWebhookIntegrationRequest {
    pub name: String,
    pub platform: String,
    pub webhook_url: String,
    pub secret: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SendWebhookMessageRequest {
    pub integration_id: i32,
    // 批次 110 P0-2：message_type 接入业务，支持 text/markdown 两种消息类型
    pub message_type: String,
    pub content: String,
    // 批次 110 P0-2：title 用于 markdown 类型消息的标题
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookCallbackRequest {
    pub event_type: String,
    // 批次 110 P0-3：payload 接入业务（持久化日志 + 回执摘要），不再标注 dead_code
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct WebhookIntegrationItem {
    pub id: i32,
    pub name: String,
    pub platform: String,
    pub webhook_url: String,
    pub is_active: bool,
    pub last_triggered_at: Option<String>,
    pub last_status: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookSendResult {
    pub message_id: String,
    pub platform: String,
    pub status: String,
    pub sent_at: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookCallbackResult {
    pub received: bool,
    pub event_type: String,
    pub processed_at: String,
    /// 批次 110 P0-3：payload 处理回执摘要
    /// - payload_size：原始 payload 序列化后的字节大小
    /// - payload_keys：若 payload 为 Object，则记录其顶层字段名（最多 10 个），便于调用方核对
    pub payload_size: usize,
    pub payload_keys: Vec<String>,
}

pub async fn list_integrations(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<WebhookIntegrationItem>>>, AppError> {
    use crate::models::webhook;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let webhooks = webhook::Entity::find()
        .filter(webhook::Column::IsActive.eq(true))
        .all(state.db.as_ref())
        .await?;

    let items: Vec<WebhookIntegrationItem> = webhooks
        .into_iter()
        .map(|w| WebhookIntegrationItem {
            id: w.id,
            name: w.name,
            platform: "GENERIC".to_string(),
            webhook_url: w.url,
            is_active: w.is_active,
            last_triggered_at: w.last_triggered_at.map(|t| t.to_rfc3339()),
            last_status: w.last_status,
            created_at: w.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

pub async fn create_integration(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateWebhookIntegrationRequest>,
) -> Result<Json<ApiResponse<WebhookIntegrationItem>>, AppError> {
    use crate::models::webhook;
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, Set};

    let now = Utc::now();
    let active_model = webhook::ActiveModel {
        name: Set(req.name),
        url: Set(req.webhook_url),
        events: Set("*".to_string()),
        secret: Set(req.secret),
        is_active: Set(req.is_active.unwrap_or(true)),
        last_triggered_at: Set(None),
        last_status: Set(None),
        retry_count: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let webhook = active_model.insert(state.db.as_ref()).await?;

    Ok(Json(ApiResponse::success_with_message(
        WebhookIntegrationItem {
            id: webhook.id,
            name: webhook.name,
            platform: req.platform,
            webhook_url: webhook.url,
            is_active: webhook.is_active,
            last_triggered_at: None,
            last_status: None,
            created_at: webhook.created_at.to_rfc3339(),
        },
        "Webhook 集成创建成功",
    )))
}

pub async fn delete_integration(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 推荐使用服务层处理删除逻辑（它已经包含了权限检查）
    let service = crate::services::webhook_service::WebhookService::new(state.db.clone());
    service.delete_webhook(id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "Webhook 集成已删除",
    )))
}

pub async fn send_wechat_message(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<SendWebhookMessageRequest>,
) -> Result<Json<ApiResponse<WebhookSendResult>>, AppError> {
    if req.content.is_empty() {
        return Err(AppError::bad_request("消息内容不能为空"));
    }

    // 批次 110 P0-2：根据 message_type 构建不同企业微信消息格式
    // 支持 text（纯文本）和 markdown（markdown 格式）两种类型
    let payload = match req.message_type.as_str() {
        "text" => serde_json::json!({
            "msgtype": "text",
            "text": {
                "content": req.content
            }
        }),
        "markdown" => serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "content": req.content
            }
        }),
        other => {
            return Err(AppError::bad_request(format!(
                "不支持的消息类型：{}，仅支持 text/markdown",
                other
            )));
        }
    };

    // 通过WebhookService发送
    use crate::services::webhook_service::WebhookService;
    let service = WebhookService::new(state.db.clone());

    let delivery = service
        .trigger_webhook(req.integration_id, "wechat_message", &payload.to_string())
        .await?;

    let result = WebhookSendResult {
        message_id: uuid::Uuid::new_v4().to_string(),
        platform: "WECHAT_WORK".to_string(),
        status: if delivery.success { "SENT" } else { "FAILED" }.to_string(),
        sent_at: chrono::Utc::now().to_rfc3339(),
    };

    if delivery.success {
        Ok(Json(ApiResponse::success_with_message(
            result,
            "企业微信消息发送成功",
        )))
    } else {
        Err(AppError::internal(
            delivery.error.unwrap_or_else(|| "发送失败".to_string()),
        ))
    }
}

pub async fn send_dingtalk_message(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<SendWebhookMessageRequest>,
) -> Result<Json<ApiResponse<WebhookSendResult>>, AppError> {
    if req.content.is_empty() {
        return Err(AppError::bad_request("消息内容不能为空"));
    }

    // 批次 110 P0-2：根据 message_type 构建不同钉钉消息格式
    // 支持 text（纯文本）和 markdown（markdown 格式，需 title）两种类型
    let payload = match req.message_type.as_str() {
        "text" => serde_json::json!({
            "msgtype": "text",
            "text": {
                "content": req.content
            }
        }),
        "markdown" => {
            // 钉钉 markdown 消息需要 title 字段，若未提供则使用默认标题
            let title = req.title.as_deref().unwrap_or("通知");
            serde_json::json!({
                "msgtype": "markdown",
                "markdown": {
                    "title": title,
                    "text": req.content
                }
            })
        }
        other => {
            return Err(AppError::bad_request(format!(
                "不支持的消息类型：{}，仅支持 text/markdown",
                other
            )));
        }
    };

    // 通过WebhookService发送
    use crate::services::webhook_service::WebhookService;
    let service = WebhookService::new(state.db.clone());

    let delivery = service
        .trigger_webhook(req.integration_id, "dingtalk_message", &payload.to_string())
        .await?;

    let result = WebhookSendResult {
        message_id: uuid::Uuid::new_v4().to_string(),
        platform: "DINGTALK".to_string(),
        status: if delivery.success { "SENT" } else { "FAILED" }.to_string(),
        sent_at: chrono::Utc::now().to_rfc3339(),
    };

    if delivery.success {
        Ok(Json(ApiResponse::success_with_message(
            result,
            "钉钉消息发送成功",
        )))
    } else {
        Err(AppError::internal(
            delivery.error.unwrap_or_else(|| "发送失败".to_string()),
        ))
    }
}

pub async fn handle_generic_callback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<Json<ApiResponse<WebhookCallbackResult>>, AppError> {
    // 1. 提取签名头
    let signature = headers
        .get("X-Webhook-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("缺少签名头 X-Webhook-Signature"))?;

    // 2. 获取 Webhook 密钥
    // M-2 修复：使用独立的 webhook_secret（与 jwt_secret 分离），
    // 避免 JWT 密钥泄露导致第三方回调被任意伪造。
    let webhook_secret = &state.webhook_secret;

    // 3. 验证签名
    crate::utils::webhook_signature::verify_webhook_signature(&body, webhook_secret, signature)?;

    // 4. 解析 payload
    let req: WebhookCallbackRequest = serde_json::from_str(&body)
        .map_err(|e| AppError::validation(format!("无效的 JSON 格式：{}", e)))?;

    // 批次 110 P0-3：将完整 payload 写入结构化日志（替代原仅记录 event_type 的占位实现）
    // 第三方平台回调通常是异步业务事件的入口，payload 内含业务关键字段（订单号/付款号/状态变更等）。
    // 在未独立持久化到 webhook_logs 表前，先通过 tracing 输出到日志聚合系统，便于：
    // 1) 调用方核对回执摘要是否与发送内容一致
    // 2) 业务侧通过日志检索回溯第三方回调历史
    // 3) 后续接入 webhook_logs 表时可作为数据源迁移
    tracing::info!(
        event_type = %req.event_type,
        payload = %req.payload,
        "Webhook 签名验证通过，已接收第三方回调事件"
    );

    // 计算 payload 摘要：序列化字节大小 + 顶层字段名（若为 Object）
    let payload_size = req.payload.to_string().len();
    let payload_keys: Vec<String> = match &req.payload {
        serde_json::Value::Object(map) => {
            map.keys().take(10).cloned().collect()
        }
        _ => Vec::new(),
    };

    let result = WebhookCallbackResult {
        received: true,
        event_type: req.event_type,
        processed_at: chrono::Utc::now().to_rfc3339(),
        payload_size,
        payload_keys,
    };

    Ok(Json(ApiResponse::success(result)))
}

pub async fn test_integration(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::services::webhook_service::WebhookService;

    let service = WebhookService::new(state.db.clone());

    let mut result = service.test_webhook(id).await?;

    // SSRF 缓解：测试接口不回显目标响应体，防止攻击者读取内网数据
    result.response_body = Some("出于安全原因，已隐藏响应内容".to_string());

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result).unwrap_or_default(),
        "测试消息已发送",
    )))
}
