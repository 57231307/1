//! 供应商黑名单管理 Handler
//!
//! 端点：
//! - GET    /supplier-blacklists           分页查询黑名单记录
//! - POST   /supplier-blacklists           将供应商加入黑名单
//! - POST   /supplier-blacklists/{id}/release  解除黑名单

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::Value as JsonValue;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::supplier_blacklist_service::{
    BlacklistQueryParams, CreateBlacklistRequest, SupplierBlacklistService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 分页查询供应商黑名单列表
pub async fn list_blacklists(
    Query(params): Query<BlacklistQueryParams>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierBlacklistService::new(state.db.clone());
    let result = service.list_blacklists(params).await?;

    let value = serde_json::to_value(result).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success(value)))
}

/// 将供应商加入黑名单
pub async fn add_to_blacklist(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateBlacklistRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierBlacklistService::new(state.db.clone());
    let record = service.add_to_blacklist(req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(record).map_err(AppError::from)?,
        "供应商已加入黑名单",
    )))
}

/// 解除供应商黑名单
pub async fn release_from_blacklist(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierBlacklistService::new(state.db.clone());
    let record = service.release_from_blacklist(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(record).map_err(AppError::from)?,
        "供应商已从黑名单解除",
    )))
}
