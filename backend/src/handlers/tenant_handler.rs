use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::services::tenant_service::TenantService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTenantRequest {
    #[validate(length(min = 1, max = 50, message = "租户编码不能为空且最长50字符"))]
    pub code: String,
    #[validate(length(min = 1, max = 200, message = "租户名称不能为空且最长200字符"))]
    pub name: String,
    #[validate(length(max = 1000, message = "描述最长1000字符"))]
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
) -> Result<Json<ApiResponse<TenantResponse>>, AppError> {
    req.validate()?;

    let service = TenantService::new(state.db);
    let tenant = service
        .create_tenant(
            &req.code,
            &req.name,
            req.description.as_deref(),
            req.plan_id,
        )
        .await
        .map_err(|e| AppError::internal(format!("创建租户失败: {}", e)))?;

    Ok(Json(ApiResponse::success(TenantResponse::from(tenant))))
}

/// 获取租户列表
#[derive(Debug, Deserialize, Validate)]
pub struct ListTenantsQuery {
    #[validate(range(min = 1, message = "页码必须大于0"))]
    pub page: Option<u64>,
    #[validate(range(min = 1, max = 100, message = "每页数量必须在1-100之间"))]
    pub page_size: Option<u64>,
}

pub async fn list_tenants(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ListTenantsQuery>,
) -> Result<Json<ApiResponse<Vec<TenantResponse>>>, AppError> {
    query.validate()?;

    let service = TenantService::new(state.db);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (tenants, total) = service
        .list_tenants(page, page_size)
        .await
        .map_err(|e| AppError::internal(format!("获取租户列表失败: {}", e)))?;

    let responses: Vec<TenantResponse> = tenants.into_iter().map(TenantResponse::from).collect();
    Ok(Json(ApiResponse::success_paginated(
        responses, total, page, page_size,
    )))
}

/// 获取单个租户
pub async fn get_tenant(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<TenantResponse>>, AppError> {
    let service = TenantService::new(state.db);

    let tenant = service
        .get_tenant(id)
        .await
        .map_err(|e| AppError::internal(format!("获取租户失败: {}", e)))?
        .ok_or_else(|| AppError::not_found("租户不存在"))?;

    Ok(Json(ApiResponse::success(TenantResponse::from(tenant))))
}

/// 更新租户状态
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTenantStatusRequest {
    #[validate(length(min = 1, max = 20, message = "状态不能为空"))]
    pub status: String,
}

pub async fn update_tenant_status(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTenantStatusRequest>,
) -> Result<Json<ApiResponse<TenantResponse>>, AppError> {
    req.validate()?;

    let service = TenantService::new(state.db);

    let tenant = service
        .update_tenant_status(id, &req.status)
        .await
        .map_err(|e| AppError::internal(format!("更新租户状态失败: {}", e)))?;

    Ok(Json(ApiResponse::success(TenantResponse::from(tenant))))
}
