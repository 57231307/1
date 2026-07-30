//! 劳动合同 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::labor_contract_service::{
    CreateLaborContractRequest, LaborContractQuery, LaborContractService,
    UpdateLaborContractRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;

/// 终止劳动合同请求
#[derive(Debug, Deserialize)]
pub struct TerminateLaborContractRequest {
    pub termination_date: NaiveDate,
    pub termination_reason: String,
}

/// 创建劳动合同
pub async fn create(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(mut req): Json<CreateLaborContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = LaborContractService::new(state.db.clone());
    let model = service.create(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 获取劳动合同详情
pub async fn get_by_id(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LaborContractService::new(state.db.clone());
    let model = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 按工人查询当前有效合同
pub async fn get_active_by_worker(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(worker_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LaborContractService::new(state.db.clone());
    let model = service.get_active_by_worker(worker_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 更新劳动合同
pub async fn update(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLaborContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LaborContractService::new(state.db.clone());
    let model = service.update(id, req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询劳动合同列表
pub async fn list(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<LaborContractQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LaborContractService::new(state.db.clone());
    let (list, total) = service.list(params).await?;
    let value = serde_json::json!({ "list": list, "total": total });
    Ok(Json(ApiResponse::success(value)))
}

/// 终止劳动合同
pub async fn terminate(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<TerminateLaborContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LaborContractService::new(state.db.clone());
    let model = service
        .terminate(id, req.termination_date, req.termination_reason)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 扫描合同到期预警
pub async fn scan_expiry_warnings(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LaborContractService::new(state.db.clone());
    let warnings = service.scan_expiry_warnings().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(warnings)?)))
}
