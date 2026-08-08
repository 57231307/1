//! 角色变更审批 Handler
//!
//! B12-P2-4：敏感角色变更双人审批

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::role_change_approval_service::{
    ApproveRoleChangeRequest, CreateRoleChangeApprovalRequest, RoleChangeApprovalQuery,
    RoleChangeApprovalService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 创建审批请求
/// POST /api/v1/erp/role-change-approvals
pub async fn create_approval(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateRoleChangeApprovalRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "创建角色变更审批请求");

    // 检查目标角色是否为敏感角色
    if !RoleChangeApprovalService::is_sensitive_role(&req.target_role_code) {
        return Err(AppError::business("只有敏感角色变更需要审批"));
    }

    let service = RoleChangeApprovalService::new(state.db.clone());
    let approval = service
        .create_request(auth.user_id, auth.username.clone(), req)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(approval)?,
        "审批请求已创建",
    )))
}

/// 一级审批
/// POST /api/v1/erp/role-change-approvals/:id/approve-l1
pub async fn approve_l1(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ApproveRoleChangeRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, approval_id = id, "一级审批");

    // 检查是否为管理员
    if !crate::utils::admin_checker::is_admin_role(&state.db, auth.role_id.unwrap_or(0)).await {
        return Err(AppError::permission_denied("只有管理员可以审批"));
    }

    let service = RoleChangeApprovalService::new(state.db.clone());
    let approval = service.approve_l1(id, auth.user_id, req).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(approval)?,
        "一级审批通过",
    )))
}

/// 二级审批
/// POST /api/v1/erp/role-change-approvals/:id/approve-l2
pub async fn approve_l2(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ApproveRoleChangeRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, approval_id = id, "二级审批");

    // 检查是否为管理员
    if !crate::utils::admin_checker::is_admin_role(&state.db, auth.role_id.unwrap_or(0)).await {
        return Err(AppError::permission_denied("只有管理员可以审批"));
    }

    let service = RoleChangeApprovalService::new(state.db.clone());
    let approval = service.approve_l2(id, auth.user_id, req).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(approval)?,
        "二级审批通过，变更已批准",
    )))
}

/// 拒绝审批
/// POST /api/v1/erp/role-change-approvals/:id/reject
pub async fn reject_approval(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ApproveRoleChangeRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, approval_id = id, "拒绝审批");

    // 检查是否为管理员
    if !crate::utils::admin_checker::is_admin_role(&state.db, auth.role_id.unwrap_or(0)).await {
        return Err(AppError::permission_denied("只有管理员可以审批"));
    }

    let service = RoleChangeApprovalService::new(state.db.clone());
    let approval = service.reject(id, auth.user_id, req).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(approval)?,
        "审批已拒绝",
    )))
}

/// 取消审批
/// POST /api/v1/erp/role-change-approvals/:id/cancel
pub async fn cancel_approval(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, approval_id = id, "取消审批");

    let service = RoleChangeApprovalService::new(state.db.clone());
    let approval = service.cancel(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(approval)?,
        "审批已取消",
    )))
}

/// 查询审批详情
/// GET /api/v1/erp/role-change-approvals/:id
pub async fn get_approval(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, approval_id = id, "查询审批详情");

    let service = RoleChangeApprovalService::new(state.db.clone());
    let approval = service.get_by_id(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(approval)?)))
}

/// 查询审批列表
/// GET /api/v1/erp/role-change-approvals
pub async fn list_approvals(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<RoleChangeApprovalQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "查询审批列表");

    let service = RoleChangeApprovalService::new(state.db.clone());
    let result = service.list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}
