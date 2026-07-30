//! 社保公积金 handler（V15 P1 batch-08 缺陷 23）
//!
//! 社保公积金扣缴 HTTP 接口：
//! - 按月计算五险一金扣缴金额（单位/个人部分）
//! - 校验缴费基数合规性（不低于当地最低基数、不高于当地最高基数）
//! - 状态机：pending(待缴) → paid(已缴) / cancelled(已撤销)
//!
//! 依据：《社会保险法》第58条 + 《住房公积金管理条例》第14条

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::social_insurance_service::{
    CreateSocialInsuranceRequest, SocialInsuranceQuery, SocialInsuranceService,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;

/// 按工人查询期间社保记录的查询参数
#[derive(Debug, Deserialize)]
pub struct WorkerPeriodQuery {
    pub period_year: i32,
    pub period_month: i32,
}

/// 确认缴纳请求体
#[derive(Debug, Deserialize)]
pub struct MarkPaidRequest {
    pub payment_date: NaiveDate,
}

/// 创建社保缴纳记录（自动计算五险一金）
/// POST /social-insurance
pub async fn create(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(mut req): Json<CreateSocialInsuranceRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = SocialInsuranceService::new(state.db.clone());
    let model = service.create(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 获取社保记录详情
/// GET /social-insurance/:id
pub async fn get_by_id(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SocialInsuranceService::new(state.db.clone());
    let model = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询社保记录列表（分页）
/// GET /social-insurance
pub async fn list(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<SocialInsuranceQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 200);
    let service = SocialInsuranceService::new(state.db.clone());
    let (items, total) = service.list(params).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        PaginatedResponse::new(items, total, page, page_size),
    )?)))
}

/// 按工人查询期间社保记录
/// GET /social-insurance/by-worker/:worker_id
pub async fn get_by_worker_period(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(worker_id): Path<i32>,
    Query(params): Query<WorkerPeriodQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SocialInsuranceService::new(state.db.clone());
    let model = service
        .get_by_worker_period(worker_id, params.period_year, params.period_month)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 确认缴纳（pending → paid）
/// POST /social-insurance/:id/mark-paid
pub async fn mark_paid(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<MarkPaidRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SocialInsuranceService::new(state.db.clone());
    let model = service.mark_paid(id, req.payment_date).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 撤销社保记录（仅 pending 状态可撤销）
/// POST /social-insurance/:id/cancel
pub async fn cancel(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SocialInsuranceService::new(state.db.clone());
    let model = service.cancel(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}
