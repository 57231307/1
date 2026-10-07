use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use validator::Validate;

use crate::models::dto::PageRequest;
use crate::models::sales_order;
use crate::services::so::order::SalesService;
use crate::services::so::{CreateSalesOrderRequest, UpdateSalesOrderRequest};
use crate::utils::admin_checker;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::optional_json::OptionalJson;
use crate::utils::response::ApiResponse;

/// 查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct SalesOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub order_no: Option<String>,
    /// 客户名称模糊查询（对应列表页的筛选输入框）
    pub customer_name: Option<String>,
    /// 订单日期范围（含端点）：页面日期区间控件发送 start_date/end_date
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

/// 创建发货请求 DTO：为 create_delivery 提供强类型入参与校验
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDeliveryDto {
    /// 仓库 ID：缺失时 create_delivery 返回 VALIDATION_ERROR（不默认 0）
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

    // 提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();

    let page_req = PageRequest {
        page: query.page.unwrap_or(1).clamp(1, 1000), // 分页参数 clamp 防 DoS
        page_size: query.page_size.unwrap_or(10).clamp(1, 100),
    };

    let orders = sales_service
        .list_orders(
            page_req,
            crate::services::so::order_query::SalesOrderFilter {
                status: query.status,
                customer_id: query.customer_id,
                order_no: query.order_no,
                customer_name: query.customer_name,
                start_date: query.start_date,
                end_date: query.end_date,
            },
            Some(&data_scope_ctx),
        )
        .await?;

    let mut orders_json = serde_json::to_value(orders)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        // 权限查询 Err 不静默：记 warn 后按 None（fail-closed 默认处理）继续，
        // 做法同本仓既有 crm_handler::resolve_role_data_permission。
        let permission = match state
            .data_permission_service
            .get_role_data_permission(role_id, "sales_order")
            .await
        {
            Ok(permission) => permission,
            Err(error) => {
                tracing::warn!(
                    %error,
                    role_id,
                    resource_type = "sales_order",
                    rule = "data_permission_lookup_fail_closed",
                    "角色数据权限查询失败，出参按无权限行 fail-closed 走默认处理"
                );
                None
            }
        };
        if let Some(permission) = permission {
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
                // 非管理员对销售订单列表手机号/邮箱脱敏。
                // PII 放行判据与下方金额/成本列隐藏同走
                // 本仓唯一权威源 `admin_checker::is_admin_role`（roles.code='admin'，
                // 查询失败 fail-closed=false），禁止角色主键字面量判定（播种漂移时
                // 各字段口径分裂、静默剔权/静默扩权）。
                // 判定在循环外、每请求至多一次（admin_checker 内部带 5 分钟缓存，
                // 同 crm_handler 既有范式）。
                let is_admin = admin_checker::is_admin_role(&state.db, role_id).await;
                for order in list.iter_mut() {
                    *order = crate::utils::field_mask::mask_contact_fields_for_role(
                        order.clone(),
                        is_admin,
                    );
                }
            }
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            // admin 判定走本仓唯一权威源
            // `admin_checker::is_admin_role`（roles.code='admin'，查询失败 fail-closed=false），
            // 禁止角色主键字面量判定（播种漂移时静默剔权/静默扩权）；判定在循环外的分支
            // 条件处、每请求至多一次（admin_checker 内部带 5 分钟缓存，同 crm_handler 既有范式）。
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

                        // 手机号脱敏（移除金额字段后仍需脱敏联系电话）。
                        // sales_orders 出参由 sales_order::Model 序列化生成，
                        // contact_phone 是真实列（models/sales_order.rs:32），
                        // 该模型无 email 列，出参不存在 email 类键。
                        if let Some(phone) = obj.get("contact_phone").and_then(|v| v.as_str()) {
                            if !phone.is_empty() {
                                obj.insert(
                                    "contact_phone".to_string(),
                                    serde_json::Value::String(
                                        crate::utils::field_mask::mask_phone(phone),
                                    ),
                                );
                            }
                        }

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
    // 提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let order = sales_service
        .get_order_detail(id, Some(&data_scope_ctx))
        .await?;
    let mut order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        // Err 记 warn 后同走 fail-closed 默认处理（与 list_orders 同款，不静默）
        let permission = match state
            .data_permission_service
            .get_role_data_permission(role_id, "sales_order")
            .await
        {
            Ok(permission) => permission,
            Err(error) => {
                tracing::warn!(
                    %error,
                    role_id,
                    resource_type = "sales_order",
                    rule = "data_permission_lookup_fail_closed",
                    "角色数据权限查询失败，出参按无权限行 fail-closed 走默认处理"
                );
                None
            }
        };
        if let Some(permission) = permission {
            // PII 放行判据与下方默认字段隐藏同走唯一权威源
            // `admin_checker::is_admin_role`，判定每请求一次、置于逐字段处理之外
            // （与 list_orders 同款，禁止角色主键字面量判定）。
            let is_admin = admin_checker::is_admin_role(&state.db, role_id).await;
            state.data_permission_service.filter_fields(
                &mut order_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
            // 非管理员对销售订单详情手机号/邮箱脱敏
            order_json =
                crate::utils::field_mask::mask_contact_fields_for_role(order_json, is_admin);
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            // 与列表同一单源判定（见 list_orders 内 admin 判定注释），每请求至多一次
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            if let Some(obj) = order_json.as_object_mut() {
                obj.remove("subtotal");
                obj.remove("tax_amount");
                obj.remove("discount_amount");
                obj.remove("shipping_cost");
                obj.remove("total_amount");
                obj.remove("paid_amount");
                obj.remove("balance_amount");

                // 手机号脱敏（sales_order::Model 无 email 列，
                // contact_phone 是真实列，models/sales_order.rs:32）
                if let Some(phone) = obj.get("contact_phone").and_then(|v| v.as_str()) {
                    if !phone.is_empty() {
                        obj.insert(
                            "contact_phone".to_string(),
                            serde_json::Value::String(crate::utils::field_mask::mask_phone(phone)),
                        );
                    }
                }

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
        return Err(AppError::from(e));
    }

    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    let order = sales_service.create_order(request, auth.user_id).await?;

    // 订单创建成功后发送通知
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 创建后订单为草稿态，通知语义为「已创建」；提交审批由 submit_order 发「订单已提交」
            if let Err(e) = event_service
                .notify_order_created(created_by, &order.order_no, order.id)
                .await
            {
                tracing::warn!(error = %e, order_id = order.id, "销售订单创建通知发送失败");
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
    // 输入验证：与 create_order 同口径下钻校验订单行（交货容差等）
    {
        use validator::Validate;
        if let Err(e) = request.validate() {
            return Err(AppError::from(e));
        }
    }
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());
    // IDOR 防护：更新前先校验资源归属（get_order_detail + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    sales_service
        .get_order_detail(id, Some(&data_scope_ctx))
        .await?;
    // 传入真实操作人 user_id 用于审计日志
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
    // IDOR 防护：删除前先校验资源归属（get_order_detail + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    sales_service
        .get_order_detail(id, Some(&data_scope_ctx))
        .await?;
    // 传入真实操作人 user_id 用于审计日志
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
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 通知发送失败记 warn 日志，不静默吞错
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
// 审批拆为「通过 / 拒绝」两条动作：本端点只处理通过，通过理由选填并落
// approval_reason 专列；拒绝动作在 /reject（理由落 rejected_reason 专列）。
pub async fn approve_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    payload: OptionalJson<ApproveSalesOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    // 入参形态用 `OptionalJson`（utils::optional_json 语义表）：`Option<Json<T>>`
    // 是假可选——axum 0.8.9 只在完全不带 Content-Type 时才放行，"带 JSON 头 + 空体"
    // 仍被解码层判 400。缺体/纯空白在此归一为 None ⇒ 列保持 NULL，不得伪造成必填、
    // 也不得落空串；有体但非法仍走 400 VALIDATION_ERROR。
    let approval_reason = payload
        .0
        .and_then(|r| r.approval_reason)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // 状态与通过理由在同一事务、同一行锁下单条 UPDATE 落库（服务侧
    // approve_order_with_reason）：理由写不进即状态一并回滚，不存在
    // "已批准但理由缺失"的半程态，也不产生第二条 UPDATE 审计行。
    let order = sales_service
        .approve_order_with_reason(id, auth.user_id, approval_reason)
        .await?;

    // 订单审批成功后发送通知给申请人
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 通知发送失败记 warn 日志，不静默吞错
            if let Err(e) = event_service
                .notify_order_approved(
                    created_by,
                    &order.order_no,
                    order.id,
                    auth.user_id,
                    &auth.username,
                )
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
    // 消除双源错位：路径 :id 与 payload.order_id 必须一致，否则以路径为准将错就错或
    // 静默发往另一订单（发货用 payload.order_id，详情/通知用 :id）。此处强校验，不一致直接拒绝，不静默。
    if payload.order_id != id {
        return Err(AppError::bad_request("发货订单 ID 与路径参数不一致"));
    }
    // IDOR 防护：发货前先按当前用户数据范围校验订单归属（复用 get_order_detail 内部的
    // validate_order_data_scope），与 customer/supplier 的「先 get_X(Some(&data_scope_ctx))」写法同源；
    // ship_order 服务侧仅 find_by_id+lock_exclusive 无归属校验，越权返回 403。
    let data_scope_ctx = auth.to_data_scope_context();
    sales_service
        .get_order_detail(id, Some(&data_scope_ctx))
        .await?;
    // 调用原有 ship_order(request, user_id)
    sales_service.ship_order(payload, auth.user_id).await?;
    // 重新获取订单详情用于通知（发货操作后内部调用，无数据权限过滤）
    let order = sales_service.get_order_detail(id, None).await?;

    // 订单发货成功后发送通知给申请人
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 通知发送失败记 warn 日志，不静默吞错
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
    // 传入真实操作人 ID 用于审计日志
    let order = sales_service.complete_order(id, auth.user_id).await?;

    // 订单完成后发送通知给申请人
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(event_service) = &state.event_notification_service {
        if let Some(created_by) = order.created_by {
            // 通知发送失败记 warn 日志，不静默吞错
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
#[allow(dead_code, reason = "反序列化输入字段")]
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
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 分页参数 clamp 防 DoS
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

use crate::utils::xlsx_export::{XlsxTable, build_xlsx_response};
// 导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::export_concurrency::ExportConcurrencyGuard;
use std::sync::Arc;

/// 导出销售订单
pub async fn export_orders(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<SalesOrderQuery>,
) -> Result<axum::response::Response, AppError> {
    // 全局导出并发控制（RAII 守卫，函数退出自动递减）
    let _guard = ExportConcurrencyGuard::acquire()?;

    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    // 提前 clone 查询条件用于审计日志（避免 service 调用 move 后借用失效）
    let audit_status = query.status.clone();
    let audit_order_no = query.order_no.clone();

    // 直接获取结构化数据生成表格
    let (headers, rows) = sales_service
        .export_orders_to_xlsx(crate::services::so::order_query::SalesOrderFilter {
            status: query.status,
            customer_id: query.customer_id,
            order_no: query.order_no,
            customer_name: query.customer_name,
            start_date: query.start_date,
            end_date: query.end_date,
        })
        .await?;

    let row_count = rows.len();

    // 销售订单导出条数上限：单次 ≤ 10000 条
    const MAX_SALES_ORDER_EXPORT_ROWS: usize = 10_000;
    if row_count > MAX_SALES_ORDER_EXPORT_ROWS {
        return Err(AppError::bad_request(format!(
            "销售订单导出条数 {} 超过上限 {}，请缩小筛选范围后重试",
            row_count, MAX_SALES_ORDER_EXPORT_ROWS
        )));
    }

    let table = XlsxTable {
        sheet_name: "销售订单".to_string(),
        headers,
        rows,
    };

    let filename = format!(
        "sales_orders_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    // 导出审计日志写入（best-effort，异步不阻塞响应）
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
            "status_filter": audit_status,
            "customer_id_filter": query.customer_id,
            "order_no_filter": audit_order_no,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    build_xlsx_response(&table, &filename)
}

/// 生成销售订单号 GET /api/v1/erp/sales/orders/generate-no；返回格式: `{ prefix: "SO", order_no: "SO20260617001" }`
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

/// 拒绝销售订单请求：长度下限与 purchase_order_handler.rs::RejectOrderRequest 同族，
/// 上限不设——sales_orders.rejected_reason 为 TEXT（migration m0079），无列宽截断风险；
/// PO 侧 255 上限是 VARCHAR(255) 列型对齐，两域列型不同不强行取同值。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RejectSalesOrderRequest {
    #[validate(length(min = 1, message = "拒绝原因不能为空"))]
    pub reason: String,
}

/// 审批通过请求体（选填档）：通过理由缺失/空串/纯空白一律按未采集落 NULL。
/// 字段保持 `Option<String>` 是为了让"缺键/不带 body"的调用与选填语义在同一
/// 形态下解码通过，不产生解码层裸 400。
#[derive(Debug, Deserialize)]
pub struct ApproveSalesOrderRequest {
    pub approval_reason: Option<String>,
}

/// 拒绝订单
/// POST /api/v1/erp/sales/orders/:id/reject
pub async fn reject_order(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<RejectSalesOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 校验真实执行：validator 的 min=1 拦不住纯空白，trim 非空门在后，
    // 落库为 trim 后的值（口径同 quotation_handler.rs / sales_price_handler.rs reject 先例）。
    req.validate()?;
    let reason = req.reason.trim().to_string();
    if reason.is_empty() {
        tracing::warn!(
            "用户 {} 拒绝销售订单被拒：单据 ID {id} 拒绝理由为纯空白（ID 只进日志不进文案）",
            _auth.user_id
        );
        return Err(AppError::validation_displayable("审批拒绝理由不能为空"));
    }

    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    // service 直接返回 AppError（状态机拒绝 business / 404 not_found 等 4xx），
    // 透传保留其 status/code/文案，不把业务拒绝压成 500。
    sales_service
        .reject_order(id, reason, _auth.user_id)
        .await?;

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

    let _order = sales_service.cancel_order(id, auth.user_id).await?;

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

    let deliveries = sales_service.get_order_deliveries(id).await?;

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
    // 强类型 DTO + validator 入参校验
    payload.validate().map_err(AppError::from)?;

    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    // warehouse_id 缺失即拒绝，不允许默认为 0 落到非法仓库
    let warehouse_id = payload
        .warehouse_id
        .ok_or_else(|| AppError::validation_displayable("发货必须指定仓库 ID"))?;

    let delivery = sales_service
        .create_delivery(id, warehouse_id, auth.user_id)
        .await?;

    let delivery_json = serde_json::to_value(delivery)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(delivery_json)))
}

/// 取消发货单 POST /api/v1/erp/sales/orders/:id/deliveries/:delivery_id/cancel
pub async fn cancel_delivery(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((_order_id, delivery_id)): Path<(i32, i32)>,
    Json(req): Json<CancelDeliveryRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate().map_err(AppError::from)?;

    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    let delivery = sales_service
        .cancel_delivery(delivery_id, req.reason.clone(), auth.user_id)
        .await?;

    let delivery_json = serde_json::to_value(delivery)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        delivery_json,
        "发货单已取消",
    )))
}

/// 取消发货单请求 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CancelDeliveryRequest {
    #[validate(length(min = 1, max = 500, message = "取消原因不能为空且最长500字符"))]
    pub reason: String,
}

// ========== 统计接口 ==========

/// 获取订单统计
/// GET /api/v1/erp/sales/orders/statistics
///
/// 本 handler 用 typed DTO 定型解析查询参数（非法值 400），再把 `customer_id` 以
/// `Value::Number` 重建后交给 service——service 用 `as_i64()` 读该键，而 urlencoded
/// 下值恒为 `Value::String`，透传原始 `Value` 会使 `customer_id` 筛选静默失效；
/// `start_date`/`end_date` 按原键名以字符串透传，保持 service 兼容。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct OrderStatisticsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub customer_id: Option<i64>,
}

pub async fn get_order_statistics(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(q): Query<OrderStatisticsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone(), state.search_client.clone());

    let mut params = serde_json::Map::new();
    if let Some(v) = q.start_date {
        params.insert("start_date".to_string(), serde_json::Value::String(v));
    }
    if let Some(v) = q.end_date {
        params.insert("end_date".to_string(), serde_json::Value::String(v));
    }
    if let Some(v) = q.customer_id {
        params.insert("customer_id".to_string(), serde_json::Value::from(v));
    }

    let statistics = sales_service
        .get_order_statistics(serde_json::Value::Object(params))
        .await?;

    Ok(Json(ApiResponse::success(statistics)))
}
