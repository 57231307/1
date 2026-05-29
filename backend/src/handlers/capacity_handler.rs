//! 产能分析 Handler
//!
//! 提供产能概览、工作中心管理、负荷分析等 API 接口

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::capacity_service::{
    CapacityService, CreateWorkCenterInput, LoadAnalysisQuery, UpdateWorkCenterInput,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 负荷分析查询参数（Handler 层）
#[derive(Debug, Deserialize)]
pub struct LoadAnalysisParams {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub work_center_id: Option<i32>,
}

/// GET /api/v1/erp/capacity/overview - 产能概览
pub async fn get_capacity_overview(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());
    let overview = service.overview().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(overview)?)))
}

/// GET /api/v1/erp/capacity/work-centers - 工作中心列表
pub async fn list_work_centers(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());
    let work_centers = service.list_work_centers().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        work_centers,
    )?)))
}

/// GET /api/v1/erp/capacity/load-analysis - 负荷分析
pub async fn get_load_analysis(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<LoadAnalysisParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());

    let query = LoadAnalysisQuery {
        date_from: params
            .date_from
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        date_to: params
            .date_to
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        work_center_id: params.work_center_id,
    };

    let items = service.load_analysis(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(items)?)))
}

/// POST /api/v1/erp/capacity/work-centers - 创建工作中心
pub async fn create_work_center(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(input): Json<CreateWorkCenterInput>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());
    let work_center = service.create_work_center(input).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        work_center,
    )?)))
}

/// PUT /api/v1/erp/capacity/work-centers/:id - 更新工作中心
pub async fn update_work_center(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(input): Json<UpdateWorkCenterInput>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());
    let work_center = service.update_work_center(id, input).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        work_center,
    )?)))
}

/// DELETE /api/v1/erp/capacity/work-centers/:id - 删除工作中心
pub async fn delete_work_center(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());
    service.delete_work_center(id).await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({"deleted": true}),
    )))
}

/// 产能预测查询参数
#[derive(Debug, Deserialize)]
pub struct CapacityForecastQuery {
    pub days: Option<i32>,
}

/// GET /api/v1/erp/capacity/work-centers/:id/forecast - 产能预测
pub async fn forecast_capacity(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Query(query): Query<CapacityForecastQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());
    let days = query.days.unwrap_or(30);
    let forecast = service.forecast_capacity(id, days).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(forecast)?)))
}

/// 可用产能查询参数
#[derive(Debug, Deserialize)]
pub struct AvailableCapacityQuery {
    pub date_from: String,
    pub date_to: String,
}

/// GET /api/v1/erp/capacity/work-centers/:id/available - 获取可用产能
pub async fn get_available_capacity(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Query(query): Query<AvailableCapacityQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());
    let date_from = chrono::NaiveDate::parse_from_str(&query.date_from, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("date_from 格式无效，应为 YYYY-MM-DD".to_string()))?;
    let date_to = chrono::NaiveDate::parse_from_str(&query.date_to, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("date_to 格式无效，应为 YYYY-MM-DD".to_string()))?;
    let available = service
        .get_available_capacity(id, date_from, date_to)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(available)?)))
}

/// 产能负荷预警查询参数
#[derive(Debug, Deserialize)]
pub struct OverloadCheckQuery {
    pub threshold: Option<rust_decimal::Decimal>,
}

/// GET /api/v1/erp/capacity/overload-check - 检查产能负荷预警
pub async fn check_capacity_overload(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<OverloadCheckQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CapacityService::new(state.db.clone());
    let threshold = query.threshold.unwrap_or(rust_decimal::Decimal::from(90));
    let alerts = service.check_capacity_overload(threshold).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(alerts)?)))
}
