//! 采购入库 Handler
//!
//! 采购入库 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::services::purchase_receipt_service::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, PurchaseReceiptService,
    UpdatePurchaseReceiptRequest, UpdateReceiptItemRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use crate::utils::app_state::AppState;
use serde::Deserialize;
use validator::Validate;

/// 查询采购入库单列表
pub async fn list_receipts(
    Query(params): Query<ReceiptQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let (receipts, total) = service
        .list_receipts(
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
            params.status,
            params.supplier_id,
            params.order_id,
        )
        .await?;

    let result = serde_json::json!({
        "items": receipts,
        "total": total,
        "page": params.page.unwrap_or(1),
        "page_size": params.page_size.unwrap_or(20),
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取采购入库单详情
pub async fn get_receipt(auth: AuthContext, 
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let receipt = service.get_receipt(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(receipt)?)))
}

/// 创建采购入库单
#[axum::debug_handler]
pub async fn create_receipt(auth: AuthContext, 
    State(state): State<AppState>,
    Json(req): Json<CreatePurchaseReceiptRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证请求
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let receipt = service.create_receipt(req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(receipt)?,
        "采购入库单创建成功",
    )))
}

/// 更新采购入库单
#[axum::debug_handler]
pub async fn update_receipt(auth: AuthContext, 
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<UpdatePurchaseReceiptRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let receipt = service.update_receipt(id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(receipt)?,
        "采购入库单更新成功",
    )))
}

/// 确认采购入库单
pub async fn confirm_receipt(auth: AuthContext, 
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let receipt = service.confirm_receipt(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(receipt)?,
        "采购入库单已确认",
    )))
}

/// 获取入库明细列表
pub async fn list_receipt_items(auth: AuthContext, 
    Path(receipt_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let items = service.list_receipt_items(receipt_id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(items)?)))
}

/// 添加入库明细
#[axum::debug_handler]
pub async fn create_receipt_item(auth: AuthContext, 
    Path(receipt_id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<CreateReceiptItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证请求
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let item = service.add_receipt_item(receipt_id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "入库明细添加成功",
    )))
}

/// 更新入库明细
#[axum::debug_handler]
pub async fn update_receipt_item(auth: AuthContext, 
    Path((_receipt_id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    Json(req): Json<UpdateReceiptItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let item = service.update_receipt_item(item_id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "入库明细更新成功",
    )))
}

/// 删除入库明细
pub async fn delete_receipt_item(auth: AuthContext, 
    Path((_receipt_id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    service.delete_receipt_item(item_id, auth.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

// =====================================================
// 请求 DTO
// =====================================================

/// 采购入库单查询参数
#[derive(Debug, Deserialize)]
pub struct ReceiptQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    pub order_id: Option<i32>,
}
