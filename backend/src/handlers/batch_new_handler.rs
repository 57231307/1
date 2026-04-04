use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, TransactionTrait,
};
use serde::Deserialize;

use crate::models::inventory_stock;
use crate::utils::app_state::AppState;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 批次列表
#[derive(Debug, Deserialize)]
pub struct BatchListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    #[allow(dead_code)]
    pub product_id: Option<i32>,
    #[allow(dead_code)]
    pub batch_no: Option<String>,
    #[allow(dead_code)]
    pub color_no: Option<String>,
    #[allow(dead_code)]
    pub grade: Option<String>,
    #[allow(dead_code)]
    pub warehouse_id: Option<i32>,
    #[allow(dead_code)]
    pub start_date: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    pub end_date: Option<DateTime<Utc>>,
}

/// 创建批次请求（面料行业版）
#[derive(Debug, Deserialize)]
pub struct CreateBatchRequest {
    pub batch_no: String,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub color_no: String,
    #[allow(dead_code)]
    pub color_name: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub production_date: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    pub supplier_id: Option<i32>,
    #[allow(dead_code)]
    pub purchase_order_no: Option<String>,
    #[allow(dead_code)]
    pub remarks: Option<String>,
}

/// 更新批次请求
#[derive(Debug, Deserialize)]
pub struct UpdateBatchRequest {
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub remarks: Option<String>,
    pub stock_status: Option<String>,
    pub quality_status: Option<String>,
}

/// 批次转移请求
#[derive(Debug, Deserialize)]
pub struct TransferBatchRequest {
    #[allow(dead_code)]
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    #[allow(dead_code)]
    pub remarks: Option<String>,
}

/// 获取批次列表
pub async fn list_batches(
    State(state): State<AppState>,
    Query(query): Query<BatchListQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    // 使用库存服务查询批次
    match inventory_stock::Entity::find()
        .filter(inventory_stock::Column::BatchNo.ne(""))
        .paginate(&*state.db, page_size)
        .fetch_page(page - 1)
        .await
    {
        Ok(batches) => {
            let total = batches.len() as u64;
            let paginated = PaginatedResponse::new(batches, total, page, page_size);
            paginated.into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!(
                "获取批次列表失败：{}",
                e
            ))),
        )
            .into_response(),
    }
}

/// 获取批次详情
pub async fn get_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match inventory_stock::Entity::find_by_id(id).one(&*state.db).await {
        Ok(Some(batch)) => (StatusCode::OK, Json(ApiResponse::success(batch))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("批次不存在")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取批次失败：{}", e))),
        )
            .into_response(),
    }
}

/// 创建批次（入库）
pub async fn create_batch(
    State(state): State<AppState>,
    Json(req): Json<CreateBatchRequest>,
) -> impl IntoResponse {
    use crate::models::inventory_stock;
    use sea_orm::{ActiveModelTrait, Set};

    let batch = inventory_stock::ActiveModel {
        id: Set(0),
        warehouse_id: Set(req.warehouse_id),
        product_id: Set(req.product_id),
        batch_no: Set(req.batch_no.clone()),
        color_no: Set(req.color_no.clone()),
        dye_lot_no: Set(req.dye_lot_no),
        grade: Set(if req.grade.is_empty() {
            "一等品".to_string()
        } else {
            req.grade.clone()
        }),
        quantity_on_hand: Set(
            Decimal::from_f64_retain(req.quantity_meters).unwrap_or(Decimal::ZERO)
        ),
        quantity_available: Set(
            Decimal::from_f64_retain(req.quantity_meters).unwrap_or(Decimal::ZERO)
        ),
        quantity_reserved: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(Decimal::ZERO),
        reorder_quantity: Set(Decimal::ZERO),
        last_count_date: Set(None),
        last_movement_date: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        // 面料行业字段
        quantity_meters: Set(Decimal::from_f64_retain(req.quantity_meters).unwrap_or(Decimal::ZERO)),
        quantity_kg: Set(Decimal::from_f64_retain(req.quantity_kg).unwrap_or(Decimal::ZERO)),
        gram_weight: Set(req
            .gram_weight
            .and_then(rust_decimal::Decimal::from_f64_retain)),
        width: Set(req
            .width
            .and_then(rust_decimal::Decimal::from_f64_retain)),
        production_date: Set(req.production_date),
        expiry_date: Set(req.expiry_date),
        stock_status: Set("正常".to_string()),
        quality_status: Set("合格".to_string()),
        location_id: Set(None),
        shelf_no: Set(None),
        layer_no: Set(None),
        bin_location: Set(None),
    };

    match batch.insert(&*state.db).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(ApiResponse::success_with_msg(created, "批次创建成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("创建批次失败：{}", e))),
        )
            .into_response(),
    }
}

/// 更新批次
pub async fn update_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateBatchRequest>,
) -> impl IntoResponse {
    use crate::models::inventory_stock;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let mut batch: inventory_stock::ActiveModel =
        match inventory_stock::Entity::find_by_id(id).one(&*state.db).await {
            Ok(Some(b)) => b.into(),
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("批次不存在")),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(format!("获取批次失败：{}", e))),
                )
                    .into_response();
            }
        };

    if let Some(color) = req.color_no {
        batch.color_no = Set(color);
    }
    if let Some(dye_lot) = req.dye_lot_no {
        batch.dye_lot_no = Set(Some(dye_lot));
    }
    if let Some(grade) = req.grade {
        batch.grade = Set(grade);
    }
    if let Some(gw) = req.gram_weight {
        batch.gram_weight = Set(Some(
            rust_decimal::Decimal::from_f64_retain(gw).unwrap_or(rust_decimal::Decimal::ZERO),
        ));
    }
    if let Some(w) = req.width {
        batch.width = Set(Some(
            rust_decimal::Decimal::from_f64_retain(w).unwrap_or(rust_decimal::Decimal::ZERO),
        ));
    }
    if let Some(exp) = req.expiry_date {
        batch.expiry_date = Set(Some(exp));
    }
    if let Some(_remarks) = req.remarks {
        // 注意：inventory_stock 模型没有 remarks 字段，可以考虑使用其他方式存储
    }
    if let Some(status) = req.stock_status {
        batch.stock_status = Set(status);
    }
    if let Some(quality) = req.quality_status {
        batch.quality_status = Set(quality);
    }

    batch.updated_at = Set(Utc::now());

    match batch.update(&*state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "批次更新成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("更新批次失败：{}", e))),
        )
            .into_response(),
    }
}

/// 删除批次
pub async fn delete_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    use sea_orm::EntityTrait;

    match inventory_stock::Entity::delete_by_id(id).exec(&*state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg((), "批次删除成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("删除批次失败：{}", e))),
        )
            .into_response(),
    }
}

/// 批次转移（调拨）
pub async fn transfer_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<TransferBatchRequest>,
) -> impl IntoResponse {
    use crate::models::inventory_stock;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    // 开启事务
    let txn = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("开启事务失败：{}", e))),
            );
        }
    };

    // 查询源批次
    let source_batch = match inventory_stock::Entity::find_by_id(id).one(&txn).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("源批次不存在")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("获取批次失败：{}", e))),
            );
        }
    };

    let transfer_meters = Decimal::from_f64_retain(req.quantity_meters).unwrap_or(Decimal::ZERO);
    let transfer_kg = Decimal::from_f64_retain(req.quantity_kg).unwrap_or(Decimal::ZERO);

    // 检查库存是否足够
    if source_batch.quantity_available < transfer_meters {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("库存数量不足")),
        );
    }

    // 更新源批次库存
    let source_product_id = source_batch.product_id;
    let source_batch_no = source_batch.batch_no.clone();
    let source_color_no = source_batch.color_no.clone();
    let source_quantity_on_hand = source_batch.quantity_on_hand;
    let source_quantity_available = source_batch.quantity_available;
    let source_dye_lot_no = source_batch.dye_lot_no.clone();
    let source_grade = source_batch.grade.clone();
    let source_gram_weight = source_batch.gram_weight;
    let source_width = source_batch.width;
    let source_production_date = source_batch.production_date;
    let source_expiry_date = source_batch.expiry_date;
    let mut source: inventory_stock::ActiveModel = source_batch.into();
    source.quantity_on_hand = Set(source_quantity_on_hand - transfer_meters);
    source.quantity_available = Set(source_quantity_available - transfer_meters);
    source.updated_at = Set(Utc::now());

    if let Err(e) = source.update(&txn).await {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("更新源批次失败：{}", e))),
        );
    }

    // 查询目标仓库是否已有相同批次
    let target_batch = inventory_stock::Entity::find()
        .filter(inventory_stock::Column::WarehouseId.eq(req.to_warehouse_id))
        .filter(inventory_stock::Column::ProductId.eq(source_product_id))
        .filter(inventory_stock::Column::BatchNo.eq(source_batch_no.clone()))
        .filter(inventory_stock::Column::ColorNo.eq(source_color_no.clone()))
        .one(&txn)
        .await;

    match target_batch {
        Ok(Some(existing)) => {
            // 更新现有批次
            let mut target: inventory_stock::ActiveModel = existing.clone().into();
            target.quantity_on_hand = Set(existing.quantity_on_hand + transfer_meters);
            target.quantity_available = Set(existing.quantity_available + transfer_meters);
            target.updated_at = Set(Utc::now());

            if let Err(e) = target.update(&txn).await {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(format!(
                        "更新目标批次失败：{}",
                        e
                    ))),
                );
            }
        }
        Ok(None) => {
            // 创建新批次
            let new_batch = inventory_stock::ActiveModel {
                id: Set(0),
                warehouse_id: Set(req.to_warehouse_id),
                product_id: Set(source_product_id),
                batch_no: Set(source_batch_no.clone()),
                color_no: Set(source_color_no.clone()),
                dye_lot_no: Set(source_dye_lot_no.clone()),
                grade: Set(source_grade.clone()),
                quantity_on_hand: Set(transfer_meters),
                quantity_available: Set(transfer_meters),
                quantity_reserved: Set(Decimal::ZERO),
                quantity_incoming: Set(Decimal::ZERO),
                reorder_point: Set(Decimal::ZERO),
                reorder_quantity: Set(Decimal::ZERO),
                bin_location: Set(None),
                last_count_date: Set(None),
                last_movement_date: Set(None),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
                quantity_meters: Set(transfer_meters),
                quantity_kg: Set(transfer_kg),
                gram_weight: Set(source_gram_weight),
                width: Set(source_width),
                production_date: Set(source_production_date),
                expiry_date: Set(source_expiry_date),
                stock_status: Set("正常".to_string()),
                quality_status: Set("合格".to_string()),
                location_id: Set(None),
                shelf_no: Set(None),
                layer_no: Set(None),
            };

            if let Err(e) = new_batch.insert(&txn).await {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(format!(
                        "创建目标批次失败：{}",
                        e
                    ))),
                );
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!(
                    "查询目标批次失败：{}",
                    e
                ))),
            );
        }
    }

    // 提交事务
    if let Err(e) = txn.commit().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("提交事务失败：{}", e))),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success_with_msg((), "批次转移成功")),
    )
}
