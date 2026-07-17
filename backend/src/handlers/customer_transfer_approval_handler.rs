//! 客户转移审批 Handler
//!
//! V15 P0-S08 修复：提供客户转移审批流 HTTP 接口
//!
//! 路由：
//! - POST   /api/v1/erp/crm/transfer-approvals              创建转移审批申请
//! - GET    /api/v1/erp/crm/transfer-approvals              查询审批列表
//! - GET    /api/v1/erp/crm/transfer-approvals/:id          获取审批详情
//! - POST   /api/v1/erp/crm/transfer-approvals/:id/cancel   申请人取消审批
//! - POST   /api/v1/erp/crm/transfer-approvals/:id/manager-approve  销售经理审批
//! - POST   /api/v1/erp/crm/transfer-approvals/:id/director-approve 总监审批（大客户）

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::middleware::auth_context::AuthContext;
use crate::services::crm::customer_transfer_approval_service::{
    ApproveRequest, ApprovalQuery, CreateTransferApprovalRequest,
    CustomerTransferApprovalService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 创建转移审批申请
/// POST /api/v1/erp/crm/transfer-approvals
pub async fn create_approval(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateTransferApprovalRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTransferApprovalService::new(state.db.clone());
    let result = service
        .create_approval(req, auth.user_id, &auth.username)
        .await?;

    tracing::info!(
        "用户 {} 创建客户转移审批单 {}",
        auth.username,
        result.approval_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "转移审批申请创建成功",
    )))
}

/// 查询审批列表
/// GET /api/v1/erp/crm/transfer-approvals
pub async fn list_approvals(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ApprovalQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTransferApprovalService::new(state.db.clone());
    let (items, total) = service.list_approvals(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
    }))))
}

/// 获取审批详情
/// GET /api/v1/erp/crm/transfer-approvals/:id
pub async fn get_approval(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTransferApprovalService::new(state.db.clone());
    let result = service.get_approval(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// 申请人取消审批
/// POST /api/v1/erp/crm/transfer-approvals/:id/cancel
pub async fn cancel_approval(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerTransferApprovalService::new(state.db.clone());
    let result = service.cancel_approval(id, auth.user_id).await?;

    tracing::info!(
        "用户 {} 取消客户转移审批单 {}",
        auth.username,
        result.approval_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "审批单已取消",
    )))
}

/// 销售经理审批
/// POST /api/v1/erp/crm/transfer-approvals/:id/manager-approve
pub async fn manager_approve(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(mut req): Json<ApproveRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 路径参数覆盖请求体中的 approval_id
    req.approval_id = id;

    let service = CustomerTransferApprovalService::new(state.db.clone());
    let result = service
        .manager_approve(req, auth.user_id, &auth.username)
        .await?;

    tracing::info!(
        "销售经理 {} 审批转移单 {}",
        auth.username,
        result.approval_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "经理审批完成",
    )))
}

/// 总监审批（仅大客户转移）
/// POST /api/v1/erp/crm/transfer-approvals/:id/director-approve
pub async fn director_approve(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(mut req): Json<ApproveRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 路径参数覆盖请求体中的 approval_id
    req.approval_id = id;

    let service = CustomerTransferApprovalService::new(state.db.clone());
    let result = service
        .director_approve(req, auth.user_id, &auth.username)
        .await?;

    tracing::info!(
        "总监 {} 审批转移单 {}",
        auth.username,
        result.approval_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "总监审批完成",
    )))
}
