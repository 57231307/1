use crate::middleware::auth_context::AuthContext;
use crate::models::sales_analysis;
use crate::services::sales_analysis_service::{
    CreateSalesTargetInput, CustomerRankingParams, ExportParams, ProductRankingParams,
    SalesAnalysisService, SalesTargetDto, UpdateSalesTargetRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use crate::utils::xlsx_export::xlsx_response;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
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
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售统计列表", auth.user_id);

    let service = SalesAnalysisService::new(state.db.clone());
    let query_params = crate::services::sales_analysis_service::SalesStatisticQueryParams {
        statistic_type: params.statistic_type,
        period: params.period,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (statistics, _total) = service.get_statistics_list(query_params).await?;
    info!("销售统计列表查询成功，共 {} 条记录", statistics.len());

    Ok(Json(ApiResponse::success(statistics)))
}

pub async fn get_trends(
    Query(params): Query<TrendQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!(
        "用户 {} 正在查询销售趋势，周期：{}",
        auth.user_id, params.period
    );

    let service = SalesAnalysisService::new(state.db.clone());
    let trends = service.get_trends(&params.period).await?;
    info!("销售趋势查询成功，共 {} 条记录", trends.len());

    Ok(Json(ApiResponse::success(trends)))
}

pub async fn get_rankings(
    Query(params): Query<RankingQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售排名", auth.user_id);

    let service = SalesAnalysisService::new(state.db.clone());
    let rankings = service
        .get_rankings(params.period.as_deref(), params.limit.unwrap_or(10).clamp(1, 100))
        .await?;
    info!("销售排名查询成功，共 {} 条记录", rankings.len());

    Ok(Json(ApiResponse::success(rankings)))
}

pub async fn get_targets(
    Query(params): Query<TargetQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售目标", auth.user_id);

    let service = SalesAnalysisService::new(state.db.clone());
    let (targets, _total) = service
        .get_targets(
            params.page.unwrap_or(1).clamp(1, 1000),
            params.page_size.unwrap_or(10).clamp(1, 100),
        )
        .await?;
    info!("销售目标查询成功，共 {} 条记录", targets.len());

    Ok(Json(ApiResponse::success(targets)))
}

pub async fn create_target(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesTargetInput>,
) -> Result<Json<ApiResponse<sales_analysis::Model>>, AppError> {
    info!("用户 {} 正在创建销售目标", auth.user_id);

    let service = SalesAnalysisService::new(state.db.clone());
    let target = service.create_target(req, auth.user_id).await?;
    info!("销售目标创建成功，ID: {}", target.id);

    Ok(Json(ApiResponse::success(target)))
}

/// GET /api/v1/erp/sales-analysis/stats - 销售概览统计
pub async fn get_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("正在获取销售概览统计");
    let service = SalesAnalysisService::new(state.db.clone());
    let stats = service.get_overview_stats().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(stats)?)))
}

/// GET /api/v1/erp/sales-analysis/product-ranking - 产品销售排名
pub async fn get_product_ranking(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ProductRankingParams>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::sales_analysis_service::ProductRankingItem>>>,
    AppError,
> {
    info!("正在获取产品销售排名");
    let service = SalesAnalysisService::new(state.db.clone());
    let list = service.product_ranking(params).await?;
    Ok(Json(ApiResponse::success(list)))
}

/// GET /api/v1/erp/sales-analysis/customer-ranking - 客户销售排名
pub async fn get_customer_ranking(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<CustomerRankingParams>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::sales_analysis_service::CustomerRankingItem>>>,
    AppError,
> {
    info!("正在获取客户销售排名");
    let service = SalesAnalysisService::new(state.db.clone());
    let list = service.customer_ranking(params).await?;
    Ok(Json(ApiResponse::success(list)))
}

/// PUT /api/v1/erp/sales-analysis/targets/:period - 更新销售目标
pub async fn update_sales_target(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(period): Path<String>,
    Json(req): Json<UpdateSalesTargetRequest>,
) -> Result<Json<ApiResponse<SalesTargetDto>>, AppError> {
    info!("正在更新销售目标，周期：{}", period);
    let service = SalesAnalysisService::new(state.db.clone());
    let target = service.update_target(&period, req).await?;
    Ok(Json(ApiResponse::success(target)))
}

/// GET /api/v1/erp/sales-analysis/export - 导出销售分析报告
///
/// v11 批次 151 P2-A：service.export_report 已直接返回 xlsx 字节流，
/// handler 只需构造下载响应，无需 CSV→xlsx 中间转换。
pub async fn export_analysis(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ExportParams>,
) -> Result<axum::response::Response, AppError> {
    info!("正在导出销售分析报告");
    let service = SalesAnalysisService::new(state.db.clone());
    let bytes = service.export_report(params).await?;

    let filename = format!(
        "sales_analysis_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    Ok(xlsx_response(bytes, &filename))
}
