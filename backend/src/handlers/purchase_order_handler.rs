//! 采购订单 Handler
//!
//! 采购订单 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层
#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use crate::middleware::auth_context::AuthContext;
use crate::models::supplier;
use crate::services::po::order::PurchaseOrderService;
use crate::services::po::{
    CreateOrderItemRequest, CreatePurchaseOrderRequest, UpdateOrderItemRequest,
    UpdatePurchaseOrderRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 查询采购订单列表
pub async fn list_orders(
    Query(params): Query<OrderQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
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
    let mut orders_json: Vec<serde_json::Value> = orders
        .into_iter()
        .map(|o| serde_json::to_value(o).unwrap_or_default())
        .collect();

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "purchase_order")
            .await
        {
            state.data_permission_service.filter_fields_batch(
                &mut orders_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            for order in &mut orders_json {
                if let Some(obj) = order.as_object_mut() {
                    obj.remove("total_amount");
                    obj.remove("total_amount_foreign");
                    obj.remove("total_quantity");
                    obj.remove("total_quantity_alt");
                    obj.remove("paid_amount");
                    obj.remove("balance_amount");

                    if let Some(items) = obj.get_mut("items").and_then(|i| i.as_array_mut()) {
                        for item in items {
                            if let Some(item_obj) = item.as_object_mut() {
                                item_obj.remove("unit_price");
                                item_obj.remove("unit_price_foreign");
                                item_obj.remove("subtotal");
                                item_obj.remove("tax_amount");
                                item_obj.remove("discount_amount");
                                item_obj.remove("total_amount");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(orders_json)))
}

/// 获取采购订单详情
pub async fn get_order(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let order = service.get_order(id).await?;
    let mut order_json = serde_json::to_value(order)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "purchase_order")
            .await
        {
            state.data_permission_service.filter_fields(
                &mut order_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            if let Some(obj) = order_json.as_object_mut() {
                obj.remove("total_amount");
                obj.remove("total_amount_foreign");
                obj.remove("total_quantity");
                obj.remove("total_quantity_alt");
                obj.remove("paid_amount");
                obj.remove("balance_amount");

                if let Some(items) = obj.get_mut("items").and_then(|i| i.as_array_mut()) {
                    for item in items {
                        if let Some(item_obj) = item.as_object_mut() {
                            item_obj.remove("unit_price");
                            item_obj.remove("unit_price_foreign");
                            item_obj.remove("subtotal");
                            item_obj.remove("tax_amount");
                            item_obj.remove("discount_amount");
                            item_obj.remove("total_amount");
                        }
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(order_json)))
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

    // 发送采购订单创建通知
    if let Some(ref event_service) = state.event_notification_service {
        let supplier_name = supplier::Entity::find_by_id(order.supplier_id)
            .one(state.db.as_ref())
            .await
            .ok()
            .flatten()
            .map(|s| s.supplier_name)
            .unwrap_or_else(|| "未知供应商".to_string());

        let amount = order.total_amount.to_string();

        let _ = event_service
            .notify_purchase_order_created(
                user_id,
                &order.order_no,
                order.id,
                &supplier_name,
                &amount,
            )
            .await;
    }

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

    Ok(Json(ApiResponse::success_with_message(
        (),
        "采购订单删除成功",
    )))
}

/// 提交采购订单
pub async fn submit_order(
    auth: AuthContext,
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
pub async fn reject_order(
    auth: AuthContext,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<RejectOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let order = service
        .reject_order(id, req.reason.clone(), user_id)
        .await?;

    // 发送审批拒绝通知
    if let Some(ref event_service) = state.event_notification_service {
        let _ = event_service
            .notify_approval_result(
                order.created_by,
                &order.order_no,
                false,
                &auth.username,
                Some(&req.reason),
            )
            .await;
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单已拒绝",
    )))
}

/// 关闭采购订单
pub async fn close_order(
    auth: AuthContext,
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
pub async fn list_order_items(
    _auth: AuthContext,
    Path(order_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let items = service.list_order_items(order_id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(items)?)))
}

/// 添加订单明细
#[axum::debug_handler]
pub async fn create_order_item(
    auth: AuthContext,
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
pub async fn update_order_item(
    auth: AuthContext,
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
pub async fn delete_order_item(
    auth: AuthContext,
    Path((_order_id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    service.delete_order_item(item_id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "订单明细删除成功",
    )))
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
    _auth: AuthContext,
    Json(req): Json<CalculateDeliveryRequest>,
) -> Result<Json<ApiResponse<DeliveryDateResponse>>, AppError> {
    let calculator = crate::services::purchase_delivery_calculator::PurchaseDeliveryCalculator::new(
        state.db.clone(),
    );

    let items: Vec<crate::services::purchase_delivery_calculator::OrderItemInfo> = req
        .items
        .into_iter()
        .map(
            |item| crate::services::purchase_delivery_calculator::OrderItemInfo {
                product_id: item.product_id,
                quantity: item.quantity,
            },
        )
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
#[derive(Debug, Deserialize, Validate)]
pub struct RejectOrderRequest {
    #[validate(length(min = 1, max = 500, message = "拒绝原因不能为空且最长500字符"))]
    pub reason: String,
}

// ========== 数据导出接口 ==========

use axum::http::header;

/// 导出采购订单
pub async fn export_orders(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<OrderQueryParams>,
) -> Result<axum::response::Response, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());

    let csv_data = service
        .export_orders_to_csv(query.status, query.supplier_id)
        .await?;

    let filename = format!(
        "purchase_orders_export_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    let response = axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(axum::body::Body::from(csv_data))
        .map_err(|e| AppError::internal(format!("响应构建失败: {}", e)))?;

    Ok(response)
}
