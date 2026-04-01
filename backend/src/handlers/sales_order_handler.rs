use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;

use crate::models::dto::{ApiResponse, PageRequest};
use crate::services::sales_service::{
    CreateSalesOrderRequest, SalesService, UpdateSalesOrderRequest,
};
use crate::utils::error::AppError;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct SalesOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub order_no: Option<String>,
}

/// 获取销售订单列表
/// GET /api/v1/erp/sales/orders
pub async fn list_orders(
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<SalesOrderQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let sales_service = SalesService::new(db.clone());

    let page_req = PageRequest {
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(10),
    };

    let orders = sales_service
        .list_orders(page_req, query.status, query.customer_id, query.order_no)
        .await?;

    let orders_json: Vec<serde_json::Value> = orders
        .data
        .into_iter()
        .map(|o| serde_json::to_value(o).unwrap_or_default())
        .collect();

    Ok(Json(ApiResponse::success(orders_json)))
}

/// 获取销售订单详情
/// GET /api/v1/erp/sales/orders/:id
pub async fn get_order(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(db.clone());
    let order = sales_service.get_order_detail(id).await?;
    let order_json = serde_json::to_value(order).unwrap_or_default();
    Ok(Json(ApiResponse::success(order_json)))
}

/// 创建销售订单
/// POST /api/v1/erp/sales/orders
pub async fn create_order(
    State(db): State<Arc<DatabaseConnection>>,
    Json(request): Json<CreateSalesOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(db.clone());
    let order = sales_service.create_order(request).await?;
    let order_json = serde_json::to_value(order).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        order_json,
        "销售订单创建成功",
    )))
}

/// 更新销售订单
/// PUT /api/v1/erp/sales/orders/:id
pub async fn update_order(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateSalesOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(db.clone());
    let order = sales_service.update_order(id, request).await?;
    let order_json = serde_json::to_value(order).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        order_json,
        "销售订单更新成功",
    )))
}

/// 删除销售订单
/// DELETE /api/v1/erp/sales/orders/:id
pub async fn delete_order(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let sales_service = SalesService::new(db.clone());
    sales_service.delete_order(id).await?;
    Ok(Json(ApiResponse::success_with_msg((), "销售订单删除成功")))
}

/// 审核销售订单
/// POST /api/v1/erp/sales/orders/:id/approve
pub async fn approve_order(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(db.clone());
    let order = sales_service.approve_order(id).await?;
    let order_json = serde_json::to_value(order).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        order_json,
        "销售订单审核成功",
    )))
}

/// 发货处理
/// POST /api/v1/erp/sales/orders/:id/ship
pub async fn ship_order(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(db.clone());
    let order = sales_service.ship_order(id).await?;
    let order_json = serde_json::to_value(order).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        order_json,
        "销售订单发货成功",
    )))
}

/// 完成订单
/// POST /api/v1/erp/sales/orders/:id/complete
pub async fn complete_order(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(db.clone());
    let order = sales_service.complete_order(id).await?;
    let order_json = serde_json::to_value(order).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        order_json,
        "销售订单完成成功",
    )))
}
