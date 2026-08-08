//! 客户团队协作与数据共享 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::crm::customer_team_share_service::{
    AddTeamMemberRequest, CustomerTeamShareService, RevokeShareRequest, ShareCustomerRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

/// 查询参数：customer_id
#[derive(Debug, Deserialize)]
pub struct CustomerIdQuery {
    pub customer_id: i32,
    pub status: Option<String>,
}

/// 查询参数：user_id + active_only
#[derive(Debug, Deserialize)]
pub struct UserTeamsQuery {
    pub user_id: i32,
    pub active_only: Option<bool>,
}

/// 查询参数：customer_id + user_id
#[derive(Debug, Deserialize)]
pub struct TeamMemberCheckQuery {
    pub customer_id: i32,
    pub user_id: i32,
}

/// 添加团队成员
pub async fn add_team_member(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<AddTeamMemberRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let dto = service.add_team_member(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(dto)?)))
}

/// 移除团队成员
pub async fn remove_team_member(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(member_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let dto = service.remove_team_member(member_id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(dto)?)))
}

/// 列出客户团队成员
pub async fn list_team_members(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(customer_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let list = service.list_team_members(customer_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 列出用户参与的团队
pub async fn list_user_teams(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<UserTeamsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let list = service
        .list_user_teams(q.user_id, q.active_only.unwrap_or(true))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 检查团队成员身份
pub async fn is_team_member(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<TeamMemberCheckQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let role = service.is_team_member(q.customer_id, q.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(role)?)))
}

/// 共享客户
pub async fn share_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ShareCustomerRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let dto = service.share_customer(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(dto)?)))
}

/// 撤销共享
pub async fn revoke_share(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RevokeShareRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let dto = service.revoke_share(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(dto)?)))
}

/// 列出客户共享记录
pub async fn list_customer_shares(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<CustomerIdQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let list = service
        .list_customer_shares(q.customer_id, q.status)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 列出用户收到的共享
pub async fn list_user_shares(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<UserTeamsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let list = service
        .list_user_shares(q.user_id, q.active_only.unwrap_or(true))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 检查共享权限
pub async fn check_share_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<TeamMemberCheckQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let perm = service
        .check_share_permission(q.customer_id, q.user_id)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(perm)?)))
}

/// 过期 overdue 共享
pub async fn expire_overdue_shares(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTeamShareService::new(state.db.clone());
    let result = service.expire_overdue_shares().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}
