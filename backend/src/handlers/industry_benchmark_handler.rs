use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::industry_benchmark_config::{
    self, CreateIndustryBenchmarkDto, UpdateIndustryBenchmarkDto,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// GET /api/v1/erp/industry-benchmarks - 获取行业基准列表
pub async fn list_industry_benchmarks(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<industry_benchmark_config::Model>>>, AppError> {
    let benchmarks = industry_benchmark_config::Entity::find()
        .filter(industry_benchmark_config::Column::IsActive.eq(true))
        .order_by_asc(industry_benchmark_config::Column::IndustryType)
        .order_by_asc(industry_benchmark_config::Column::MetricName)
        .all(&*state.db)
        .await?;
    Ok(Json(ApiResponse::success(benchmarks)))
}

/// POST /api/v1/erp/industry-benchmarks - 创建行业基准
pub async fn create_industry_benchmark(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(dto): Json<CreateIndustryBenchmarkDto>,
) -> Result<Json<ApiResponse<industry_benchmark_config::Model>>, AppError> {
    let now = chrono::Utc::now();
    let benchmark = industry_benchmark_config::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        benchmark_name: Set(dto.benchmark_name),
        industry_type: Set(dto.industry_type),
        metric_name: Set(dto.metric_name),
        metric_value: Set(dto.metric_value),
        unit: Set(dto.unit),
        data_source: Set(dto.data_source),
        data_year: Set(dto.data_year),
        is_active: Set(dto.is_active.unwrap_or(true)),
        remark: Set(dto.remark),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let inserted = benchmark.insert(&*state.db).await?;
    Ok(Json(ApiResponse::success(inserted)))
}

/// PUT /api/v1/erp/industry-benchmarks/:id - 更新行业基准
pub async fn update_industry_benchmark(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    _auth: AuthContext,
    Json(dto): Json<UpdateIndustryBenchmarkDto>,
) -> Result<Json<ApiResponse<industry_benchmark_config::Model>>, AppError> {
    let existing = industry_benchmark_config::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("行业基准不存在"))?;

    let mut active: industry_benchmark_config::ActiveModel = existing.into();
    if let Some(v) = dto.benchmark_name {
        active.benchmark_name = Set(v);
    }
    if let Some(v) = dto.industry_type {
        active.industry_type = Set(v);
    }
    if let Some(v) = dto.metric_name {
        active.metric_name = Set(v);
    }
    if let Some(v) = dto.metric_value {
        active.metric_value = Set(v);
    }
    if let Some(v) = dto.unit {
        active.unit = Set(Some(v));
    }
    if let Some(v) = dto.data_source {
        active.data_source = Set(Some(v));
    }
    if let Some(v) = dto.data_year {
        active.data_year = Set(Some(v));
    }
    if let Some(v) = dto.is_active {
        active.is_active = Set(v);
    }
    if let Some(v) = dto.remark {
        active.remark = Set(Some(v));
    }
    active.updated_at = Set(chrono::Utc::now());
    let updated = active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// DELETE /api/v1/erp/industry-benchmarks/:id - 删除行业基准
pub async fn delete_industry_benchmark(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let existing = industry_benchmark_config::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("行业基准不存在"))?;

    let mut active: industry_benchmark_config::ActiveModel = existing.into();
    active.delete(&*state.db).await?;
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}
