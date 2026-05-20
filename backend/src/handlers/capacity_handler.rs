//! 产能分析 Handler
//!
//! 提供产能概览、工作中心列表、负荷分析等 API 接口

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::capacity_service::{CapacityService, LoadAnalysisQuery};
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
    Ok(Json(ApiResponse::success(serde_json::to_value(work_centers)?)))
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
