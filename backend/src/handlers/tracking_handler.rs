//! 用户行为追踪 HTTP 端点
//!
//! v11 批次 143 P1-2：真实实现追踪分析功能
//!
//! 提供端点：
//! - `POST /api/v1/erp/analytics/tracking/page-view` — 记录页面访问（持久化）
//! - `GET /api/v1/erp/analytics/tracking/page-view/stats` — 页面访问统计（总量）
//! - `GET /api/v1/erp/analytics/tracking/page-view/stats/by-day` — 按日统计
//! - `GET /api/v1/erp/analytics/tracking/popular-pages` — 热门页面排行
//! - `POST /api/v1/erp/analytics/tracking/behavior` — 记录用户行为
//! - `POST /api/v1/erp/analytics/tracking/funnel` — 漏斗分析
//! - `GET /api/v1/erp/analytics/tracking/user-path` — 用户路径分析

use crate::middleware::auth_context::AuthContext;
use crate::services::tracking_service::{
    BehaviorInput, FunnelQuery, PageViewInput, StatsQuery, TrackingService, UserPathQuery,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

/// 页面访问记录请求体
#[derive(Debug, Deserialize)]
pub struct PageViewRequest {
    pub path: String,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

/// 页面访问记录响应
#[derive(Debug, Serialize)]
pub struct PageViewResponse {
    pub success: bool,
}

/// 用户行为记录请求体
#[derive(Debug, Deserialize)]
pub struct BehaviorRequest {
    pub event_type: String,
    pub event_target: Option<String>,
    pub event_data: Option<serde_json::Value>,
    pub path: Option<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
}

/// 漏斗分析请求体
#[derive(Debug, Deserialize)]
pub struct FunnelRequest {
    pub steps: Vec<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

/// 记录页面访问埋点（持久化到 page_views 表）
pub async fn track_page_view(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<PageViewRequest>,
) -> Result<Json<ApiResponse<PageViewResponse>>, AppError> {
    let service = TrackingService::new(state.db.clone());
    let input = PageViewInput {
        path: req.path,
        timestamp: req.timestamp,
        session_id: req.session_id,
        user_id: Some(auth.user_id),
        referrer: req.referrer,
        user_agent: req.user_agent,
        ip_address: req.ip_address,
    };
    service.record_page_view(input).await?;
    Ok(Json(ApiResponse::success(PageViewResponse { success: true })))
}

/// 页面访问统计（总量）
pub async fn get_page_view_stats(
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<ApiResponse<crate::services::tracking_service::PageViewStats>>, AppError> {
    let service = TrackingService::new(state.db.clone());
    let date_from = parse_date_param(&params.date_from)?;
    let date_to = parse_date_param(&params.date_to)?;
    let stats = service.get_page_view_stats(date_from, date_to).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// 按日统计
pub async fn get_page_view_stats_by_day(
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::tracking_service::DailyStats>>>,
    AppError,
> {
    let service = TrackingService::new(state.db.clone());
    let date_from = parse_date_param(&params.date_from)?;
    let date_to = parse_date_param(&params.date_to)?;
    let stats = service.get_daily_stats(date_from, date_to).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// 热门页面排行
#[derive(Debug, Deserialize)]
pub struct PopularPagesQuery {
    pub limit: Option<u64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

pub async fn get_popular_pages(
    State(state): State<AppState>,
    Query(params): Query<PopularPagesQuery>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::tracking_service::PopularPage>>>,
    AppError,
> {
    let service = TrackingService::new(state.db.clone());
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let date_from = parse_date_param(&params.date_from)?;
    let date_to = parse_date_param(&params.date_to)?;
    let pages = service
        .get_popular_pages(limit, date_from, date_to)
        .await?;
    Ok(Json(ApiResponse::success(pages)))
}

/// 记录用户行为
pub async fn record_behavior(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<BehaviorRequest>,
) -> Result<Json<ApiResponse<PageViewResponse>>, AppError> {
    let service = TrackingService::new(state.db.clone());
    let input = BehaviorInput {
        event_type: req.event_type,
        event_target: req.event_target,
        event_data: req.event_data,
        path: req.path,
        session_id: req.session_id,
        user_id: Some(auth.user_id),
        ip_address: req.ip_address,
    };
    service.record_behavior(input).await?;
    Ok(Json(ApiResponse::success(PageViewResponse { success: true })))
}

/// 漏斗分析
pub async fn get_funnel_analysis(
    State(state): State<AppState>,
    Json(req): Json<FunnelRequest>,
) -> Result<Json<ApiResponse<crate::services::tracking_service::FunnelAnalysis>>, AppError> {
    let service = TrackingService::new(state.db.clone());
    let query = FunnelQuery {
        steps: req.steps,
        date_from: req.date_from,
        date_to: req.date_to,
    };
    let analysis = service.get_funnel_analysis(query).await?;
    Ok(Json(ApiResponse::success(analysis)))
}

/// 用户路径分析
pub async fn get_user_path(
    State(state): State<AppState>,
    Query(params): Query<UserPathQuery>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::tracking_service::UserPathNode>>>,
    AppError,
> {
    let service = TrackingService::new(state.db.clone());
    let date_from = parse_date_param(&params.date_from)?;
    let date_to = parse_date_param(&params.date_to)?;
    let path = service
        .get_user_path(&params.session_id, date_from, date_to)
        .await?;
    Ok(Json(ApiResponse::success(path)))
}

/// 解析日期参数
fn parse_date_param(s: &Option<String>) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    match s {
        Some(s) => {
            let dt = s
                .parse::<chrono::DateTime<chrono::Utc>>()
                .or_else(|_| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
                })
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?;
            Ok(Some(dt))
        }
        None => Ok(None),
    }
}
