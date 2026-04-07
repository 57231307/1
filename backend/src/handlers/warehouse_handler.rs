use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
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
    pub warehouse_type: Option<String>,
    pub temperature_control: Option<bool>,
    pub humidity_control: Option<bool>,
    pub fire_protection_level: Option<String>,
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
    pub status: Option<String>,
    pub warehouse_type: Option<String>,
    pub temperature_control: Option<bool>,
    pub humidity_control: Option<bool>,
    pub fire_protection_level: Option<String>,
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
    pub location_type: Option<String>,
    pub max_weight: Option<f64>,
    pub max_height: Option<f64>,
    pub is_batch_managed: Option<bool>,
    pub is_color_managed: Option<bool>,
}

/// 更新库位请求
#[derive(Debug, Deserialize)]
pub struct UpdateLocationRequest {
    pub location_code: Option<String>,
    pub location_type: Option<String>,
    pub max_weight: Option<f64>,
    pub max_height: Option<f64>,
    pub is_batch_managed: Option<bool>,
    pub is_color_managed: Option<bool>,
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
            "active".to_string(),
            req.warehouse_type,
            req.temperature_control,
            req.humidity_control,
            req.fire_protection_level,
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
            req.status,
            req.warehouse_type,
            req.temperature_control,
            req.humidity_control,
            req.fire_protection_level,
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
        location_type: sea_orm::ActiveValue::Set(req.location_type),
        max_weight: sea_orm::ActiveValue::Set(
            req.max_weight
                .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default()),
        ),
        max_height: sea_orm::ActiveValue::Set(
            req.max_height
                .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default()),
        ),
        is_batch_managed: sea_orm::ActiveValue::Set(req.is_batch_managed),
        is_color_managed: sea_orm::ActiveValue::Set(req.is_color_managed),
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
    if let Some(location_type) = req.location_type {
        active_location.location_type = sea_orm::Set(Some(location_type));
    }
    if let Some(max_weight) = req.max_weight {
        active_location.max_weight = sea_orm::Set(Some(
            rust_decimal::Decimal::from_f64_retain(max_weight).unwrap_or_default(),
        ));
    }
    if let Some(max_height) = req.max_height {
        active_location.max_height = sea_orm::Set(Some(
            rust_decimal::Decimal::from_f64_retain(max_height).unwrap_or_default(),
        ));
    }
    if let Some(is_batch_managed) = req.is_batch_managed {
        active_location.is_batch_managed = sea_orm::Set(Some(is_batch_managed));
    }
    if let Some(is_color_managed) = req.is_color_managed {
        active_location.is_color_managed = sea_orm::Set(Some(is_color_managed));
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
