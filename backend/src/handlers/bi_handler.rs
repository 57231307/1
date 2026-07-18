//! P3-4 BI 多维分析 Handler
//!
//! 16 个 HTTP 端点：
//! - 8 个维度聚合：by-time / by-customer / by-product / by-region / by-category / trend / profit / kpi
//! - 4 个钻取：year-to-month / month-to-day / customer-to-order / product-to-order
//! - 4 个切片/上卷：slice / dice / rollup / pivot
//!
//! v9 批次 130 修复：所有 handler 从静态方法调用改为实例方法调用，
//! 通过 `state.db.clone()` 注入数据库连接到 BiAnalysisService。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::bi_analysis_service::{
    BiAnalysisService, BiResponse, CategoryStat, CustomerRank, KpiSummary, ProductRank,
    ProfitAnalysis, RegionStat, TimeSeriesPoint,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// =====================================================
// 8 个维度聚合端点
// =====================================================

/// GET /api/v1/erp/bi/sales/by-time
/// 按时间聚合销售
#[derive(Debug, Deserialize)]
pub struct ByTimeQuery {
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub granularity: String,  // day/week/month/quarter/year
}

pub async fn sales_by_time(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<ByTimeQuery>,
) -> Result<Json<ApiResponse<BiResponse<Vec<TimeSeriesPoint>>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service
        .sales_by_time(q.start_date, q.end_date, &q.granularity)
        .await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/by-customer
/// 按客户聚合
#[derive(Debug, Deserialize)]
pub struct ByCustomerQuery {
    pub limit: Option<i64>,
}

pub async fn sales_by_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<ByCustomerQuery>,
) -> Result<Json<ApiResponse<BiResponse<Vec<CustomerRank>>>>, AppError> {
    let limit = q.limit.unwrap_or(10).clamp(1, 100);
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.sales_by_customer(limit).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/by-product
/// 按产品聚合
pub async fn sales_by_product(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<ByCustomerQuery>,
) -> Result<Json<ApiResponse<BiResponse<Vec<ProductRank>>>>, AppError> {
    let limit = q.limit.unwrap_or(10).clamp(1, 100);
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.sales_by_product(limit).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/by-region
/// 按区域聚合
pub async fn sales_by_region(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<BiResponse<Vec<RegionStat>>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.sales_by_region().await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/by-category
/// 按品类聚合
pub async fn sales_by_category(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<BiResponse<Vec<CategoryStat>>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.sales_by_category().await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/trend
/// 销售趋势（时间序列）
#[derive(Debug, Deserialize)]
pub struct TrendQuery {
    pub days: Option<i32>,
}

pub async fn sales_trend(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<TrendQuery>,
) -> Result<Json<ApiResponse<BiResponse<Vec<TimeSeriesPoint>>>>, AppError> {
    let days = q.days.unwrap_or(30);
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.sales_trend(days).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/profit
/// 利润分析
pub async fn profit_analysis(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<BiResponse<ProfitAnalysis>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.profit_analysis().await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/kpi
/// 核心 KPI
pub async fn kpi_summary(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<BiResponse<KpiSummary>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.kpi_summary().await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

// =====================================================
// 4 个钻取端点
// =====================================================

/// GET /api/v1/erp/bi/sales/drilldown/year-to-month
/// 钻取：年 → 月
#[derive(Debug, Deserialize)]
pub struct DrillYearMonthQuery {
    pub year: i32,
}

pub async fn drilldown_year_to_month(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<DrillYearMonthQuery>,
) -> Result<Json<ApiResponse<BiResponse<Vec<TimeSeriesPoint>>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.drilldown_year_to_month(q.year).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/drilldown/month-to-day
/// 钻取：月 → 日
#[derive(Debug, Deserialize)]
pub struct DrillMonthDayQuery {
    pub year: i32,
    pub month: u32,
}

pub async fn drilldown_month_to_day(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<DrillMonthDayQuery>,
) -> Result<Json<ApiResponse<BiResponse<Vec<TimeSeriesPoint>>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.drilldown_month_to_day(q.year, q.month).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/drilldown/customer-to-order/:customer_id
/// 钻取：客户 → 订单
pub async fn drilldown_customer_to_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(customer_id): Path<i64>,
) -> Result<Json<ApiResponse<BiResponse<serde_json::Value>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.drilldown_customer_to_order(customer_id).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// GET /api/v1/erp/bi/sales/drilldown/product-to-order/:product_id
/// 钻取：产品 → 订单
pub async fn drilldown_product_to_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(product_id): Path<i64>,
) -> Result<Json<ApiResponse<BiResponse<serde_json::Value>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.drilldown_product_to_order(product_id).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

// =====================================================
// 4 个切片/上卷端点
// =====================================================

/// POST /api/v1/erp/bi/sales/slice
/// 切片（固定其他维度）
#[derive(Debug, Deserialize)]
pub struct SliceRequest {
    pub dimension: String,
    pub filters: serde_json::Value,
}

pub async fn slice(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<SliceRequest>,
) -> Result<Json<ApiResponse<BiResponse<serde_json::Value>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.slice(&body.dimension, &body.filters).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// POST /api/v1/erp/bi/sales/dice
/// 切块（多维范围筛选）
#[derive(Debug, Deserialize)]
pub struct DiceRequest {
    pub filters: serde_json::Value,
}

pub async fn dice(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<DiceRequest>,
) -> Result<Json<ApiResponse<BiResponse<serde_json::Value>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.dice(&body.filters).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// POST /api/v1/erp/bi/sales/rollup
/// 上卷（细粒度 → 粗粒度）
#[derive(Debug, Deserialize)]
pub struct RollupRequest {
    pub from: String,
    pub to: String,
}

pub async fn rollup(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<RollupRequest>,
) -> Result<Json<ApiResponse<BiResponse<serde_json::Value>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.rollup(&body.from, &body.to).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}

/// POST /api/v1/erp/bi/sales/pivot
/// 透视（行列转换）
#[derive(Debug, Deserialize)]
pub struct PivotRequest {
    pub row: String,
    pub col: String,
    pub measure: String,
}

pub async fn pivot(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<PivotRequest>,
) -> Result<Json<ApiResponse<BiResponse<serde_json::Value>>>, AppError> {
    let service = BiAnalysisService::new_with_data_scope(state.db.clone(), auth.to_data_scope_context());
    let data = service.pivot(&body.row, &body.col, &body.measure).await?;
    Ok(Json(ApiResponse::success(BiResponse::success(data))))
}
