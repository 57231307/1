//! 采购退货 Handler
//!
//! 采购退货 HTTP 接口层

use crate::services::purchase_return_service::{
    CreatePurchaseReturnRequest, PurchaseReturnService, UpdatePurchaseReturnRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::utils::app_state::AppState;
use serde::Deserialize;
use validator::Validate;

/// 查询采购退货单列表
pub async fn list_returns(
    Query(params): Query<ReturnQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let (returns, total) = service
        .list_returns(
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
            params.status,
            params.supplier_id,
        )
        .await?;

    let result = serde_json::json!({
        "items": returns,
        "total": total,
        "page": params.page.unwrap_or(1),
        "page_size": params.page_size.unwrap_or(20),
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取采购退货单详情
pub async fn get_return(
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
pub async fn create_return(
    State(state): State<AppState>,
    Json(req): Json<CreatePurchaseReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = PurchaseReturnService::new(state.db.clone());
    let user_id = 1;

    let return_order = service.create_return(req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单创建成功",
    )))
}

/// 更新采购退货单
#[axum::debug_handler]
pub async fn update_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<UpdatePurchaseReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let user_id = 1;

    let return_order = service.update_return(id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单更新成功",
    )))
}

/// 提交采购退货单
pub async fn submit_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let user_id = 1;

    let return_order = service.submit_return(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已提交",
    )))
}

/// 审批采购退货单
pub async fn approve_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let user_id = 1;

    let return_order = service.approve_return(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已审批",
    )))
}

/// 拒绝采购退货单
#[axum::debug_handler]
pub async fn reject_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<RejectReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let user_id = 1;

    let return_order = service.reject_return(id, req.reason, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已拒绝",
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
