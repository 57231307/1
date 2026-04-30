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
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 销售订单列表
#[derive(Debug, Deserialize)]
pub struct FabricOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub customer_id: Option<i32>,
    pub order_no: Option<String>,
    pub status: Option<String>,
    #[allow(dead_code)]
    pub batch_no: Option<String>,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub payment_terms: Option<String>,
    #[allow(dead_code)]
    pub remarks: Option<String>,
    #[allow(dead_code)]
    pub batch_no: Option<String>,
    #[allow(dead_code)]
    pub color_no: Option<String>,
    #[allow(dead_code)]
    pub dye_lot_no: Option<String>,
    #[allow(dead_code)]
    pub grade: Option<String>,
    #[allow(dead_code)]
    pub packaging_requirement: Option<String>,
    #[allow(dead_code)]
    pub quality_standard: Option<String>,
}

/// 销售订单明细请求（面料行业版）
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FabricOrderItemRequest {
    pub product_id: i32,
    #[allow(dead_code)]
    pub product_name: Option<String>,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub unit_price_meters: f64,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub color_no: String,
    #[allow(dead_code)]
    pub batch_no: Option<String>,
    #[allow(dead_code)]
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
    pub pantone_code: Option<String>,
    pub color_name: Option<String>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<f64>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    pub color_extra_cost: Option<f64>,
    pub grade_price_diff: Option<f64>,
    pub final_price: Option<f64>,
}

/// 更新销售订单请求
#[derive(Debug, Deserialize)]
pub struct UpdateFabricOrderRequest {
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    #[allow(dead_code)]
    pub payment_terms: Option<String>,
    #[allow(dead_code)]
    pub remarks: Option<String>,
    #[allow(dead_code)]
    pub items: Option<Vec<FabricOrderItemRequest>>,
    #[allow(dead_code)]
    pub batch_no: Option<String>,
    #[allow(dead_code)]
    pub color_no: Option<String>,
    #[allow(dead_code)]
    pub packaging_requirement: Option<String>,
    #[allow(dead_code)]
    pub quality_standard: Option<String>,
}

/// 获取销售订单列表（面料行业版）
pub async fn list_fabric_orders(
    State(state): State<AppState>,
    Query(query): Query<FabricOrderQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let mut query_builder = sales_order::Entity::find();

    if let Some(cid) = query.customer_id {
        query_builder = query_builder.filter(sales_order::Column::CustomerId.eq(cid));
    }

    if let Some(no) = query.order_no {
        query_builder =
            query_builder.filter(sales_order::Column::OrderNo.like(format!("%{}%", no)));
    }

    if let Some(status) = query.status {
        query_builder = query_builder.filter(sales_order::Column::Status.eq(status));
    }

    let paginator = query_builder
        .order_by(sales_order::Column::CreatedAt, Order::Desc)
        .paginate(&*state.db, page_size);
    let orders = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let total = paginator
        .num_items()
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let orders_json: Vec<serde_json::Value> = orders
        .into_iter()
        .map(|o| serde_json::to_value(o).unwrap_or_default())
        .collect();

    Ok(Json(
        PaginatedResponse::new(orders_json, total, page, page_size).into(),
    ))
}

/// 获取销售订单详情
pub async fn get_fabric_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let order = sales_order::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("订单不存在".to_string()))?;

    let order_json = serde_json::to_value(order).unwrap_or_default();
    Ok(Json(ApiResponse::success(order_json)))
}

/// 创建销售订单（面料行业版）
pub async fn create_fabric_order(
    State(state): State<AppState>,
    Json(req): Json<CreateFabricOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use chrono::Utc;
    use rust_decimal::Decimal;

    // 开启事务
    let txn = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("开启事务失败：{}", e)))?;

    // 生成订单号
    let order_no = format!("SO{}", Utc::now().format("%Y%m%d%H%M%S"));

    // 计算总金额和总数量
    let mut total_amount = Decimal::ZERO;
    let mut total_quantity_meters = Decimal::ZERO;
    let mut total_quantity_kg = Decimal::ZERO;

    for item in &req.items {
        let quantity_meters = Decimal::from_f64_retain(item.quantity_meters).unwrap_or_default();
        let unit_price = Decimal::from_f64_retain(item.unit_price_meters).unwrap_or_default();
        let quantity_kg = Decimal::from_f64_retain(item.quantity_kg).unwrap_or_default();

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
        .map_err(|e| AppError::BadRequest(format!("创建订单失败：{}", e)))?;

    // 创建订单明细
    for item in &req.items {
        let quantity_meters = Decimal::from_f64_retain(item.quantity_meters).unwrap_or_default();
        let quantity_kg = Decimal::from_f64_retain(item.quantity_kg).unwrap_or_default();
        let base_price_val =
            Decimal::from_f64_retain(item.base_price.unwrap_or(item.unit_price_meters))
                .unwrap_or_default();
        let color_extra =
            Decimal::from_f64_retain(item.color_extra_cost.unwrap_or(0.0)).unwrap_or_default();
        let grade_diff =
            Decimal::from_f64_retain(item.grade_price_diff.unwrap_or(0.0)).unwrap_or_default();
        let final_p = Decimal::from_f64_retain(item.final_price.unwrap_or(item.unit_price_meters))
            .unwrap_or_default();
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
            gram_weight: Set(Some(
                Decimal::from_f64_retain(item.gram_weight.unwrap_or(0.0)).unwrap_or_default(),
            )),
            width: Set(Some(
                Decimal::from_f64_retain(item.width.unwrap_or(0.0)).unwrap_or_default(),
            )),
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
            .map_err(|e| AppError::BadRequest(format!("创建订单明细失败：{}", e)))?;
    }

    // 提交事务
    txn.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("提交事务失败：{}", e)))?;

    let order_json = serde_json::to_value(created_order).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
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
        .ok_or_else(|| AppError::NotFound("订单不存在".to_string()))?
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
        .map_err(|e| AppError::BadRequest(format!("更新订单失败：{}", e)))?;
    let order_json = serde_json::to_value(updated).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        order_json,
        "订单更新成功",
    )))
}

/// 删除销售订单
pub async fn delete_fabric_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    sales_order::Entity::delete_by_id(id)
        .exec(&*state.db)
        .await
        .map_err(|e| AppError::BadRequest(format!("删除订单失败：{}", e)))?;

    Ok(Json(ApiResponse::success_with_msg((), "订单删除成功")))
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
        .ok_or_else(|| AppError::NotFound("订单不存在".to_string()))?
        .into();

    order.status = Set("approved".to_string());
    order.approved_by = Set(None);
    order.approved_at = Set(Some(Utc::now()));
    order.updated_at = Set(Utc::now());

    let updated = order
        .update(&*state.db)
        .await
        .map_err(|e| AppError::BadRequest(format!("审核订单失败：{}", e)))?;
    let order_json = serde_json::to_value(updated).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        order_json,
        "订单审核成功",
    )))
}
