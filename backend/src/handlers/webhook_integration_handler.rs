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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SendWebhookMessageRequest {
    pub integration_id: i32,
    pub message_type: String,
    pub content: String,
    pub title: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct WebhookCallbackRequest {
    pub event_type: String,
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
    auth: AuthContext,
    Json(req): Json<CreateWebhookIntegrationRequest>,
) -> Result<Json<ApiResponse<WebhookIntegrationItem>>, AppError> {
    let tenant_id = auth.tenant_id.unwrap_or(0);

    use crate::models::webhook;
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, Set};

    let now = Utc::now();
    let active_model = webhook::ActiveModel {
        tenant_id: Set(tenant_id),
        name: Set(req.name.clone()),
        url: Set(req.webhook_url.clone()),
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
    use crate::models::webhook;
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let webhook = webhook::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::ResourceNotFound("Webhook 集成不存在".to_string()))?;

    let mut active_model: webhook::ActiveModel = webhook.into();
    active_model.is_active = Set(false);
    active_model.updated_at = Set(Utc::now());
    active_model.update(state.db.as_ref()).await?;

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
        return Err(AppError::BadRequest("消息内容不能为空".to_string()));
    }

    // 构建企业微信消息格式
    let payload = serde_json::json!({
        "msgtype": "text",
        "text": {
            "content": req.content
        }
    });

    // 通过WebhookService发送
    use crate::services::webhook_service::WebhookService;
    let service = WebhookService::new(state.db.clone());
    let delivery = service
        .trigger_webhook(req.integration_id, "wechat_message", &payload.to_string())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
        Err(AppError::InternalError(
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
        return Err(AppError::BadRequest("消息内容不能为空".to_string()));
    }

    // 构建钉钉消息格式
    let payload = serde_json::json!({
        "msgtype": "text",
        "text": {
            "content": req.content
        }
    });

    // 通过WebhookService发送
    use crate::services::webhook_service::WebhookService;
    let service = WebhookService::new(state.db.clone());
    let delivery = service
        .trigger_webhook(req.integration_id, "dingtalk_message", &payload.to_string())
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
        Err(AppError::InternalError(
            delivery.error.unwrap_or_else(|| "发送失败".to_string()),
        ))
    }
}

pub async fn handle_generic_callback(
    State(_state): State<AppState>,
    Json(req): Json<WebhookCallbackRequest>,
) -> Result<Json<ApiResponse<WebhookCallbackResult>>, AppError> {
    tracing::info!("收到通用 Webhook 回调: event_type={}", req.event_type);

    let result = WebhookCallbackResult {
        received: true,
        event_type: req.event_type,
        processed_at: chrono::Utc::now().to_rfc3339(),
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
    let result = service
        .test_webhook(id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result).unwrap_or_default(),
        "测试消息已发送",
    )))
}
