use axum::{
    extract::{Path, State},
    Json,
};
use crate::middleware::auth_context::AuthContext;
use crate::services::accounting_period_service::AccountingPeriodService;
use crate::models::accounting_period;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use chrono::Datelike;

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
    Ok(Json(ApiResponse::success_with_msg(period, "财务期间初始化成功")))
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
    Ok(Json(ApiResponse::success_with_msg(period, "月末结账成功，已自动开启下一期间")))
}
