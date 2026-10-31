//! 敏感数据导出审批 Handler（生命周期改造版）
//!
//! 端点：
//!   1. POST /api/v1/erp/export-approvals - 创建审批请求（同时发起 BPM 流程）
//!   2. GET /api/v1/erp/export-approvals - 审批请求列表（脱敏出参）
//!   3. GET /api/v1/erp/export-approvals/:id - 审批请求详情（脱敏出参）
//!   4. POST /api/v1/erp/export-approvals/:id/approve - 审批通过（代理到 BPM 待办任务）
//!   5. POST /api/v1/erp/export-approvals/:id/reject - 审批拒绝（代理到 BPM 待办任务）
//!   6. POST /api/v1/erp/export-approvals/:id/cancel - 申请人取消（代理到 BPM 撤回）
//!   7. GET /api/v1/erp/export-approvals/verify-token - 校验下载 token（脱敏出参）
//!   8. GET /api/v1/erp/export-approvals/pending-for-me - 当前用户可见的在途申请
//!   9. GET /api/v1/erp/export-approvals/:id/token-once - 一次性取走令牌明文
//!      （第二次调用只回指纹，不再回明文；仅申请人本人）
//!  10. POST /api/v1/erp/export-approvals/:id/rotate-token - 重发令牌
//!      （旧令牌即刻作废；仅申请人本人）
//!
//! 权限映射（按 middleware/permission.rs URL 段派生）：
//!   - 创建/取消 → export-approvals:create / export-approvals:cancel
//!   - 查询/校验 → export-approvals:read（token-once 例外，见其函数注释）
//!   - 审批通过/拒绝 → export-approvals:approve / export-approvals:reject
//!   - token-once 末段不在 PATH_ACTION_KEYWORDS 词表内，派生动作回落到方法默认，
//!     即 GET→export-approvals:read；rotate-token 同理 POST→export-approvals:create。
//!     两枚键申请人本就持有；service 层再强制“仅申请人本人”身份门（双保险，
//!     不扩动作关键字表、不新增授权键）。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::export_approval_service::{
    ApproveRequest, CreateApprovalRequest, ExportApprovalService, ListApprovalQuery,
    TokenViewOutcome,
};
use crate::utils::error::AppError;
use crate::utils::optional_json::OptionalJson;
use crate::utils::response::ApiResponse;

/// POST /api/v1/erp/export-approvals
/// 创建敏感数据导出审批请求（落库后发起 BPM 流程，发起失败即补偿删除申请行）
pub async fn create_approval_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<CreateApprovalRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc
        .create_request(auth.user_id, auth.username.clone(), None, None, body)
        .await?;
    Ok(Json(ApiResponse::success(svc.detail_masked(&model))))
}

/// GET /api/v1/erp/export-approvals
/// 审批请求列表查询（非 admin 强制只看本人申请；出参已脱敏）
pub async fn list_approval_requests(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(mut q): Query<ListApprovalQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
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
/// 当前用户可见的在途审批任务（admin 看全部 pending，普通用户看他人申请；
/// 实际可办理性由 BPM 工单归属裁决）
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
/// 审批请求详情（仅申请人本人、审批人、admin 可见；出参脱敏）
pub async fn get_approval_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db.clone());
    let model = svc.get_request(id).await?;
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
    Ok(Json(ApiResponse::success(svc.detail_masked(&model))))
}

/// POST /api/v1/erp/export-approvals/:id/approve
/// 审批通过（代理到 BPM 待办任务；终审完成与否以 BPM 事件回写后的申请行状态为准）
pub async fn approve_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    // 审批意见选填：缺体经 OptionalJson 归一为「未采集」（等价体 comments:null）
    OptionalJson(body): OptionalJson<ApproveRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let body = body.unwrap_or(ApproveRequest { comments: None });
    let model = svc
        .approve(id, auth.user_id, auth.username.clone(), body)
        .await?;
    Ok(Json(ApiResponse::success(svc.detail_masked(&model))))
}

/// POST /api/v1/erp/export-approvals/:id/reject
/// 审批拒绝（代理到 BPM 待办任务）
pub async fn reject_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(body): Json<ApproveRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc
        .reject(id, auth.user_id, auth.username.clone(), body)
        .await?;
    Ok(Json(ApiResponse::success(svc.detail_masked(&model))))
}

/// POST /api/v1/erp/export-approvals/:id/cancel
/// 申请人取消（仅申请人本人，代理到 BPM 撤回实例）
pub async fn cancel_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let model = svc.cancel(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(svc.detail_masked(&model))))
}

/// GET /api/v1/erp/export-approvals/verify-token?token=xxx
/// 校验下载 token（出参脱敏，不回显任何凭据原文；校验失败由 service 给区分态文案）
pub async fn verify_token(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<VerifyTokenQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let svc = ExportApprovalService::new(state.db);
    let model = svc.verify_download_token(&q.token).await?;
    // 行级归属门：表无 department_id 列，按申请人字段判定归属人可见性
    let owner = Some(model.applicant_user_id);
    if !crate::utils::data_scope::check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权访问该导出审批记录（数据范围限制）".to_string(),
        ));
    }
    Ok(Json(ApiResponse::success(svc.detail_masked(&model))))
}

/// GET /api/v1/erp/export-approvals/:id/token-once
/// 一次性取走令牌明文：仅申请人本人、仅第一次；此后任何接口不再出现明文。
/// 已取过再调用只回指纹与引导重发的提示，不回任何凭据原文。
pub async fn view_token_once(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    match svc.view_token_once(id, auth.user_id).await? {
        TokenViewOutcome::Token(plaintext) => Ok(Json(ApiResponse::success(serde_json::json!({
            "available": true,
            "token": plaintext,
            "message": "令牌仅显示这一次，请立即用于导出；页面关闭或刷新后将无法再次查看",
        })))),
        TokenViewOutcome::AlreadyViewed { fingerprint } => {
            Ok(Json(ApiResponse::success(serde_json::json!({
                "available": false,
                "fingerprint": fingerprint,
                "message": "该令牌已被查看过，明文不再可见；如已丢失请重发令牌（旧令牌将作废）",
            }))))
        }
    }
}

/// POST /api/v1/erp/export-approvals/:id/rotate-token
/// 重发令牌：旧令牌即刻作废，签发新令牌并返回一次明文（仅申请人本人）
pub async fn rotate_token(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = ExportApprovalService::new(state.db);
    let issued = svc.rotate_token(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "available": true,
        "token": issued.plaintext,
        "fingerprint": issued.fingerprint,
        "message": "旧令牌已作废，新令牌仅显示这一次，请立即用于导出",
    }))))
}

/// 校验 token 查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct VerifyTokenQuery {
    pub token: String,
}
