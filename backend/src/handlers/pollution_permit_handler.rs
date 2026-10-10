//! 排污许可证管理 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::pollution_permit_service::{
    CreatePollutionPermitRequest, PollutionPermitQuery, PollutionPermitService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, Query, State},
};

/// 创建排污许可证
pub async fn create(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePollutionPermitRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionPermitService::new(state.db.clone());
    // 建单人取服务端会话（AuthContext.user_id），请求体不承载身份。
    let model = service.create(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询排污许可证列表（分页）
pub async fn list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<PollutionPermitQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = PollutionPermitService::new(state.db.clone());
    let (list, total) = service.list(params, Some(&ctx)).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": serde_json::to_value(list)?,
        "total": total,
    }))))
}

/// 获取排污许可证详情
pub async fn get_by_id(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = PollutionPermitService::new(state.db.clone());
    let model = service.get_by_id(id).await?;
    // 行级归属门：排污许可证表无 department_id 列，按成员集合判定
    if !crate::utils::data_scope::check_resource_owner_by_member_scope(&ctx, model.created_by) {
        return Err(AppError::permission_denied(
            "无权访问该排污许可证记录（数据范围限制）".to_string(),
        ));
    }
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 吊销排污许可证
pub async fn revoke(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = PollutionPermitService::new(state.db.clone());
    // 行级归属门：吊销前校验本行归属
    let permit = service.get_by_id(id).await?;
    if !crate::utils::data_scope::check_resource_owner_by_member_scope(&ctx, permit.created_by) {
        return Err(AppError::permission_denied(
            "无权访问该排污许可证记录（数据范围限制）".to_string(),
        ));
    }
    let model = service.revoke(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 扫描即将到期/已过期的许可证并生成预警
pub async fn scan_expiry_warnings(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = PollutionPermitService::new(state.db.clone());
    let warnings = service.scan_expiry_warnings(Some(&ctx)).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(warnings)?)))
}
