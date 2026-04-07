use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde::Deserialize;

use crate::models::location::Entity as LocationEntity;
use crate::models::location::{self as location_model};
use crate::services::warehouse_service::WarehouseService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 仓库列表
#[derive(Debug, Deserialize)]
pub struct WarehouseListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub search: Option<String>,
}

/// 创建仓库请求
#[derive(Debug, Deserialize)]
pub struct CreateWarehouseRequest {
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub capacity: Option<i32>,
    #[allow(dead_code)]
    pub description: Option<String>,
}

/// 更新仓库请求
#[derive(Debug, Deserialize)]
pub struct UpdateWarehouseRequest {
    pub name: Option<String>,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub capacity: Option<i32>,
    pub status: Option<String>,
}

/// 查询参数 - 库位列表
#[derive(Debug, Deserialize)]
pub struct LocationListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
    #[allow(dead_code)]
    pub search: Option<String>,
}

/// 创建库位请求
#[derive(Debug, Deserialize)]
pub struct CreateLocationRequest {
    pub warehouse_id: i32,
    pub location_code: String,
    pub shelf_no: String,
    pub layer_no: String,
    pub position_no: String,
    pub max_capacity: f64,
    pub remarks: Option<String>,
}

/// 更新库位请求
#[derive(Debug, Deserialize)]
pub struct UpdateLocationRequest {
    pub location_code: Option<String>,
    pub shelf_no: Option<String>,
    pub layer_no: Option<String>,
    pub position_no: Option<String>,
    pub max_capacity: Option<f64>,
    pub remarks: Option<String>,
    pub is_active: Option<bool>,
}

/// 获取仓库列表
pub async fn list_warehouses(
    State(state): State<AppState>,
    Query(query): Query<WarehouseListQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let warehouse_service = WarehouseService::new(state.db.clone());
    let warehouses = warehouse_service
        .list_warehouses(page, page_size, query.status, query.search)
        .await?;

    let (warehouses_vec, total) = warehouses;
    let warehouses_json: Vec<serde_json::Value> = warehouses_vec
        .into_iter()
        .map(|w| serde_json::to_value(w).unwrap_or_default())
        .collect();

    Ok(Json(
        PaginatedResponse::new(warehouses_json, total, page, page_size).into(),
    ))
}

/// 获取仓库详情
pub async fn get_warehouse(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let warehouse_service = WarehouseService::new(state.db.clone());
    let warehouse = warehouse_service.get_warehouse(id).await?;
    let warehouse_json = serde_json::to_value(warehouse).unwrap_or_default();
    Ok(Json(ApiResponse::success(warehouse_json)))
}

/// 创建仓库
pub async fn create_warehouse(
    State(state): State<AppState>,
    Json(req): Json<CreateWarehouseRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let warehouse_service = WarehouseService::new(state.db.clone());
    let warehouse = warehouse_service
        .create_warehouse(
            req.name,
            req.code,
            req.address,
            req.manager,
            req.phone,
            req.capacity,
            "active".to_string(),
        )
        .await?;
    let warehouse_json = serde_json::to_value(warehouse).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        warehouse_json,
        "仓库创建成功",
    )))
}

/// 更新仓库
pub async fn update_warehouse(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWarehouseRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let warehouse_service = WarehouseService::new(state.db.clone());
    let warehouse = warehouse_service
        .update_warehouse(
            id,
            req.name,
            req.address,
            req.manager,
            req.phone,
            req.capacity,
            req.status,
        )
        .await?;
    let warehouse_json = serde_json::to_value(warehouse).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        warehouse_json,
        "仓库更新成功",
    )))
}

/// 删除仓库
pub async fn delete_warehouse(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let warehouse_service = WarehouseService::new(state.db.clone());
    warehouse_service.delete_warehouse(id).await?;
    Ok(Json(ApiResponse::success_with_msg((), "仓库删除成功")))
}

/// 获取库位列表
pub async fn list_locations(
    State(state): State<AppState>,
    Query(query): Query<LocationListQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let mut query_builder = LocationEntity::find();

    if let Some(warehouse_id) = query.warehouse_id {
        query_builder = query_builder.filter(location_model::Column::WarehouseId.eq(warehouse_id));
    }

    let paginator = query_builder.paginate(&*state.db, page_size);
    let locations = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let total = paginator
        .num_items()
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let locations_json: Vec<serde_json::Value> = locations
        .into_iter()
        .map(|l| serde_json::to_value(l).unwrap_or_default())
        .collect();

    Ok(Json(
        PaginatedResponse::new(locations_json, total, page, page_size).into(),
    ))
}

/// 创建库位
pub async fn create_location(
    State(state): State<AppState>,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let active_location = location_model::ActiveModel {
        id: Default::default(),
        warehouse_id: sea_orm::ActiveValue::Set(req.warehouse_id),
        location_code: sea_orm::ActiveValue::Set(req.location_code),
        shelf_no: sea_orm::ActiveValue::Set(req.shelf_no),
        layer_no: sea_orm::ActiveValue::Set(req.layer_no),
        position_no: sea_orm::ActiveValue::Set(req.position_no),
        max_capacity: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::from_f64_retain(req.max_capacity).unwrap_or_default(),
        ),
        current_usage: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
        remarks: sea_orm::ActiveValue::Set(req.remarks),
        is_active: sea_orm::ActiveValue::Set(true),
        created_at: Default::default(),
        updated_at: Default::default(),
    };

    let location = active_location.insert(&*state.db).await?;
    let location_json = serde_json::to_value(location).unwrap_or_default();
    Ok(Json(ApiResponse::success_with_msg(
        location_json,
        "库位创建成功",
    )))
}

/// 获取库位详情
pub async fn get_location(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let location = LocationEntity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("库位不存在".to_string()))?;
    let location_json = serde_json::to_value(location).unwrap_or_default();
    Ok(Json(ApiResponse::success(location_json)))
}

/// 更新库位
pub async fn update_location(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLocationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let location = LocationEntity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("库位不存在".to_string()))?;

    let mut active_location: crate::models::location::ActiveModel = location.into();

    if let Some(location_code) = req.location_code {
        active_location.location_code = sea_orm::Set(location_code);
    }
    if let Some(shelf_no) = req.shelf_no {
        active_location.shelf_no = sea_orm::Set(shelf_no);
    }
    if let Some(layer_no) = req.layer_no {
        active_location.layer_no = sea_orm::Set(layer_no);
    }
    if let Some(position_no) = req.position_no {
        active_location.position_no = sea_orm::Set(position_no);
    }
    if let Some(max_capacity) = req.max_capacity {
        active_location.max_capacity = sea_orm::Set(rust_decimal::Decimal::from_f64_retain(max_capacity).unwrap_or_default());
    }
    if let Some(remarks) = req.remarks {
        active_location.remarks = sea_orm::Set(Some(remarks));
    }
    if let Some(is_active) = req.is_active {
        active_location.is_active = sea_orm::Set(is_active);
    }
    
    active_location.updated_at = sea_orm::Set(chrono::Utc::now());

    let updated_location = active_location.update(&*state.db).await?;
    let location_json = serde_json::to_value(updated_location).unwrap_or_default();

    Ok(Json(ApiResponse::success_with_msg(
        location_json,
        "库位更新成功",
    )))
}

/// 删除库位
pub async fn delete_location(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    LocationEntity::delete_by_id(id).exec(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_msg((), "库位删除成功")))
}
