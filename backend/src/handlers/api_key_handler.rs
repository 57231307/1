use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::extract_tenant_id;
use crate::services::api_key_service::ApiKeyService;
use crate::utils::app_state::AppState;
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

pub async fn create_api_key(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiResponse<CreateApiKeyResponse>>, StatusCode> {
    let tenant_id = auth.tenant_id.ok_or(StatusCode::BAD_REQUEST)?;
    let service = ApiKeyService::new(state.db);
    let rate_limit = req.rate_limit_per_minute.unwrap_or(100);

    match service
        .create_api_key(
            tenant_id,
            &req.name,
            req.permissions.as_deref(),
            rate_limit,
            req.expires_days,
        )
        .await
    {
        Ok((model, plain_key)) => {
            let response = CreateApiKeyResponse {
                api_key: ApiKeyResponse::from(model),
                plain_key,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("创建 API 密钥失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ApiKeyResponse>>>, StatusCode> {
    let tenant_id = auth.tenant_id.ok_or(StatusCode::BAD_REQUEST)?;
    let service = ApiKeyService::new(state.db);

    match service.list_api_keys(tenant_id).await {
        Ok(keys) => {
            let responses: Vec<ApiKeyResponse> =
                keys.into_iter().map(ApiKeyResponse::from).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("获取 API 密钥列表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let service = ApiKeyService::new(state.db);
    let tenant_id = match extract_tenant_id(&auth) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("获取租户ID失败: {}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 漏洞 #5 修复：传入 AppCache 以启用 key_hash 黑名单
    match service
        .revoke_api_key(id, tenant_id, Some(&state.cache))
        .await
    {
        Ok(()) => Ok(Json(ApiResponse::success_with_message((), "撤销成功"))),
        Err(e) => {
            tracing::error!("撤销 API 密钥失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
