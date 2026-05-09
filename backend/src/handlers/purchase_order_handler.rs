//! 采购订单 Handler
//!
//! 采购订单 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::services::purchase_order_service::{
    CreateOrderItemRequest, CreatePurchaseOrderRequest, PurchaseOrderService,
    UpdateOrderItemRequest, UpdatePurchaseOrderRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 查询采购订单列表
pub async fn list_orders(
    Query(params): Query<OrderQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let (orders, _total) = service
        .list_orders(
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
            params.status,
            params.supplier_id,
        )
        .await?;

    // 转换为JSON值数组
    let orders_json: Vec<serde_json::Value> = orders
        .into_iter()
        .map(|o| serde_json::to_value(o).unwrap_or_default())
        .collect();

    Ok(Json(ApiResponse::success(orders_json)))
}

/// 获取采购订单详情
pub async fn get_order(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let order = service.get_order(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(order)?)))
}

/// 创建采购订单
#[axum::debug_handler]
pub async fn create_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePurchaseOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证请求
    req.validate()?;

    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

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
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdatePurchaseOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let order = service.update_order(id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单更新成功",
    )))
}

/// 删除采购订单
pub async fn delete_order(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    service.delete_order(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_msg((), "采购订单删除成功")))
}

/// 提交采购订单
pub async fn submit_order(auth: AuthContext, 
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let order = service.submit_order(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单提交成功",
    )))
}

/// 审批采购订单
pub async fn approve_order(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let order = service.approve_order(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单审批成功",
    )))
}

/// 拒绝采购订单
#[axum::debug_handler]
pub async fn reject_order(auth: AuthContext, 
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<RejectOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let order = service.reject_order(id, req.reason, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单已拒绝",
    )))
}

/// 关闭采购订单
pub async fn close_order(auth: AuthContext, 
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let order = service.close_order(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单已关闭",
    )))
}

/// 获取订单明细列表
pub async fn list_order_items(_auth: AuthContext, 
    Path(order_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let items = service.list_order_items(order_id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(items)?)))
}

/// 添加订单明细
#[axum::debug_handler]
pub async fn create_order_item(auth: AuthContext, 
    Path(order_id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<CreateOrderItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证请求
    req.validate()?;

    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let item = service.add_order_item(order_id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "订单明细添加成功",
    )))
}

/// 更新订单明细
#[axum::debug_handler]
pub async fn update_order_item(auth: AuthContext, 
    Path((_order_id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    Json(req): Json<UpdateOrderItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let item = service.update_order_item(item_id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "订单明细更新成功",
    )))
}

/// 删除订单明细
pub async fn delete_order_item(auth: AuthContext, 
    Path((_order_id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    service.delete_order_item(item_id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_msg((), "订单明细删除成功")))
}

/// 计算建议交货日期
/// POST /api/v1/erp/purchase/orders/delivery-date
#[derive(Debug, Deserialize)]
pub struct CalculateDeliveryRequest {
    pub supplier_id: i32,
    pub order_date: chrono::NaiveDate,
    pub items: Vec<DeliveryItemRequest>,
}

#[derive(Debug, Deserialize)]
pub struct DeliveryItemRequest {
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
}

#[derive(Debug, Serialize)]
pub struct DeliveryDateResponse {
    pub suggested_date: chrono::NaiveDate,
    pub avg_lead_time_days: i32,
    pub max_production_days: i32,
    pub calculation_basis: String,
    pub historical_orders: i64,
}

pub async fn calculate_delivery_date(
    State(state): State<AppState>,
    Json(req): Json<CalculateDeliveryRequest>,
) -> Result<Json<ApiResponse<DeliveryDateResponse>>, AppError> {
    let calculator = crate::services::purchase_delivery_calculator::PurchaseDeliveryCalculator::new(state.db.clone());

    let items: Vec<crate::services::purchase_delivery_calculator::OrderItemInfo> = req.items
        .into_iter()
        .map(|item| crate::services::purchase_delivery_calculator::OrderItemInfo {
            product_id: item.product_id,
            quantity: item.quantity,
        })
        .collect();

    let calc_req = crate::services::purchase_delivery_calculator::DeliveryCalculationRequest {
        supplier_id: req.supplier_id,
        order_date: req.order_date,
        items,
    };

    let result = calculator.calculate_delivery_date(&calc_req).await?;

    Ok(Json(ApiResponse::success(DeliveryDateResponse {
        suggested_date: result.suggested_date,
        avg_lead_time_days: result.avg_lead_time_days,
        max_production_days: result.max_production_days,
        calculation_basis: result.calculation_basis,
        historical_orders: result.historical_orders,
    })))
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
