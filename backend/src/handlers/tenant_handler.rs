use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::tenant_service::TenantService;
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub plan_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
}

impl From<crate::models::tenant::Model> for TenantResponse {
    fn from(model: crate::models::tenant::Model) -> Self {
        Self {
            id: model.id,
            code: model.code,
            name: model.name,
            status: model.status,
            created_at: model.created_at.to_rfc3339(),
        }
    }
}

/// 创建租户
pub async fn create_tenant(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<ApiResponse<TenantResponse>>, StatusCode> {
    let service = TenantService::new(state.db);
    
    match service.create_tenant(&req.code, &req.name, req.description.as_deref(), req.plan_id).await {
        Ok(tenant) => Ok(Json(ApiResponse::success(TenantResponse::from(tenant)))),
        Err(e) => {
            tracing::error!("创建租户失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取租户列表
#[derive(Debug, Deserialize)]
pub struct ListTenantsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub async fn list_tenants(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ListTenantsQuery>,
) -> Result<Json<ApiResponse<Vec<TenantResponse>>>, StatusCode> {
    let service = TenantService::new(state.db);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    match service.list_tenants(page, page_size).await {
        Ok((tenants, _total)) => {
            let responses: Vec<TenantResponse> = tenants.into_iter().map(TenantResponse::from).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("获取租户列表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取单个租户
pub async fn get_tenant(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<TenantResponse>>, StatusCode> {
    let service = TenantService::new(state.db);

    match service.get_tenant(id).await {
        Ok(Some(tenant)) => Ok(Json(ApiResponse::success(TenantResponse::from(tenant)))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("获取租户失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 更新租户状态
#[derive(Debug, Deserialize)]
pub struct UpdateTenantStatusRequest {
    pub status: String,
}

pub async fn update_tenant_status(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTenantStatusRequest>,
) -> Result<Json<ApiResponse<TenantResponse>>, StatusCode> {
    let service = TenantService::new(state.db);

    match service.update_tenant_status(id, &req.status).await {
        Ok(tenant) => Ok(Json(ApiResponse::success(TenantResponse::from(tenant)))),
        Err(e) => {
            tracing::error!("更新租户状态失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
