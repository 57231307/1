//! V15 P0-S14 敏感数据导出二级审批 Handler
//!
//! 实现 7 个 HTTP 端点：
//!   1. POST /api/v1/erp/export-approvals - 创建审批请求
//!   2. GET /api/v1/erp/export-approvals - 审批请求列表
//!   3. GET /api/v1/erp/export-approvals/:id - 审批请求详情
//!   4. POST /api/v1/erp/export-approvals/:id/approve - 审批通过
//!   5. POST /api/v1/erp/export-approvals/:id/reject - 审批拒绝
//!   6. POST /api/v1/erp/export-approvals/:id/cancel - 申请人取消
//!   7. GET /api/v1/erp/export-approvals/verify-token - 校验下载 token
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

use crate::middleware::auth_context::AuthContext;
use crate::services::export_approval_service::{
    ApproveRequest, CreateApprovalRequest, ExportApprovalService, ListApprovalQuery,
};
use crate::utils::app_state::AppState;
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
        .create_request(
            auth.user_id,
            auth.username.clone(),
            None,
            None,
            body,
        )
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// GET /api/v1/erp/export-approvals
/// 审批请求列表查询
pub async fn list_approval_requests(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<ListApprovalQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let vo = svc.list_requests(q).await?;
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
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc.get_request(id).await?;
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
        .approve(
            id,
            auth.user_id,
            auth.username.clone(),
            None,
            body,
        )
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
        .reject(
            id,
            auth.user_id,
            auth.username.clone(),
            None,
            body,
        )
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
