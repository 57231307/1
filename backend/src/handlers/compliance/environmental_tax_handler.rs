//! 环保税 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::environmental_tax_service::{
    CreateDischargeRecordRequest, EnvironmentalTaxService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

/// 查询参数：申报期间
#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    pub period_year: i32,
    pub period_month: i32,
}

/// 创建污染物排放记录（自动计算环保税）
pub async fn create_discharge_record(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(mut req): Json<CreateDischargeRecordRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = EnvironmentalTaxService::new(state.db.clone());
    let model = service.create_discharge_record(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 按期间查询污染物排放记录
pub async fn list_discharge_records(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PeriodQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = EnvironmentalTaxService::new(state.db.clone());
    let list = service
        .list_by_period(params.period_year, params.period_month)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 生成环保税申报表（按期间汇总）
pub async fn generate_tax_declaration(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PeriodQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = EnvironmentalTaxService::new(state.db.clone());
    let result = service
        .generate_tax_declaration(params.period_year, params.period_month)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}
