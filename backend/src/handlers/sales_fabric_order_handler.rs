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
use crate::utils::admin_checker;
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
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let (orders, total) = service
        .list_fabric_orders(crate::services::so::fabric_order::FabricOrderListQuery {
            page,
            page_size,
            customer_id: query.customer_id,
            order_no: query.order_no,
            status: query.status,
            batch_no: query.batch_no,
            color_no: query.color_no,
        })
        .await?;

    let orders_json: Vec<serde_json::Value> = orders
        .into_iter()
        .map(|o: sales_order::Model| {
            serde_json::to_value(o).map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // 面料订单行与客户域 sales_order 是**同一张表同一行**（services/so/fabric_order.rs 复用
    // sales_order::Entity），contact_phone/contact_person 为真实列（models/sales_order.rs:28-30）；
    // 标准入口 GET /sales/orders 列表已按权威列集合打码（sales_order_handler.rs:98），
    // 本入口整行原文即"同一份数据、不同入口不一致"的旁路，掩码实现只复用
    // utils/field_mask::mask_contact_fields_for_role 一份，不新造列名清单。
    // admin 判定走本仓唯一权威源 admin_checker::is_admin_role（roles.code='admin'，
    // role_id 缺失/查询失败 fail-closed=false，与原字面量判定下"无角色必脱敏"同方向），
    // 每请求在循环外算一次并复用，禁止下放进逐行循环（admin_checker 带 5 分钟缓存）。
    let is_admin = match auth.role_id {
        Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
        None => false,
    };
    let orders_json: Vec<serde_json::Value> = orders_json
        .into_iter()
        .map(|o| crate::utils::field_mask::mask_contact_fields_for_role(o, is_admin))
        .collect();

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
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    let order = service.get_fabric_order(id).await?;
    // 与列表/标准入口详情同一权威掩码实现（sales_order 行含 contact_phone 真实列）；
    // admin 判定走唯一权威源 admin_checker::is_admin_role（口径见 list_fabric_orders）
    let is_admin = match auth.role_id {
        Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
        None => false,
    };
    let order_json = crate::utils::field_mask::mask_contact_fields_for_role(
        serde_json::to_value(order)
            .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?,
        is_admin,
    );
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
    // 建单响应也是整行 sales_order（contact_phone/contact_person 当前由服务层恒置
    // None，但出参形状与已打码的 list/detail 同一张表同一批列）——同一掩码实现
    // 对齐，不靠"当前恒 None"放行原文（列一旦接入真实值即成旁路）。
    // admin 判定走唯一权威源 admin_checker::is_admin_role（口径见 list_fabric_orders）
    let is_admin = match auth.role_id {
        Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
        None => false,
    };
    let order_json = crate::utils::field_mask::mask_contact_fields_for_role(
        serde_json::to_value(created_order)
            .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?,
        is_admin,
    );
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "订单创建成功",
    )))
}

/// 更新销售订单
pub async fn update_fabric_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateFabricOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    let updated = service.update_fabric_order(id, req).await?;
    // 写响应不得整行原文回传 sales_order 行的 contact_phone/contact_person 明文
    // （标准入口详情打码、本入口原文 = 同一行出参不一致）；掩码实现只复用
    // utils/field_mask::mask_contact_fields_for_role 一份；
    // admin 判定走唯一权威源 admin_checker::is_admin_role（口径见 list_fabric_orders）
    let is_admin = match auth.role_id {
        Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
        None => false,
    };
    let order_json = crate::utils::field_mask::mask_contact_fields_for_role(
        serde_json::to_value(updated)
            .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?,
        is_admin,
    );
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
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesService::new(state.db.clone(), state.search_client.clone());
    let updated = service.approve_fabric_order(id).await?;
    // 同 update_fabric_order：审核写响应走同一权威掩码实现，不整行原文回传；
    // admin 判定走唯一权威源 admin_checker::is_admin_role（口径见 list_fabric_orders）
    let is_admin = match auth.role_id {
        Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
        None => false,
    };
    let order_json = crate::utils::field_mask::mask_contact_fields_for_role(
        serde_json::to_value(updated)
            .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?,
        is_admin,
    );
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "订单审核成功",
    )))
}
