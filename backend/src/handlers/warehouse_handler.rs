#![allow(dead_code)]

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Deserialize;
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::location::Entity as LocationEntity;
use crate::models::location::{self as location_model};
use crate::services::warehouse_service::WarehouseService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 仓库列表
#[derive(Debug, Deserialize, Validate)]
pub struct WarehouseListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub search: Option<String>,
}

/// 创建仓库请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateWarehouseRequest {
    #[validate(length(min = 1, max = 100, message = "仓库名称不能为空"))]
    pub name: Option<String>,
    #[validate(length(min = 1, max = 50, message = "仓库编码不能为空"))]
    pub code: Option<String>,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub capacity: Option<i32>,
    pub description: Option<String>,
}

/// 更新仓库请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateWarehouseRequest {
    #[validate(length(min = 1, max = 100, message = "仓库名称不能为空"))]
    pub name: Option<String>,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub capacity: Option<i32>,
    pub status: Option<String>,
}

crate::define_crud_handlers!(
    WarehouseService,
    CreateWarehouseRequest,
    UpdateWarehouseRequest,
    WarehouseListQuery,
    i32
);

/// 查询参数 - 库位列表
#[derive(Debug, Deserialize)]
pub struct LocationListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
    pub search: Option<String>,
}

/// 创建库位请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateLocationRequest {
    #[validate(range(min = 1, message = "仓库ID必须大于0"))]
    pub warehouse_id: i32,
    #[validate(length(min = 1, max = 50, message = "库位编码不能为空且最长50字符"))]
    pub location_code: String,
    #[validate(length(max = 20, message = "库位类型最长20字符"))]
    pub location_type: Option<String>,
    pub max_weight: Option<f64>,
    pub max_height: Option<f64>,
}

/// 更新库位请求
#[derive(Debug, Deserialize)]
pub struct UpdateLocationRequest {
    pub location_name: Option<String>,
    pub location_type: Option<String>,
    pub zone: Option<String>,
    pub capacity: Option<f64>,
    pub status: Option<String>,
}

/// 获取库位列表
pub async fn list_locations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<LocationListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let mut query_builder = LocationEntity::find();

    if let Some(warehouse_id) = query.warehouse_id {
        query_builder = query_builder.filter(location_model::Column::WarehouseId.eq(warehouse_id));
    }

    let paginator = query_builder.paginate(&*state.db, page_size);
    let locations = paginator.fetch_page(page - 1).await?;
    let total = paginator.num_items().await?;

    let locations_json: Vec<serde_json::Value> = locations
        .into_iter()
        .map(|l| {
            serde_json::to_value(l).map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        locations_json,
        total,
        page,
        page_size,
    ))))
}

/// 创建库位
pub async fn create_location(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let active_location = location_model::ActiveModel {
        id: Default::default(),
        warehouse_id: sea_orm::ActiveValue::Set(req.warehouse_id),
        location_code: sea_orm::ActiveValue::Set(req.location_code),
        location_type: sea_orm::ActiveValue::Set(req.location_type),
        max_weight: sea_orm::ActiveValue::Set(
            req.max_weight
                .and_then(rust_decimal::Decimal::from_f64_retain),
        ),
        max_height: sea_orm::ActiveValue::Set(
            req.max_height
                .and_then(rust_decimal::Decimal::from_f64_retain),
        ),
        is_batch_managed: sea_orm::ActiveValue::Set(Some(true)),
        is_color_managed: sea_orm::ActiveValue::Set(Some(true)),
        created_at: Default::default(),
        updated_at: Default::default(),
    };

    let location = active_location.insert(&*state.db).await?;
    let location_json = serde_json::to_value(location)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        location_json,
        "库位创建成功",
    )))
}

/// 获取库位详情
pub async fn get_location(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let location = LocationEntity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("库位不存在"))?;
    let location_json = serde_json::to_value(location)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(location_json)))
}

/// 更新库位
pub async fn update_location(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(_req): Json<UpdateLocationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 待实现(v1.1): 增加具体的库位(Location)分配与更新逻辑
    let location = LocationEntity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("库位不存在"))?;
    let location_json = serde_json::to_value(location)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        location_json,
        "库位更新成功",
    )))
}

/// 删除库位
pub async fn delete_location(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    LocationEntity::delete_by_id(id).exec(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message((), "库位删除成功")))
}
