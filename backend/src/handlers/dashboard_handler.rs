use axum::{
    extract::{Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use serde::Deserialize;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};

use crate::services::dashboard_service::DashboardService;
use crate::utils::response::ApiResponse;
use crate::utils::error::AppError;
use crate::services::dashboard_service::{DashboardOverview, SalesStatistics, InventoryStatistics, LowStockAlert};

/// 查询参数 - 仪表板
#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// 将 NaiveDate 转换为 DateTime<Utc>（一天的开始）
fn naive_date_to_utc(date: NaiveDate) -> DateTime<Utc> {
    Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
}

/// 获取仪表板概览数据
pub async fn get_dashboard_overview(
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<ApiResponse<DashboardOverview>>, AppError> {
    let dashboard_service = DashboardService::new(db.clone());
    let start_datetime = query.start_date.map(naive_date_to_utc);
    let end_datetime = query.end_date.map(naive_date_to_utc);
    let overview = dashboard_service.get_overview(start_datetime, end_datetime).await?;
    Ok(Json(ApiResponse::success(overview)))
}

/// 获取销售统计数据
pub async fn get_sales_statistics(
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<ApiResponse<SalesStatistics>>, AppError> {
    let dashboard_service = DashboardService::new(db.clone());
    let start_datetime = query.start_date.map(naive_date_to_utc);
    let end_datetime = query.end_date.map(naive_date_to_utc);
    let stats = dashboard_service.get_sales_statistics(start_datetime, end_datetime).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// 获取库存统计数据
pub async fn get_inventory_statistics(
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<ApiResponse<InventoryStatistics>>, AppError> {
    let dashboard_service = DashboardService::new(db.clone());
    let start_datetime = query.start_date.map(naive_date_to_utc);
    let end_datetime = query.end_date.map(naive_date_to_utc);
    let stats = dashboard_service.get_inventory_statistics(start_datetime, end_datetime).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// 获取低库存预警数据
pub async fn get_low_stock_alerts(
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<LowStockAlert>>>, AppError> {
    let dashboard_service = DashboardService::new(db.clone());
    let alerts = dashboard_service.get_low_stock_alerts().await?;
    Ok(Json(ApiResponse::success(alerts)))
}
