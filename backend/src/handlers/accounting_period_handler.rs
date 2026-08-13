use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::accounting_period;
use crate::services::accounting_period_service::AccountingPeriodService;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Datelike;
use serde::Deserialize;

/// 获取当前开放的财务期间
pub async fn get_current_period(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Option<accounting_period::Model>>>, AppError> {
    let service = AccountingPeriodService::new(state.db.clone());
    let period = service.get_current_period().await?;
    Ok(Json(ApiResponse::success(period)))
}

/// 初始化当前期间 (如果不存在)
pub async fn init_period(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<accounting_period::Model>>, AppError> {
    let service = AccountingPeriodService::new(state.db.clone());
    let now = chrono::Utc::now();
    let period = service.init_first_period(now.year(), now.month()).await?;
    Ok(Json(ApiResponse::success_with_message(
        period,
        "财务期间初始化成功",
    )))
}

/// 执行月末结账
pub async fn close_period(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<accounting_period::Model>>, AppError> {
    let service = AccountingPeriodService::new(state.db.clone());
    let user_id = auth.user_id;
    let period = service.close_period(id, user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        period,
        "月末结账成功，已自动开启下一期间",
    )))
}

/// 反结账请求
#[derive(Debug, Deserialize)]
pub struct ReopenPeriodRequest {
    pub reason: String,
}

/// 反结账（重开期间）
pub async fn reopen_period(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<ReopenPeriodRequest>,
) -> Result<Json<ApiResponse<accounting_period::Model>>, AppError> {
    let service = AccountingPeriodService::new(state.db.clone());
    let period = service.reopen_period(id, auth.user_id, &req.reason).await?;
    Ok(Json(ApiResponse::success_with_message(
        period,
        "反结账成功，期间已重新打开",
    )))
}

/// 年结查询参数
#[derive(Debug, Deserialize)]
pub struct YearEndClosingQuery {
    pub year: i32,
}

/// 年度结账
pub async fn year_end_closing(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<YearEndClosingQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AccountingPeriodService::new(state.db.clone());
    let result = service.year_end_closing(query.year, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        result,
        "年度结账成功",
    )))
}
