use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::webhook_service::WebhookService;
use crate::utils::app_state::AppState;
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
    auth: AuthContext,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<Json<ApiResponse<WebhookResponse>>, StatusCode> {
    let tenant_id = auth.tenant_id.ok_or(StatusCode::BAD_REQUEST)?;
    let service = WebhookService::new(state.db);
    let events: Vec<&str> = req.events.iter().map(|s| s.as_str()).collect();

    match service
        .create_webhook(
            tenant_id,
            &req.name,
            &req.url,
            &events,
            req.secret.as_deref(),
        )
        .await
    {
        Ok(webhook) => Ok(Json(ApiResponse::success(WebhookResponse::from(webhook)))),
        Err(e) => {
            tracing::error!("创建 Webhook 失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn list_webhooks(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<WebhookResponse>>>, StatusCode> {
    let tenant_id = auth.tenant_id.ok_or(StatusCode::BAD_REQUEST)?;
    let service = WebhookService::new(state.db);

    match service.list_webhooks(tenant_id).await {
        Ok(webhooks) => {
            let responses: Vec<WebhookResponse> =
                webhooks.into_iter().map(WebhookResponse::from).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("获取 Webhook 列表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_webhook(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let service = WebhookService::new(state.db);
    let tenant_id = auth.tenant_id.unwrap_or(0);

    match service.delete_webhook(id, tenant_id).await {
        Ok(()) => Ok(Json(ApiResponse::success_with_message((), "删除成功"))),
        Err(e) => {
            tracing::error!("删除 Webhook 失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
