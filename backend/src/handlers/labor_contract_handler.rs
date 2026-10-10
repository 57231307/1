//! 劳动合同 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::labor_contract_service::{
    CreateLaborContractRequest, LaborContractQuery, LaborContractService,
    UpdateLaborContractRequest,
};
use crate::utils::data_scope::check_resource_owner_by_member_scope;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDate;
use serde::Deserialize;

/// 终止劳动合同请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct TerminateLaborContractRequest {
    pub termination_date: NaiveDate,
    pub termination_reason: String,
}

/// 创建劳动合同
pub async fn create(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateLaborContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 建单人取服务端会话（AuthContext.user_id），请求体不承载身份。
    let service = LaborContractService::new(state.db.clone());
    let model = service.create(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 获取劳动合同详情
pub async fn get_by_id(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = LaborContractService::new(state.db.clone());
    let model = service.get_by_id(id).await?;
    let owner = model.created_by;
    if !check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权访问该劳动合同（数据范围限制）",
        ));
    }
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 按工人查询当前有效合同
pub async fn get_active_by_worker(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(worker_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = LaborContractService::new(state.db.clone());
    let model = service.get_active_by_worker(worker_id).await?;
    // 行级归属门：命中记录须在当前用户可见范围内；无记录时返回 null 不受影响
    if let Some(ref contract) = model {
        let owner = contract.created_by;
        if !check_resource_owner_by_member_scope(&ctx, owner) {
            return Err(AppError::permission_denied(
                "无权访问该劳动合同（数据范围限制）",
            ));
        }
    }
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 更新劳动合同
pub async fn update(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLaborContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = LaborContractService::new(state.db.clone());
    // 行级归属门：先取行验证归属再执行写入
    let existing = service.get_by_id(id).await?;
    let owner = existing.created_by;
    if !check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权操作该劳动合同（数据范围限制）",
        ));
    }
    let model = service.update(id, req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询劳动合同列表
pub async fn list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<LaborContractQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = LaborContractService::new(state.db.clone());
    let (list, total) = service.list(params, Some(&ctx)).await?;
    let value = serde_json::json!({ "list": list, "total": total });
    Ok(Json(ApiResponse::success(value)))
}

/// 终止劳动合同
pub async fn terminate(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<TerminateLaborContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = LaborContractService::new(state.db.clone());
    // 行级归属门：终止是写操作，先验证归属
    let existing = service.get_by_id(id).await?;
    let owner = existing.created_by;
    if !check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权操作该劳动合同（数据范围限制）",
        ));
    }
    let model = service
        .terminate(id, req.termination_date, req.termination_reason)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 扫描合同到期预警
pub async fn scan_expiry_warnings(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = LaborContractService::new(state.db.clone());
    let warnings = service.scan_expiry_warnings(Some(&ctx)).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(warnings)?)))
}
