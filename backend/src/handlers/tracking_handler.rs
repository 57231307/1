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
use validator::Validate;

/// 页面访问记录请求体
///
/// v14 中风险安全修复：添加输入长度约束，防止超大 path/字段触发 DoS
#[derive(Debug, Deserialize, Validate)]
pub struct PageViewRequest {
    #[validate(length(max = 2048, message = "path 长度不能超过 2048 个字符"))]
    pub path: String,
    #[validate(length(max = 64, message = "timestamp 长度不能超过 64 个字符"))]
    pub timestamp: String,
    #[validate(length(max = 128, message = "session_id 长度不能超过 128 个字符"))]
    pub session_id: Option<String>,
    #[validate(length(max = 2048, message = "referrer 长度不能超过 2048 个字符"))]
    pub referrer: Option<String>,
    #[validate(length(max = 512, message = "user_agent 长度不能超过 512 个字符"))]
    pub user_agent: Option<String>,
    #[validate(length(max = 64, message = "ip_address 长度不能超过 64 个字符"))]
    pub ip_address: Option<String>,
}

/// 页面访问记录响应
#[derive(Debug, Serialize)]
pub struct PageViewResponse {
    pub success: bool,
}

/// 用户行为记录请求体
///
/// v14 中风险安全修复：添加输入长度约束，防止超大 event_type/event_data 触发 DoS
#[derive(Debug, Deserialize, Validate)]
pub struct BehaviorRequest {
    #[validate(length(max = 128, message = "event_type 长度不能超过 128 个字符"))]
    pub event_type: String,
    #[validate(length(max = 2048, message = "event_target 长度不能超过 2048 个字符"))]
    pub event_target: Option<String>,
    /// event_data 限制为 JSON 值，validator 无法直接约束嵌套深度，
    /// 通过 service 层序列化后检查字节数（见 tracking_service::record_behavior）
    pub event_data: Option<serde_json::Value>,
    #[validate(length(max = 2048, message = "path 长度不能超过 2048 个字符"))]
    pub path: Option<String>,
    #[validate(length(max = 128, message = "session_id 长度不能超过 128 个字符"))]
    pub session_id: Option<String>,
    #[validate(length(max = 64, message = "ip_address 长度不能超过 64 个字符"))]
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
    // v14 中风险安全修复：输入长度校验，防止超大字段触发 DoS
    req.validate()
        .map_err(|e| AppError::validation(format!("参数校验失败: {}", e)))?;
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
    // v14 中风险安全修复：输入长度校验，防止超大字段触发 DoS
    req.validate()
        .map_err(|e| AppError::validation(format!("参数校验失败: {}", e)))?;
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
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap(/* 不变量：0,0,0 永远合法 */).and_utc())
                })
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?;
            Ok(Some(dt))
        }
        None => Ok(None),
    }
}
