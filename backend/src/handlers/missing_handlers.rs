//! 缺失的 Handler 补充
//!
//! 临时补充缺失的 API 端点

use axum::{extract::State, Json};
use serde::Serialize;

use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 销售用户
#[derive(Serialize)]
pub struct SalesUser {
    pub id: i32,
    pub username: String,
    pub real_name: Option<String>,
}

/// 获取销售用户列表
pub async fn get_sales_users(
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<SalesUser>>>, AppError> {
    // 临时返回空列表
    Ok(Json(ApiResponse::success(vec![])))
}

/// 回收规则
#[derive(Serialize)]
pub struct RecycleRule {
    pub id: i32,
    pub name: String,
    pub days: i32,
    pub is_enabled: bool,
}

/// 获取回收规则列表
pub async fn get_recycle_rules(
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<RecycleRule>>>, AppError> {
    Ok(Json(ApiResponse::success(vec![])))
}

/// 创建回收规则
pub async fn create_recycle_rule(
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<RecycleRule>>, AppError> {
    Err(AppError::BadRequest("功能暂未实现".to_string()))
}

/// 更新回收规则
pub async fn update_recycle_rule(
    axum::extract::Path(_id): axum::extract::Path<i32>,
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<RecycleRule>>, AppError> {
    Err(AppError::BadRequest("功能暂未实现".to_string()))
}

/// 删除回收规则
pub async fn delete_recycle_rule(
    axum::extract::Path(_id): axum::extract::Path<i32>,
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    Err(AppError::BadRequest("功能暂未实现".to_string()))
}

/// MRP 历史记录
#[derive(Serialize)]
pub struct MrpHistory {
    pub id: i32,
    pub calculation_no: String,
    pub status: String,
}

/// 获取 MRP 历史列表
pub async fn get_mrp_history(
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<MrpHistory>>>, AppError> {
    Ok(Json(ApiResponse::success(vec![])))
}

/// 获取 MRP 历史详情
pub async fn get_mrp_history_detail(
    axum::extract::Path(_id): axum::extract::Path<i32>,
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<MrpHistory>>, AppError> {
    Err(AppError::BadRequest("功能暂未实现".to_string()))
}

/// 会计期间
#[derive(Serialize)]
pub struct AccountingPeriod {
    pub id: i32,
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    pub status: String,
}

/// 获取会计期间列表
pub async fn get_accounting_periods(
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<AccountingPeriod>>>, AppError> {
    Ok(Json(ApiResponse::success(vec![])))
}

/// 获取当前会计期间
pub async fn get_current_accounting_period(
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Option<AccountingPeriod>>>, AppError> {
    Ok(Json(ApiResponse::success(None)))
}

/// 获取会计期间详情
pub async fn get_accounting_period_detail(
    axum::extract::Path(_id): axum::extract::Path<i32>,
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<AccountingPeriod>>, AppError> {
    Err(AppError::BadRequest("功能暂未实现".to_string()))
}

/// 创建会计期间
pub async fn create_accounting_period(
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<AccountingPeriod>>, AppError> {
    Err(AppError::BadRequest("功能暂未实现".to_string()))
}

/// 更新会计期间
pub async fn update_accounting_period(
    axum::extract::Path(_id): axum::extract::Path<i32>,
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<AccountingPeriod>>, AppError> {
    Err(AppError::BadRequest("功能暂未实现".to_string()))
}

/// 删除会计期间
pub async fn delete_accounting_period(
    axum::extract::Path(_id): axum::extract::Path<i32>,
    _State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    Err(AppError::BadRequest("功能暂未实现".to_string()))
}
