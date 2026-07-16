use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use validator::Validate;

use crate::models::dto::PageRequest;
use crate::models::sales_order;
use crate::services::so::order::SalesService;
use crate::services::so::{CreateSalesOrderRequest, UpdateSalesOrderRequest};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::response::ApiResponse;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct SalesOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub order_no: Option<String>,
}

/// P1-2d 修复（批次 81 v1 复审）：创建发货请求 DTO
/// 替代 create_delivery 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDeliveryDto {
    /// 仓库 ID：可选，缺失时默认 0（保持原向后兼容逻辑）
    pub warehouse_id: Option<i32>,
}

/// 获取销售订单列表
/// GET /api/v1/erp/sales/orders
pub async fn list_orders(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<SalesOrderQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    let page_req = PageRequest {
        page: query.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        page_size: query.page_size.unwrap_or(10).clamp(1, 100),
    };

    let orders = sales_service
        .list_orders(page_req, query.status, query.customer_id, query.order_no)
        .await?;

    let mut orders_json = serde_json::to_value(orders)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "sales_order")
            .await
        {
            let mut list_opt = orders_json.get_mut("list");
            if list_opt.is_none() {
                list_opt = orders_json.get_mut("data");
            }
            if let Some(list) = list_opt.and_then(|v| v.as_array_mut()) {
                state.data_permission_service.filter_fields_batch(
                    list,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            }
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            let mut list_opt = orders_json.get_mut("list");
            if list_opt.is_none() {
                list_opt = orders_json.get_mut("data");
            }
            if let Some(list) = list_opt.and_then(|v| v.as_array_mut()) {
                for order in list {
                    if let Some(obj) = order.as_object_mut() {
                        obj.remove("subtotal");
                        obj.remove("tax_amount");
                        obj.remove("discount_amount");
                        obj.remove("shipping_cost");
                        obj.remove("total_amount");
                        obj.remove("paid_amount");
                        obj.remove("balance_amount");

                        if let Some(items) = obj.get_mut("items").and_then(|i| i.as_array_mut()) {
                            for item in items {
                                if let Some(item_obj) = item.as_object_mut() {
                                    item_obj.remove("unit_price");
                                    item_obj.remove("tax_rate");
                                    item_obj.remove("total_price");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(orders_json)))
}

/// 获取销售订单详情
/// GET /api/v1/erp/sales/orders/:id
pub async fn get_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    let order = sales_service.get_order_detail(id).await?;
    let mut order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "sales_order")
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
                obj.remove("subtotal");
                obj.remove("tax_amount");
                obj.remove("discount_amount");
                obj.remove("shipping_cost");
                obj.remove("total_amount");
                obj.remove("paid_amount");
                obj.remove("balance_amount");

                if let Some(items) = obj.get_mut("items").and_then(|i| i.as_array_mut()) {
                    for item in items {
                        if let Some(item_obj) = item.as_object_mut() {
                            item_obj.remove("unit_price");
                            item_obj.remove("tax_rate");
                            item_obj.remove("total_price");
                        }
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(order_json)))
}

/// 创建销售订单
/// POST /api/v1/erp/sales/orders
pub async fn create_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(request): Json<CreateSalesOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 输入验证
    use validator::Validate;
    if let Err(e) = request.validate() {
        return Err(AppError::validation(e.to_string()));
    }

    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    let order = sales_service.create_order(request, auth.user_id).await?;

    // 订单创建成功后发送通知
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
            if let Err(e) = event_service
                .notify_order_submitted(created_by, &order.order_no, order.id)
                .await
            {
                tracing::warn!("批次 94 P2-11：订单创建通知发送失败: {}", e);
            }
        }
    }

    let order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "销售订单创建成功",
    )))
}

/// 更新销售订单
/// PUT /api/v1/erp/sales/orders/:id
pub async fn update_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(request): Json<UpdateSalesOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    // 批次 94 P2-10：传入真实操作人 user_id 用于审计日志
    let order = sales_service
        .update_order(id, auth.user_id, request)
        .await?;
    let order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "销售订单更新成功",
    )))
}

/// 删除销售订单
/// DELETE /api/v1/erp/sales/orders/:id
pub async fn delete_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    // 批次 94 P2-10：传入真实操作人 user_id 用于审计日志
    sales_service.delete_order(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "销售订单删除成功",
    )))
}

/// 提交销售订单审批
/// POST /api/v1/erp/sales/orders/:id/submit
pub async fn submit_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    let user_id = auth.user_id;
    let order = sales_service.submit_order(id, user_id).await?;

    // 订单提交成功后发送通知给申请人
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
            if let Err(e) = event_service
                .notify_order_submitted(created_by, &order.order_no, order.id)
                .await
            {
                tracing::warn!("批次 94 P2-11：订单提交通知发送失败: {}", e);
            }
        }
    }

    let order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "销售订单已提交审批",
    )))
}

/// 审核销售订单
/// POST /api/v1/erp/sales/orders/:id/approve
pub async fn approve_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    let order = sales_service.approve_order(id, auth.user_id).await?;

    // 订单审批成功后发送通知给申请人
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
            if let Err(e) = event_service
                .notify_order_approved(created_by, &order.order_no, order.id, auth.user_id, &auth.username)
                .await
            {
                tracing::warn!("批次 94 P2-11：订单审批通知发送失败: {}", e);
            }
        }
    }

    let order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "销售订单审核成功",
    )))
}

/// 发货处理
/// POST /api/v1/erp/sales/orders/:id/ship
pub async fn ship_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<crate::services::so::delivery::ShipOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    // 调用原有 ship_order(request, user_id)
    sales_service.ship_order(payload, auth.user_id).await?;
    // 重新获取订单详情用于通知
    let order = sales_service.get_order_detail(id).await?;

    // 订单发货成功后发送通知给申请人
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
            if let Err(e) = event_service
                .notify_order_shipped(created_by, &order.order_no, order.id)
                .await
            {
                tracing::warn!("批次 94 P2-11：订单发货通知发送失败: {}", e);
            }
        }
    }

    let order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "销售订单发货成功",
    )))
}

/// 完成订单
/// POST /api/v1/erp/sales/orders/:id/complete
pub async fn complete_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID 用于审计日志
    let order = sales_service.complete_order(id, auth.user_id).await?;

    // 订单完成后发送通知给申请人
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
            if let Err(e) = event_service
                .notify_order_completed(created_by, &order.order_no, order.id)
                .await
            {
                tracing::warn!("批次 94 P2-11：订单完成通知发送失败: {}", e);
            }
        }
    }

    let order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "销售订单完成成功",
    )))
}

/// 查询订单变更历史
/// GET /api/v1/erp/sales/orders/:id/history
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub async fn get_order_history(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let history_service =
        crate::services::order_change_history_service::OrderChangeHistoryService::new(
            state.db.clone(),
        );
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let (histories, total) = history_service
        .get_history_by_order(id, page, page_size)
        .await?;

    let result = serde_json::json!({
        "list": histories,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

// ========== 数据导出接口 ==========

use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use std::sync::Arc;

/// 导出销售订单
pub async fn export_orders(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<SalesOrderQuery>,
) -> Result<axum::response::Response, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    let csv_data = sales_service
        .export_orders_to_csv(query.status, query.customer_id, query.order_no)
        .await
        .map_err(|e| AppError::internal(format!("导出失败: {}", e)))?;

    // 规则 3：将 service 返回的 CSV（Vec<u8>）解析为 xlsx 表格
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(std::io::Cursor::new(csv_data));
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
        sheet_name: "销售订单".to_string(),
        headers,
        rows,
    };

    let filename = format!(
        "sales_orders_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("sales_order".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出销售订单（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/sales/orders/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "status_filter": query.status,
            "customer_id_filter": query.customer_id,
            "order_no_filter": query.order_no,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    build_xlsx_response(&table, &filename)
}

/// 生成销售订单号
/// GET /api/v1/erp/sales/orders/generate-no
///
/// 返回格式: `{ prefix: "SO", order_no: "SO20260617001" }`
pub async fn generate_order_no(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let order_no = DocumentNumberGenerator::generate_no(
        &*state.db,
        "SO",
        sales_order::Entity,
        sales_order::Column::OrderNo,
    )
    .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "prefix": "SO",
        "order_no": order_no
    }))))
}

// ========== 订单状态操作接口 ==========

/// 拒绝订单
/// POST /api/v1/erp/sales/orders/:id/reject
pub async fn reject_order(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    sales_service
        .reject_order(id, "订单被拒绝".to_string(), _auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("拒绝订单失败: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "订单已拒绝"
    }))))
}

/// 取消订单
/// POST /api/v1/erp/sales/orders/:id/cancel
pub async fn cancel_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    let _order = sales_service
        .cancel_order(id, auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("取消订单失败: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "订单已取消"
    }))))
}

// ========== 发货记录接口 ==========

/// 获取订单发货记录
/// GET /api/v1/erp/sales/orders/:id/deliveries
pub async fn get_order_deliveries(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    let deliveries = sales_service
        .get_order_deliveries(id)
        .await
        .map_err(|e| AppError::internal(format!("获取发货记录失败: {}", e)))?;

    let result = serde_json::json!({
        "list": deliveries,
        "total": deliveries.len(),
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 创建发货
/// POST /api/v1/erp/sales/orders/:id/deliveries
pub async fn create_delivery(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<CreateDeliveryDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P1-2d 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    payload
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    // 批次 407 修复：warehouse_id 缺失时不可默认为 0，否则发货可能落到非法仓库
    let warehouse_id = payload
        .warehouse_id
        .ok_or_else(|| AppError::validation("发货必须指定仓库 ID"))?;

    let delivery = sales_service
        .create_delivery(id, warehouse_id, auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("创建发货失败: {}", e)))?;

    let delivery_json = serde_json::to_value(delivery)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(delivery_json)))
}

/// 取消发货单
/// 批次 216 P2-1 修复（v12 复审）：实现销售发货取消功能
/// POST /api/v1/erp/sales/orders/:id/deliveries/:delivery_id/cancel
pub async fn cancel_delivery(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((_order_id, delivery_id)): Path<(i32, i32)>,
    Json(req): Json<CancelDeliveryRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    let delivery = sales_service
        .cancel_delivery(delivery_id, req.reason.clone(), auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("取消发货单失败: {}", e)))?;

    let delivery_json = serde_json::to_value(delivery)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        delivery_json,
        "发货单已取消",
    )))
}

/// 取消发货单请求 DTO（批次 216 P2-1）
#[derive(Debug, Deserialize, Validate)]
pub struct CancelDeliveryRequest {
    #[validate(length(min = 1, max = 500, message = "取消原因不能为空且最长500字符"))]
    pub reason: String,
}

// ========== 统计接口 ==========

/// 获取订单统计
/// GET /api/v1/erp/sales/orders/statistics
pub async fn get_order_statistics(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    let statistics = sales_service
        .get_order_statistics(query)
        .await
        .map_err(|e| AppError::internal(format!("获取订单统计失败: {}", e)))?;

    Ok(Json(ApiResponse::success(statistics)))
}
