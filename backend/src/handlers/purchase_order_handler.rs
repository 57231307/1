//! 采购订单 Handler
//!
//! 采购订单 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::services::purchase_order_service::{
    CreateOrderItemRequest, CreatePurchaseOrderRequest, PurchaseOrderService,
    UpdateOrderItemRequest, UpdatePurchaseOrderRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

/// 查询采购订单列表
pub async fn list_orders(
    Query(params): Query<OrderQueryParams>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let (orders, total) = service
        .list_orders(
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
            params.status,
            params.supplier_id,
        )
        .await?;

    let result = serde_json::json!({
        "items": orders,
        "total": total,
        "page": params.page.unwrap_or(1),
        "page_size": params.page_size.unwrap_or(20),
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取采购订单详情
pub async fn get_order(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let order = service.get_order(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(order)?)))
}

/// 创建采购订单
#[axum::debug_handler]
pub async fn create_order(
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CreatePurchaseOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证请求
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = PurchaseOrderService::new(db);
    let user_id = 1; // TODO: 从认证中获取

    let order = service.create_order(req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单创建成功",
    )))
}

/// 更新采购订单
#[axum::debug_handler]
pub async fn update_order(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<UpdatePurchaseOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let user_id = 1; // TODO: 从认证中获取

    let order = service.update_order(id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单更新成功",
    )))
}

/// 删除采购订单
pub async fn delete_order(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<StatusCode, AppError> {
    let service = PurchaseOrderService::new(db);
    service.delete_order(id, 1).await?; // TODO: 从认证中获取 user_id

    Ok(StatusCode::NO_CONTENT)
}

/// 提交采购订单
pub async fn submit_order(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let user_id = 1; // TODO: 从认证中获取

    let order = service.submit_order(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单提交成功",
    )))
}

/// 审批采购订单
pub async fn approve_order(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let user_id = 1; // TODO: 从认证中获取

    let order = service.approve_order(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单审批成功",
    )))
}

/// 拒绝采购订单
#[axum::debug_handler]
pub async fn reject_order(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<RejectOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let user_id = 1; // TODO: 从认证中获取

    let order = service.reject_order(id, req.reason, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单已拒绝",
    )))
}

/// 关闭采购订单
pub async fn close_order(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let user_id = 1; // TODO: 从认证中获取

    let order = service.close_order(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单已关闭",
    )))
}

/// 获取订单明细列表
pub async fn list_order_items(
    Path(order_id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let items = service.list_order_items(order_id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(items)?)))
}

/// 添加订单明细
#[axum::debug_handler]
pub async fn create_order_item(
    Path(order_id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CreateOrderItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证请求
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = PurchaseOrderService::new(db);
    let user_id = 1; // TODO: 从认证中获取

    let item = service.add_order_item(order_id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "订单明细添加成功",
    )))
}

/// 更新订单明细
#[axum::debug_handler]
pub async fn update_order_item(
    Path((_order_id, item_id)): Path<(i32, i32)>,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<UpdateOrderItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(db);
    let user_id = 1; // TODO: 从认证中获取

    let item = service.update_order_item(item_id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "订单明细更新成功",
    )))
}

/// 删除订单明细
pub async fn delete_order_item(
    Path((_order_id, item_id)): Path<(i32, i32)>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<StatusCode, AppError> {
    let service = PurchaseOrderService::new(db);
    service.delete_order_item(item_id, 1).await?; // TODO: 从认证中获取 user_id

    Ok(StatusCode::NO_CONTENT)
}

// =====================================================
// 请求 DTO
// =====================================================

/// 采购订单查询参数
#[derive(Debug, Deserialize)]
pub struct OrderQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

/// 拒绝订单请求
#[derive(Debug, Deserialize)]
pub struct RejectOrderRequest {
    pub reason: String,
}
