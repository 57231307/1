//! 采购退货 Handler
//!
//! 采购退货 HTTP 接口层

use crate::middleware::auth_context::AuthContext;
use crate::services::purchase_return_service::{
    CreatePurchaseReturnRequest, CreateReturnItemRequest, PurchaseReturnService,
    UpdatePurchaseReturnRequest, UpdateReturnItemRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use validator::Validate;

/// 查询采购退货单列表
pub async fn list_purchase_returns(
    Query(params): Query<ReturnQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let (returns, total) = service
        .list_returns(
            params.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
            params.page_size.unwrap_or(20).clamp(1, 100),
            params.status,
            params.supplier_id,
        )
        .await?;

    let result = serde_json::to_value(PaginatedResponse::new(
        returns,
        total,
        params.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        params.page_size.unwrap_or(20).clamp(1, 100),
    ))
    .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取采购退货单详情
pub async fn get_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let return_order = service.get_return(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(
        return_order,
    )?)))
}

/// 创建采购退货单
#[axum::debug_handler]
pub async fn create_purchase_return(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePurchaseReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()?;

    let service = PurchaseReturnService::new(state.db.clone());

    let return_order = service.create_return(req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单创建成功",
    )))
}

/// 更新采购退货单
#[axum::debug_handler]
pub async fn update_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdatePurchaseReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());

    let return_order = service.update_return(id, req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单更新成功",
    )))
}

/// 提交采购退货单
pub async fn submit_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());

    let return_order = service.submit_return(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已提交",
    )))
}

/// 审批采购退货单
pub async fn approve_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());

    let return_order = service.approve_return(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已审批",
    )))
}

/// 拒绝采购退货单
#[axum::debug_handler]
pub async fn reject_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RejectReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());

    let return_order = service
        .reject_return(id, req.reason.clone(), auth.user_id)
        .await?;

    // 发送审批拒绝通知
    if let Some(ref event_service) = state.event_notification_service {
        if let Some(created_by) = return_order.created_by {
            // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
            if let Err(e) = event_service
                .notify_approval_result(
                    created_by,
                    &return_order.return_no,
                    false,
                    &auth.username,
                    Some(&req.reason),
                )
                .await
            {
                tracing::warn!(error = %e, return_id = id, "采购退货审批拒绝通知发送失败");
            }
        }
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已拒绝",
    )))
}

/// 删除采购退货单
pub async fn delete_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // 批次 101 v6 复审 P2-4：透传操作人 user_id 用于审计日志
    service.delete(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "采购退货单已删除",
    )))
}

// =====================================================
// 请求 DTO
// =====================================================

/// 采购退货单查询参数
#[derive(Debug, Deserialize)]
pub struct ReturnQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

/// 拒绝退货单请求
#[derive(Debug, Deserialize)]
pub struct RejectReturnRequest {
    pub reason: String,
}

pub async fn list_purchase_return_items(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::purchase_return_service::PurchaseReturnItemDto>>>,
    AppError,
> {
    let service = PurchaseReturnService::new(state.db.clone());
    let items = service.list_items(id).await?;
    Ok(Json(ApiResponse::success(items)))
}

pub async fn create_purchase_return_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<CreateReturnItemRequest>,
) -> Result<Json<ApiResponse<crate::models::purchase_return_item::Model>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // 批次 101 v6 复审 P2-5：透传操作人 user_id 给 update_return_totals 用于审计日志
    let item = service.create_item(id, req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(item)))
}

pub async fn update_purchase_return_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((_id, item_id)): Path<(i32, i32)>,
    Json(req): Json<UpdateReturnItemRequest>,
) -> Result<Json<ApiResponse<crate::models::purchase_return_item::Model>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // 批次 101 v6 复审 P2-3：透传操作人 user_id 用于审计日志
    let item = service.update_item(item_id, req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(item)))
}

pub async fn delete_purchase_return_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((_id, item_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // 批次 101 v6 复审 P2-5：透传操作人 user_id 给 update_return_totals 用于审计日志
    service.delete_item(item_id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(())))
}
