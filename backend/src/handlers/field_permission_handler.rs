use crate::middleware::auth_context::AuthContext;
use crate::services::field_permission_service::{
    CreateFieldPermissionRequest, FieldPermissionService, UpdateFieldPermissionRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

/// 字段权限响应
#[derive(Debug, Serialize)]
pub struct FieldPermissionResponse {
    pub id: i32,
    pub role_id: i32,
    pub resource_type: String,
    pub field_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub mask_strategy: String,
    pub is_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 字段权限列表查询参数
#[derive(Debug, Deserialize)]
pub struct FieldPermissionQuery {
    pub resource_type: Option<String>,
    pub role_id: Option<i32>,
}

/// 创建字段权限请求体
#[derive(Debug, Deserialize)]
pub struct CreateFieldPermissionPayload {
    pub role_id: i32,
    pub resource_type: String,
    pub field_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub mask_strategy: Option<String>,
}

/// 更新字段权限请求体
#[derive(Debug, Deserialize)]
pub struct UpdateFieldPermissionPayload {
    pub can_read: Option<bool>,
    pub can_write: Option<bool>,
    pub mask_strategy: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 获取字段权限列表
pub async fn list_field_permissions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<FieldPermissionQuery>,
) -> Result<Json<ApiResponse<Vec<FieldPermissionResponse>>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    let permissions = service
        .list_field_permissions(query.resource_type.as_deref(), query.role_id)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    let responses: Vec<FieldPermissionResponse> = permissions
        .into_iter()
        .map(|p| FieldPermissionResponse {
            id: p.id,
            role_id: p.role_id,
            resource_type: p.resource_type,
            field_name: p.field_name,
            can_read: p.can_read,
            can_write: p.can_write,
            mask_strategy: p.mask_strategy,
            is_enabled: p.is_enabled,
            created_at: p.created_at,
            updated_at: p.updated_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 获取字段权限详情
pub async fn get_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<FieldPermissionResponse>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    let perm = service
        .get_field_permission(id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    Ok(Json(ApiResponse::success(FieldPermissionResponse {
        id: perm.id,
        role_id: perm.role_id,
        resource_type: perm.resource_type,
        field_name: perm.field_name,
        can_read: perm.can_read,
        can_write: perm.can_write,
        mask_strategy: perm.mask_strategy,
        is_enabled: perm.is_enabled,
        created_at: perm.created_at,
        updated_at: perm.updated_at,
    })))
}

/// 创建字段权限
pub async fn create_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<CreateFieldPermissionPayload>,
) -> Result<Json<ApiResponse<FieldPermissionResponse>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    let request = CreateFieldPermissionRequest {
        role_id: payload.role_id,
        resource_type: payload.resource_type,
        field_name: payload.field_name,
        can_read: payload.can_read,
        can_write: payload.can_write,
        mask_strategy: payload.mask_strategy,
    };

    let perm = service
        .create_field_permission(request)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(FieldPermissionResponse {
        id: perm.id,
        role_id: perm.role_id,
        resource_type: perm.resource_type,
        field_name: perm.field_name,
        can_read: perm.can_read,
        can_write: perm.can_write,
        mask_strategy: perm.mask_strategy,
        is_enabled: perm.is_enabled,
        created_at: perm.created_at,
        updated_at: perm.updated_at,
    })))
}

/// 更新字段权限
pub async fn update_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateFieldPermissionPayload>,
) -> Result<Json<ApiResponse<FieldPermissionResponse>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    let request = UpdateFieldPermissionRequest {
        can_read: payload.can_read,
        can_write: payload.can_write,
        mask_strategy: payload.mask_strategy,
        is_enabled: payload.is_enabled,
    };

    let perm = service
        .update_field_permission(id, request)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(FieldPermissionResponse {
        id: perm.id,
        role_id: perm.role_id,
        resource_type: perm.resource_type,
        field_name: perm.field_name,
        can_read: perm.can_read,
        can_write: perm.can_write,
        mask_strategy: perm.mask_strategy,
        is_enabled: perm.is_enabled,
        created_at: perm.created_at,
        updated_at: perm.updated_at,
    })))
}

/// 删除字段权限
pub async fn delete_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    service
        .delete_field_permission(id)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}
