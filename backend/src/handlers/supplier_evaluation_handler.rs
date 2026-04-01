use crate::middleware::auth_context::AuthContext;
use crate::models::supplier_evaluation;
use crate::models::supplier_evaluation_record;
use crate::services::supplier_evaluation_service::{
    CreateEvaluationIndicatorRequest, SupplierEvaluationService, SupplierScoreResponse,
};
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use crate::utils::app_state::AppState;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct EvaluationIndicatorQuery {
    pub category: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list_indicators(
    Query(params): Query<EvaluationIndicatorQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<supplier_evaluation::Model>>>, AppError> {
    info!("用户 {} 正在查询供应商评估指标列表", auth.user_id);

    let service = SupplierEvaluationService::new(db);
    let query_params =
        crate::services::supplier_evaluation_service::EvaluationIndicatorQueryParams {
            category: params.category,
            status: params.status,
            page: params.page.unwrap_or(0),
            page_size: params.page_size.unwrap_or(10),
        };

    let (indicators, _total) = service.get_indicators_list(query_params).await?;
    info!("供应商评估指标列表查询成功，共 {} 条记录", indicators.len());

    Ok(Json(ApiResponse::success(indicators)))
}

pub async fn create_indicator(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateEvaluationIndicatorRequest>,
) -> Result<Json<ApiResponse<supplier_evaluation::Model>>, AppError> {
    info!(
        "用户 {} 正在创建评估指标：{}",
        auth.user_id, req.indicator_code
    );

    let service = SupplierEvaluationService::new(db);
    let indicator = service.create_indicator(req, auth.user_id).await?;
    info!("评估指标创建成功：{}", indicator.indicator_code);

    Ok(Json(ApiResponse::success(indicator)))
}

pub async fn create_evaluation_record(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<crate::services::supplier_evaluation_service::SupplierEvaluationRequest>,
) -> Result<Json<ApiResponse<supplier_evaluation_record::Model>>, AppError> {
    info!(
        "用户 {} 正在创建评估记录，供应商：{}",
        auth.user_id, req.supplier_id
    );

    let service = SupplierEvaluationService::new(db);
    let record = service.create_evaluation_record(req, auth.user_id).await?;
    info!("评估记录创建成功");

    Ok(Json(ApiResponse::success(record)))
}

pub async fn get_supplier_score(
    State(state): State<AppState>,
    Path(supplier_id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<SupplierScoreResponse>>, AppError> {
    info!(
        "用户 {} 正在查询供应商 {} 的评分",
        auth.user_id, supplier_id
    );

    let service = SupplierEvaluationService::new(db);
    let score = service.get_supplier_score(supplier_id).await?;
    info!("供应商评分查询成功");

    Ok(Json(ApiResponse::success(score)))
}

pub async fn list_ratings(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<supplier_evaluation::Model>>>, AppError> {
    info!("用户 {} 正在查询供应商评级列表", auth.user_id);

    let service = SupplierEvaluationService::new(db);
    let ratings = service.list_ratings().await?;
    info!("供应商评级列表查询成功，共 {} 条记录", ratings.len());

    Ok(Json(ApiResponse::success(ratings)))
}

#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    pub limit: Option<i64>,
}

pub async fn get_rankings(
    Query(params): Query<RankingQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<
    Json<ApiResponse<Vec<crate::services::supplier_evaluation_service::SupplierScoreResponse>>>,
    AppError,
> {
    info!("用户 {} 正在查询供应商排名榜", auth.user_id);

    let service = SupplierEvaluationService::new(db);
    let rankings = service
        .get_supplier_rankings(params.limit.unwrap_or(10))
        .await?;
    info!("供应商排名榜查询成功，共 {} 条记录", rankings.len());

    Ok(Json(ApiResponse::success(rankings)))
}

#[derive(Debug, Deserialize)]
pub struct EvaluationRecordQuery {
    pub supplier_id: Option<i32>,
    pub period: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list_evaluation_records(
    Query(params): Query<EvaluationRecordQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<supplier_evaluation_record::Model>>>, AppError> {
    info!("用户 {} 正在查询评估记录列表", auth.user_id);

    let service = SupplierEvaluationService::new(db);
    let records = service
        .get_evaluation_records(
            params.supplier_id,
            params.period,
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
        )
        .await?;
    info!("评估记录列表查询成功，共 {} 条记录", records.len());

    Ok(Json(ApiResponse::success(records)))
}

pub async fn get_evaluation_record(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<supplier_evaluation_record::Model>>, AppError> {
    info!("用户 {} 正在查询评估记录：{}", auth.user_id, id);

    let service = SupplierEvaluationService::new(db);
    let record = service.get_evaluation_record_by_id(id).await?;
    info!("评估记录查询成功：{}", id);

    Ok(Json(ApiResponse::success(record)))
}
