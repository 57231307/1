//! 排污许可证管理 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::pollution_permit_service::{
    CreatePollutionPermitRequest, PollutionPermitQuery, PollutionPermitService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// 创建排污许可证
pub async fn create(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreatePollutionPermitRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionPermitService::new(state.db.clone());
    let model = service.create(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询排污许可证列表（分页）
pub async fn list(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PollutionPermitQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionPermitService::new(state.db.clone());
    let (list, total) = service.list(params).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": serde_json::to_value(list)?,
        "total": total,
    }))))
}

/// 获取排污许可证详情
pub async fn get_by_id(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionPermitService::new(state.db.clone());
    let model = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 吊销排污许可证
pub async fn revoke(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionPermitService::new(state.db.clone());
    let model = service.revoke(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 扫描即将到期/已过期的许可证并生成预警
pub async fn scan_expiry_warnings(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionPermitService::new(state.db.clone());
    let warnings = service.scan_expiry_warnings().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(warnings)?)))
}
