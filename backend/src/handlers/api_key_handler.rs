use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::api_key_service::ApiKeyService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Option<String>,
    pub rate_limit_per_minute: Option<i32>,
    pub expires_days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: i32,
    pub name: String,
    pub key_prefix: String,
    pub permissions: Option<String>,
    pub rate_limit_per_minute: i32,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub api_key: ApiKeyResponse,
    pub plain_key: String,
}

impl From<crate::models::api_key::Model> for ApiKeyResponse {
    fn from(model: crate::models::api_key::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            key_prefix: model.key_prefix,
            permissions: model.permissions,
            rate_limit_per_minute: model.rate_limit_per_minute,
            expires_at: model.expires_at.map(|d| d.to_rfc3339()),
            created_at: model.created_at.to_rfc3339(),
        }
    }
}

/// BE-A/H 统一（2026-06-26）：错误类型从 StatusCode 改为 AppError
pub async fn create_api_key(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiResponse<CreateApiKeyResponse>>, AppError> {
    let service = ApiKeyService::new(state.db);
    let rate_limit = req.rate_limit_per_minute.unwrap_or(100);

    let (model, plain_key) = service
        .create_api_key(
            &req.name,
            req.permissions.as_deref(),
            rate_limit,
            req.expires_days,
        )
        .await?;
    let response = CreateApiKeyResponse {
        api_key: ApiKeyResponse::from(model),
        plain_key,
    };
    Ok(Json(ApiResponse::success(response)))
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ApiKeyResponse>>>, AppError> {
    let service = ApiKeyService::new(state.db);
    let keys = service.list_api_keys().await?;
    let responses: Vec<ApiKeyResponse> = keys.into_iter().map(ApiKeyResponse::from).collect();
    Ok(Json(ApiResponse::success(responses)))
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = ApiKeyService::new(state.db);

    // 漏洞 #5 修复：传入 AppCache 以启用 key_hash 黑名单
    service.revoke_api_key(id, Some(&state.cache)).await?;
    Ok(Json(ApiResponse::success_with_message((), "撤销成功")))
}
