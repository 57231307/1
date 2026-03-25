use crate::middleware::auth_context::AuthContext;
use crate::models::financial_analysis;
use crate::models::financial_analysis_result;
use crate::services::financial_analysis_service::{
    CreateIndicatorRequest, FinancialAnalysisRequest, FinancialAnalysisService,
};
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct IndicatorQuery {
    pub indicator_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TrendQuery {
    pub indicator_id: Option<i32>,
    pub limit: Option<i64>,
}

pub async fn list_indicators(
    Query(params): Query<IndicatorQuery>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<financial_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询财务指标列表", auth.user_id);

    let service = FinancialAnalysisService::new(db);
    let query_params = crate::services::financial_analysis_service::IndicatorQueryParams {
        indicator_type: params.indicator_type,
        status: params.status,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (indicators, _total) = service.get_indicators_list(query_params).await?;
    info!("财务指标列表查询成功，共 {} 条记录", indicators.len());

    Ok(Json(ApiResponse::success(indicators)))
}

pub async fn create_indicator(
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<CreateIndicatorRequest>,
) -> Result<Json<ApiResponse<financial_analysis::Model>>, AppError> {
    info!(
        "用户 {} 正在创建财务指标：{}",
        auth.user_id, req.indicator_code
    );

    let service = FinancialAnalysisService::new(db);
    let indicator = service.create_indicator(req, auth.user_id).await?;
    info!("财务指标创建成功：{}", indicator.indicator_code);

    Ok(Json(ApiResponse::success(indicator)))
}

pub async fn create_analysis_result(
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<FinancialAnalysisRequest>,
) -> Result<Json<ApiResponse<financial_analysis_result::Model>>, AppError> {
    info!("用户 {} 正在创建财务分析结果", auth.user_id);

    let service = FinancialAnalysisService::new(db);
    let result = service.create_analysis_result(req, auth.user_id).await?;
    info!("财务分析结果创建成功");

    Ok(Json(ApiResponse::success(result)))
}

pub async fn get_trends(
    Query(params): Query<TrendQuery>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<financial_analysis_result::Model>>>, AppError> {
    info!("用户 {} 正在查询财务趋势", auth.user_id);

    let service = FinancialAnalysisService::new(db);
    let trends = service
        .get_trends(params.indicator_id.unwrap_or(0), params.limit.unwrap_or(10))
        .await?;
    info!("财务趋势查询成功，共 {} 条记录", trends.len());

    Ok(Json(ApiResponse::success(trends)))
}
