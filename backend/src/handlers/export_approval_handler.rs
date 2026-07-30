//! V15 P0-S14 敏感数据导出二级审批 Handler
//!
//! 实现 8 个 HTTP 端点：
//!   1. POST /api/v1/erp/export-approvals - 创建审批请求
//!   2. GET /api/v1/erp/export-approvals - 审批请求列表
//!   3. GET /api/v1/erp/export-approvals/:id - 审批请求详情
//!   4. POST /api/v1/erp/export-approvals/:id/approve - 审批通过
//!   5. POST /api/v1/erp/export-approvals/:id/reject - 审批拒绝
//!   6. POST /api/v1/erp/export-approvals/:id/cancel - 申请人取消
//!   7. GET /api/v1/erp/export-approvals/verify-token - 校验下载 token
//!   8. GET /api/v1/erp/export-approvals/pending-for-me - 当前用户的待审批任务（V15 P2-05）
//!
//! 设计依据：V15 审计报告 类十三 P0-S14
//! 关联文件：services/export_approval_service.rs / models/export_approval_request.rs / migration 047
//!
//! 权限映射：
//!   - 创建审批请求 → export-approval:create
//!   - 列表/详情查询 → export-approval:read
//!   - 审批通过/拒绝 → export-approval:approve
//!   - 取消 → export-approval:create（仅申请人本人）
//!   - 校验 token → export-approval:read（导出 handler 内部调用）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::export_approval_service::{
    ApproveRequest, CreateApprovalRequest, ExportApprovalService, ListApprovalQuery,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// POST /api/v1/erp/export-approvals
/// 创建敏感数据导出审批请求
pub async fn create_approval_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<CreateApprovalRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc
        .create_request(auth.user_id, auth.username.clone(), None, None, body)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// GET /api/v1/erp/export-approvals
/// 审批请求列表查询
pub async fn list_approval_requests(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(mut q): Query<ListApprovalQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // V15 主线审计 High 修复：非 admin 角色强制仅看自己的申请；admin 可看全部。
    let is_admin = match auth.role_id {
        Some(rid) => crate::utils::admin_checker::is_admin_role(&state.db, rid).await,
        None => false,
    };
    if !is_admin {
        // 强制覆盖为当前用户，避免被 query 参数绕开
        q.applicant_user_id = Some(auth.user_id);
    }
    let svc = ExportApprovalService::new(state.db);
    let vo = svc.list_requests(q).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": vo.items,
        "total": vo.total,
        "page": vo.page,
        "page_size": vo.page_size,
    }))))
}

/// GET /api/v1/erp/export-approvals/pending-for-me
/// V15 主线审计 P2-05 修复：当前用户作为审批人/二级审批人/管理员的待办任务。
/// - admin 角色：所有 pending/pending_l2 任务；
/// - 普通用户：自己作为审批人(approver_user_id = self) 或 申请人≠自己 且 状态在 pending/pending_l2 的任务。
pub async fn list_pending_for_me(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<ListApprovalQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let is_admin = match auth.role_id {
        Some(rid) => crate::utils::admin_checker::is_admin_role(&state.db, rid).await,
        None => false,
    };
    let svc = ExportApprovalService::new(state.db);
    let vo = svc.list_pending_for_user(auth.user_id, is_admin, q).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": vo.items,
        "total": vo.total,
        "page": vo.page,
        "page_size": vo.page_size,
    }))))
}

/// GET /api/v1/erp/export-approvals/:id
/// 审批请求详情
pub async fn get_approval_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db.clone());
    let model = svc.get_request(id).await?;
    // V15 主线审计 High 修复：仅申请人本人、审批人、admin 可查看详情。
    let is_admin = match auth.role_id {
        Some(rid) => crate::utils::admin_checker::is_admin_role(&state.db, rid).await,
        None => false,
    };
    let is_approver = model
        .approver_user_id
        .map(|u| u == auth.user_id)
        .unwrap_or(false);
    if !is_admin && model.applicant_user_id != auth.user_id && !is_approver {
        return Err(AppError::permission_denied(
            "仅申请人/审批人/管理员可查看该审批单",
        ));
    }
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// POST /api/v1/erp/export-approvals/:id/approve
/// 审批通过（一级或二级）
pub async fn approve_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(body): Json<ApproveRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc
        .approve(id, auth.user_id, auth.username.clone(), None, body)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// POST /api/v1/erp/export-approvals/:id/reject
/// 审批拒绝
pub async fn reject_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(body): Json<ApproveRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc
        .reject(id, auth.user_id, auth.username.clone(), None, body)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// POST /api/v1/erp/export-approvals/:id/cancel
/// 申请人取消（仅申请人本人）
pub async fn cancel_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc.cancel(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// GET /api/v1/erp/export-approvals/verify-token?token=xxx
/// 校验下载 token（导出 handler 调用前校验）
pub async fn verify_token(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<VerifyTokenQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc.verify_download_token(&q.token).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 校验 token 查询参数
#[derive(Debug, Deserialize)]
pub struct VerifyTokenQuery {
    pub token: String,
}
