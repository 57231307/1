//! 权限委托 handler（V15 P1 12.6）

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::permission_delegation_service::{
    CreateDelegationRequest, PermissionDelegationService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

/// 查询参数：委托列表查询
#[derive(Debug, Deserialize)]
pub struct DelegationListQuery {
    pub user_id: Option<i32>,
    pub as_delegator: Option<bool>,
}

/// 查询参数：委托权限检查
#[derive(Debug, Deserialize)]
pub struct DelegatedPermissionQuery {
    pub delegatee_id: i32,
    pub permission_code: String,
}

/// 撤销委托请求体
#[derive(Debug, Deserialize)]
pub struct RevokeDelegationRequest {
    pub revoke_reason: Option<String>,
}

/// 创建权限委托
pub async fn create_delegation(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateDelegationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PermissionDelegationService::new(state.db.clone());
    let model = service.create_delegation(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 撤销权限委托
pub async fn revoke_delegation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(delegation_id): Path<i64>,
    Json(req): Json<RevokeDelegationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PermissionDelegationService::new(state.db.clone());
    service
        .revoke_delegation(delegation_id, auth.user_id, req.revoke_reason)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::Value::Null)))
}

/// 查询用户当前有效的委托权限
pub async fn get_active_delegated_permissions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(delegatee_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PermissionDelegationService::new(state.db.clone());
    let list = service
        .get_active_delegated_permissions(delegatee_id)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 检查用户是否拥有某委托权限
pub async fn has_delegated_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<DelegatedPermissionQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PermissionDelegationService::new(state.db.clone());
    let has = service
        .has_delegated_permission(params.delegatee_id, &params.permission_code)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(has)?)))
}

/// 扫描并标记过期委托（定时任务调用）
pub async fn expire_overdue_delegations(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PermissionDelegationService::new(state.db.clone());
    let count = service.expire_overdue_delegations().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(count)?)))
}

/// 查询委托记录列表
pub async fn list_delegations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<DelegationListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PermissionDelegationService::new(state.db.clone());
    let list = service
        .list_delegations(params.user_id, params.as_delegator.unwrap_or(false))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 获取委托详情
pub async fn get_delegation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(delegation_id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PermissionDelegationService::new(state.db.clone());
    let model = service.get_delegation(delegation_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}
