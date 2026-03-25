use crate::middleware::auth_context::AuthContext;
use crate::models::sales_analysis;
use crate::services::sales_analysis_service::{CreateSalesTargetInput, SalesAnalysisService};
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
pub struct SalesStatisticQuery {
    pub statistic_type: Option<String>,
    pub period: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TrendQuery {
    pub period: String,
}

#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    pub period: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TargetQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list_statistics(
    Query(params): Query<SalesStatisticQuery>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售统计列表", auth.user_id);

    let service = SalesAnalysisService::new(db);
    let query_params = crate::services::sales_analysis_service::SalesStatisticQueryParams {
        statistic_type: params.statistic_type,
        period: params.period,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (statistics, _total) = service.get_statistics_list(query_params).await?;
    info!("销售统计列表查询成功，共 {} 条记录", statistics.len());

    Ok(Json(ApiResponse::success(statistics)))
}

pub async fn get_trends(
    Query(params): Query<TrendQuery>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!(
        "用户 {} 正在查询销售趋势，周期：{}",
        auth.user_id, params.period
    );

    let service = SalesAnalysisService::new(db);
    let trends = service.get_trends(&params.period).await?;
    info!("销售趋势查询成功，共 {} 条记录", trends.len());

    Ok(Json(ApiResponse::success(trends)))
}

pub async fn get_rankings(
    Query(params): Query<RankingQuery>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售排名", auth.user_id);

    let service = SalesAnalysisService::new(db);
    let rankings = service
        .get_rankings(params.period.as_deref(), params.limit.unwrap_or(10))
        .await?;
    info!("销售排名查询成功，共 {} 条记录", rankings.len());

    Ok(Json(ApiResponse::success(rankings)))
}

pub async fn get_targets(
    Query(params): Query<TargetQuery>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售目标", auth.user_id);

    let service = SalesAnalysisService::new(db);
    let (targets, _total) = service
        .get_targets(params.page.unwrap_or(0), params.page_size.unwrap_or(10))
        .await?;
    info!("销售目标查询成功，共 {} 条记录", targets.len());

    Ok(Json(ApiResponse::success(targets)))
}

pub async fn create_target(
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<CreateSalesTargetInput>,
) -> Result<Json<ApiResponse<sales_analysis::Model>>, AppError> {
    info!("用户 {} 正在创建销售目标", auth.user_id);

    let service = SalesAnalysisService::new(db);
    let target = service.create_target(req, auth.user_id).await?;
    info!("销售目标创建成功，ID: {}", target.id);

    Ok(Json(ApiResponse::success(target)))
}
