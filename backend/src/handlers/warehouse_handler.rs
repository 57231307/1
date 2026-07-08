
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
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
    /// 仓库容量（批次 158 v11 真实接入：扩展 schema 持久化，原 #[allow(dead_code)] 移除）
    pub capacity: Option<i32>,
    // 批次 93 P1 扩展：description 已接入 WarehouseService::create（写入 notes 列）
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
    /// 仓库容量（批次 158 v11 真实接入：扩展 schema 持久化，原 #[allow(dead_code)] 移除）
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

/// 更新库位请求（字段对齐 warehouse_locations 表结构）
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateLocationRequest {
    /// 库位编码
    #[validate(length(min = 1, max = 50, message = "库位编码不能为空且最长50字符"))]
    pub location_code: Option<String>,
    /// 库位类型
    #[validate(length(max = 20, message = "库位类型最长20字符"))]
    pub location_type: Option<String>,
    /// 最大承重
    pub max_weight: Option<f64>,
    /// 最大高度
    pub max_height: Option<f64>,
    /// 是否启用批次管理
    pub is_batch_managed: Option<bool>,
    /// 是否启用色号管理
    pub is_color_managed: Option<bool>,
}

/// 获取库位列表
pub async fn list_locations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<LocationListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let mut query_builder = LocationEntity::find();

    if let Some(warehouse_id) = query.warehouse_id {
        query_builder = query_builder.filter(location_model::Column::WarehouseId.eq(warehouse_id));
    }

    let paginator = query_builder.paginate(&*state.db, page_size);
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let locations = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;
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
    // v8 P1-C 修复：调用 DTO 验证，激活 Validate 注解
    req.validate()?;
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

/// 更新库位（批次 197 P0-1：真实接入字段更新逻辑，原 stub 仅返回原记录）
pub async fn update_location(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLocationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()?;
    let location = LocationEntity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("库位不存在"))?;
    let mut active: location_model::ActiveModel = location.into();

    if let Some(c) = req.location_code {
        active.location_code = sea_orm::ActiveValue::Set(c);
    }
    if let Some(t) = req.location_type {
        active.location_type = sea_orm::ActiveValue::Set(Some(t));
    }
    if let Some(w) = req.max_weight {
        active.max_weight = sea_orm::ActiveValue::Set(rust_decimal::Decimal::from_f64_retain(w));
    }
    if let Some(h) = req.max_height {
        active.max_height = sea_orm::ActiveValue::Set(rust_decimal::Decimal::from_f64_retain(h));
    }
    if let Some(b) = req.is_batch_managed {
        active.is_batch_managed = sea_orm::ActiveValue::Set(Some(b));
    }
    if let Some(c) = req.is_color_managed {
        active.is_color_managed = sea_orm::ActiveValue::Set(Some(c));
    }
    active.updated_at = sea_orm::ActiveValue::Set(Utc::now());

    let updated = active.update(&*state.db).await?;
    let location_json = serde_json::to_value(updated)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        location_json,
        "库位更新成功",
    )))
}

/// 删除库位
pub async fn delete_location(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // P0 8-3 修复：delete 操作补审计日志
    // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
    crate::services::audit_log_service::AuditLogService::delete_with_audit::<
        LocationEntity,
        _,
    >(&*state.db, "warehouse_location", id, Some(auth.user_id))
    .await?;
    Ok(Json(ApiResponse::success_with_message((), "库位删除成功")))
}
