
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use serde::Deserialize;

use crate::models::sales_order;
use crate::models::sales_order_item;
use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use crate::utils::sql_escape::safe_like_pattern;

/// 查询参数 - 销售订单列表
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

/// 创建销售订单请求（面料行业版）
#[derive(Debug, Deserialize)]
pub struct CreateFabricOrderRequest {
    pub customer_id: i32,
    pub order_date: chrono::DateTime<chrono::Utc>,
    pub required_date: chrono::DateTime<chrono::Utc>,
    pub items: Vec<FabricOrderItemRequest>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

/// 销售订单明细请求（面料行业版）
///
/// 批次 86 v2 复审 P2-11 修复：金额/数量字段 f64 → Decimal（消除精度漂移），
/// 在 handler 入口添加非负校验 + round_dp(2) 精度校验
#[derive(Debug, Deserialize)]
pub struct FabricOrderItemRequest {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub quantity_meters: rust_decimal::Decimal,
    pub quantity_kg: rust_decimal::Decimal,
    pub unit_price_meters: rust_decimal::Decimal,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub color_no: String,
    pub batch_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
    pub pantone_code: Option<String>,
    pub color_name: Option<String>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<rust_decimal::Decimal>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    pub color_extra_cost: Option<rust_decimal::Decimal>,
    pub grade_price_diff: Option<rust_decimal::Decimal>,
    pub final_price: Option<rust_decimal::Decimal>,
}

/// 更新销售订单请求
#[derive(Debug, Deserialize)]
pub struct UpdateFabricOrderRequest {
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<FabricOrderItemRequest>>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

/// 获取销售订单列表（面料行业版）
pub async fn list_fabric_orders(
    State(state): State<AppState>,
    Query(query): Query<FabricOrderQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut query_builder = sales_order::Entity::find();

    if let Some(cid) = query.customer_id {
        query_builder = query_builder.filter(sales_order::Column::CustomerId.eq(cid));
    }

    if let Some(no) = query.order_no {
        let pattern = safe_like_pattern(&no);
        query_builder = query_builder.filter(sales_order::Column::OrderNo.like(&pattern));
    }

    if let Some(status) = query.status {
        query_builder = query_builder.filter(sales_order::Column::Status.eq(status));
    }

    let paginator = query_builder
        .order_by(sales_order::Column::CreatedAt, Order::Desc)
        .paginate(&*state.db, page_size);
    let orders = paginator.fetch_page(page.saturating_sub(1)).await?;
    let total = paginator.num_items().await?;

    let orders_json: Vec<serde_json::Value> = orders
        .into_iter()
        .map(|o| {
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
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let order = sales_order::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("订单不存在"))?;

    let order_json = serde_json::to_value(order)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(order_json)))
}

/// 创建销售订单（面料行业版）
pub async fn create_fabric_order(
    State(state): State<AppState>,
    Json(req): Json<CreateFabricOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use chrono::Utc;
    use rust_decimal::Decimal;

    // P2-11 修复（批次 86 v2 复审）：金额/数量非负校验 + round_dp(2) 精度校验
    for (idx, item) in req.items.iter().enumerate() {
        if item.quantity_meters < Decimal::ZERO {
            return Err(AppError::validation(format!(
                "第 {} 项 quantity_meters 不能为负数",
                idx + 1
            )));
        }
        if item.quantity_kg < Decimal::ZERO {
            return Err(AppError::validation(format!(
                "第 {} 项 quantity_kg 不能为负数",
                idx + 1
            )));
        }
        if item.unit_price_meters < Decimal::ZERO {
            return Err(AppError::validation(format!(
                "第 {} 项 unit_price_meters 不能为负数",
                idx + 1
            )));
        }
        // 金额字段最多 2 位小数（货币精度）
        if item.unit_price_meters.round_dp(2) != item.unit_price_meters {
            return Err(AppError::validation(format!(
                "第 {} 项 unit_price_meters 精度不能超过 2 位小数",
                idx + 1
            )));
        }
        if let Some(p) = item.base_price {
            if p < Decimal::ZERO {
                return Err(AppError::validation(format!(
                    "第 {} 项 base_price 不能为负数",
                    idx + 1
                )));
            }
            if p.round_dp(2) != p {
                return Err(AppError::validation(format!(
                    "第 {} 项 base_price 精度不能超过 2 位小数",
                    idx + 1
                )));
            }
        }
        if let Some(p) = item.final_price {
            if p < Decimal::ZERO {
                return Err(AppError::validation(format!(
                    "第 {} 项 final_price 不能为负数",
                    idx + 1
                )));
            }
            if p.round_dp(2) != p {
                return Err(AppError::validation(format!(
                    "第 {} 项 final_price 精度不能超过 2 位小数",
                    idx + 1
                )));
            }
        }
    }

    // 开启事务
    let txn = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::internal(format!("开启事务失败：{}", e)))?;

    // 生成订单号
    let order_no = format!("SO{}", Utc::now().format("%Y%m%d%H%M%S"));

    // 计算总金额和总数量
    let mut total_amount = Decimal::ZERO;
    let mut total_quantity_meters = Decimal::ZERO;
    let mut total_quantity_kg = Decimal::ZERO;

    for item in &req.items {
        // P2-11 修复：字段已是 Decimal，无需 from_f64_retain 转换
        let quantity_meters = item.quantity_meters;
        let unit_price = item.unit_price_meters;
        let quantity_kg = item.quantity_kg;

        let amount = quantity_meters * unit_price;
        total_amount += amount;
        total_quantity_meters += quantity_meters;
        total_quantity_kg += quantity_kg;
    }

    // 创建订单主表
    let order = sales_order::ActiveModel {
        id: Set(0),
        order_no: Set(order_no.clone()),
        customer_id: Set(req.customer_id),
        opportunity_id: Set(None),
        order_date: Set(req.order_date),
        required_date: Set(req.required_date),
        ship_date: Set(None),
        status: Set("pending".to_string()),
        subtotal: Set(total_amount),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(total_amount),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(total_amount),
        shipping_address: Set(req.shipping_address),
        billing_address: Set(req.delivery_address),
        notes: Set(req.remarks), // 使用 notes 字段存储备注
        created_by: Set(None),
        approved_by: Set(None),
        approved_at: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    };

    let created_order = order
        .insert(&txn)
        .await
        .map_err(|e| AppError::bad_request(format!("创建订单失败：{}", e)))?;

    // 创建订单明细
    for item in &req.items {
        // P2-11 修复：字段已是 Decimal，无需 from_f64_retain 转换
        let quantity_meters = item.quantity_meters;
        let quantity_kg = item.quantity_kg;
        let base_price_val = item.base_price.unwrap_or(item.unit_price_meters);
        let color_extra = item.color_extra_cost.unwrap_or_default();
        let grade_diff = item.grade_price_diff.unwrap_or_default();
        let final_p = item.final_price.unwrap_or(item.unit_price_meters);
        let subtotal = quantity_meters * final_p;

        let order_item = sales_order_item::ActiveModel {
            id: Set(0),
            order_id: Set(created_order.id),
            product_id: Set(item.product_id),
            quantity: Set(quantity_meters),
            unit_price: Set(final_p),
            discount_percent: Set(Decimal::ZERO),
            tax_percent: Set(Decimal::ZERO),
            subtotal: Set(subtotal),
            tax_amount: Set(Decimal::ZERO),
            discount_amount: Set(Decimal::ZERO),
            total_amount: Set(subtotal),
            shipped_quantity: Set(Decimal::ZERO),
            notes: Set(item.remarks.clone()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            color_no: Set(item.color_no.clone()),
            color_name: Set(item.color_name.clone()),
            pantone_code: Set(item.pantone_code.clone()),
            grade_required: Set(item.grade.clone()),
            quantity_meters: Set(quantity_meters),
            quantity_kg: Set(quantity_kg),
            gram_weight: Set(item.gram_weight),
            width: Set(item.width),
            batch_requirement: Set(item.batch_requirement.clone()),
            dye_lot_requirement: Set(item.dye_lot_requirement.clone()),
            base_price: Set(Some(base_price_val)),
            color_extra_cost: Set(color_extra),
            grade_price_diff: Set(grade_diff),
            final_price: Set(Some(final_p)),
            shipped_quantity_meters: Set(Decimal::ZERO),
            shipped_quantity_kg: Set(Decimal::ZERO),
            paper_tube_weight: Set(item.paper_tube_weight),
            is_net_weight: Set(item.is_net_weight),
        };

        order_item
            .insert(&txn)
            .await
            .map_err(|e| AppError::bad_request(format!("创建订单明细失败：{}", e)))?;
    }

    // 提交事务
    txn.commit()
        .await
        .map_err(|e| AppError::internal(format!("提交事务失败：{}", e)))?;

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
    Path(id): Path<i32>,
    Json(req): Json<UpdateFabricOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let mut order: sales_order::ActiveModel = sales_order::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("订单不存在"))?
        .into();

    if let Some(date) = req.required_date {
        order.required_date = Set(date);
    }
    if let Some(status) = req.status {
        order.status = Set(status);
    }
    if let Some(addr) = req.shipping_address {
        order.shipping_address = Set(Some(addr));
    }
    if let Some(addr) = req.delivery_address {
        order.billing_address = Set(Some(addr));
    }
    // 注意：sales_order 模型没有 payment_terms, remarks, batch_no, color_no 等字段
    // 如有需要，可以考虑使用 notes 字段或其他方式存储

    order.updated_at = Set(chrono::Utc::now());

    let updated = order
        .update(&*state.db)
        .await
        .map_err(|e| AppError::bad_request(format!("更新订单失败：{}", e)))?;
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
    // P0 8-3 修复：delete 操作补审计日志
    // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
    crate::services::audit_log_service::AuditLogService::delete_with_audit::<
        sales_order::Entity,
        _,
    >(&*state.db, "sales_fabric_order", id, Some(auth.user_id))
    .await?;

    Ok(Json(ApiResponse::success_with_message((), "订单删除成功")))
}

/// 审核订单
pub async fn approve_fabric_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use chrono::Utc;

    let mut order: sales_order::ActiveModel = sales_order::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("订单不存在"))?
        .into();

    order.status = Set("approved".to_string());
    order.approved_by = Set(None);
    order.approved_at = Set(Some(Utc::now()));
    order.updated_at = Set(Utc::now());

    let updated = order
        .update(&*state.db)
        .await
        .map_err(|e| AppError::bad_request(format!("审核订单失败：{}", e)))?;
    let order_json = serde_json::to_value(updated)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        order_json,
        "订单审核成功",
    )))
}
