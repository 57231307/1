//! 采购订单 Handler
//!
//! 采购订单 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::models::purchase_order;
use crate::models::supplier;
use crate::services::po::order::PurchaseOrderService;
use crate::services::po::{
    CreateOrderItemRequest, CreatePurchaseOrderRequest, UpdateOrderItemRequest,
    UpdatePurchaseOrderRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
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
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let (orders, _total) = service
        .list_orders(
            params.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
            params.page_size.unwrap_or(20).clamp(1, 100),
            params.status,
            params.supplier_id,
            Some(&data_scope_ctx),
        )
        .await?;

    // 转换为JSON值数组
    // 批次 406 修复：序列化失败应传播错误而非返回 Null
    let mut orders_json: Vec<serde_json::Value> = orders
        .into_iter()
        .map(|o| serde_json::to_value(o).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

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
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let order = service.get_order(id, Some(&data_scope_ctx)).await?;
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

        // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
        if let Err(e) = event_service
            .notify_purchase_order_created(
                user_id,
                &order.order_no,
                order.id,
                &supplier_name,
                &amount,
            )
            .await
        {
            tracing::warn!(error = %e, order_id = order.id, "采购订单创建通知发送失败");
        }
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
        // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
        if let Err(e) = event_service
            .notify_approval_result(
                order.created_by,
                &order.order_no,
                false,
                auth.user_id,
                &auth.username,
                Some(&req.reason),
            )
            .await
        {
            tracing::warn!(error = %e, order_id = id, "采购订单审批拒绝通知发送失败");
        }
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

/// 取消采购订单
/// 批次 215 P2-1 修复（v12 复审）：实现采购订单取消功能
pub async fn cancel_order(
    auth: AuthContext,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<CancelOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());
    let user_id = auth.user_id;

    let order = service
        .cancel_order(id, req.reason.clone(), user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(order)?,
        "采购订单已取消",
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

/// 取消采购订单请求（批次 215 P2-1）
#[derive(Debug, Deserialize, Validate)]
pub struct CancelOrderRequest {
    #[validate(length(min = 1, max = 500, message = "取消原因不能为空且最长500字符"))]
    pub reason: String,
}

// ========== 数据导出接口 ==========

use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use std::sync::Arc;

/// 导出采购订单
pub async fn export_orders(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OrderQueryParams>,
) -> Result<axum::response::Response, AppError> {
    let service = PurchaseOrderService::new(state.db.clone());

    // V15 P0-S11：提前 clone 查询条件用于审计日志（避免 service 调用 move 后 borrow of moved value）
    let audit_status = query.status.clone();

    let csv_data = service
        .export_orders_to_csv(query.status, query.supplier_id)
        .await?;

    // 规则 3：将 service 返回的 CSV 解析为 xlsx 表格
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_data.as_slice());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| AppError::internal(format!("CSV解析错误: {}", e)))?
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| AppError::internal(format!("CSV解析错误: {}", e)))?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    let row_count = rows.len();
    let table = XlsxTable {
        sheet_name: "采购订单".to_string(),
        headers,
        rows,
    };

    let filename = format!(
        "purchase_orders_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("purchase_order".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出采购订单（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/purchases/orders/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "status_filter": audit_status,
            "supplier_id_filter": query.supplier_id,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    build_xlsx_response(&table, &filename)
}

/// 生成采购订单号
/// GET /api/v1/erp/purchases/orders/generate-no
///
/// 返回格式: `{ prefix: "PO", order_no: "PO20260617001" }`
pub async fn generate_order_no(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let order_no = DocumentNumberGenerator::generate_no(
        &*state.db,
        "PO",
        purchase_order::Entity,
        purchase_order::Column::OrderNo,
    )
    .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "prefix": "PO",
        "order_no": order_no
    }))))
}
