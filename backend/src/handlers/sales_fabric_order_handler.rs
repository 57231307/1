//! 面料行业版销售订单 handler
//!
//! 缺陷 3 修复：原实现直接操作 Entity 构建事务/订单号/金额计算（绕过 Service 层），
//! 现已下沉至 `services/so/fabric_order.rs`（impl SalesService），
//! 本文件仅保留请求 DTO（兼容外部引用）+ 参数提取 + service 调用。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::sales_order;
use crate::services::so::fabric_order::{CreateFabricOrderRequest, UpdateFabricOrderRequest};
use crate::services::so::order::SalesService;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 销售订单列表（反序列化输入字段）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct FabricOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub customer_id: Option<i32>,
    pub order_no: Option<String>,
    pub status: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
}

// FabricOrderItemRequest / CreateFabricOrderRequest / UpdateFabricOrderRequest
// 已迁移至 services/so/fabric_order.rs

/// 获取销售订单列表（面料行业版）
pub async fn list_fabric_orders(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<FabricOrderQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let _data_scope = auth.to_data_scope_context();
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    let (orders, total) = service
        .list_fabric_orders(
            query.page.unwrap_or(1),
            query.page_size.unwrap_or(20),
            query.customer_id,
            query.order_no,
            query.status,
            query.batch_no,
            query.color_no,
        )
        .await?;

    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let orders_json: Vec<serde_json::Value> = orders
        .into_iter()
        .map(|o: sales_order::Model| {
            serde_json::to_value(o).map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        orders_json,
        total,
        page,
        page_size,
    ))))
}

/// 获取销售订单详情
pub async fn get_fabric_order(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    let order = service.get_fabric_order(id).await?;
    let order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(order_json)))
}

/// 创建销售订单（面料行业版）
pub async fn create_fabric_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateFabricOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    let created_order = service.create_fabric_order(req, auth.user_id).await?;
    let order_json = serde_json::to_value(created_order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "订单创建成功",
    )))
}

/// 更新销售订单
pub async fn update_fabric_order(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateFabricOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    let updated = service.update_fabric_order(id, req).await?;
    let order_json = serde_json::to_value(updated)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "订单更新成功",
    )))
}

/// 删除销售订单
pub async fn delete_fabric_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    service.delete_fabric_order(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message((), "订单删除成功")))
}

/// 审核订单
pub async fn approve_fabric_order(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    let updated = service.approve_fabric_order(id).await?;
    let order_json = serde_json::to_value(updated)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "订单审核成功",
    )))
}
