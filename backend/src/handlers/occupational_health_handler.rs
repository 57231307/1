//! 职业健康合规 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::occupational_health_service::{
    CreateHazardMonitoringRequest, CreateHealthExamRequest, CreatePpeDistributionRequest,
    HazardMonitoringQuery, HealthExamQuery, OccupationalHealthService, PpeDistributionQuery,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// 创建职业危害因素检测记录
pub async fn create_hazard_monitoring(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(mut req): Json<CreateHazardMonitoringRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = OccupationalHealthService::new(state.db.clone());
    let model = service.create_hazard_monitoring(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询职业危害因素检测记录列表
pub async fn list_hazard_monitorings(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<HazardMonitoringQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OccupationalHealthService::new(state.db.clone());
    let (list, total) = service.list_hazard_monitorings(params).await?;
    let value = serde_json::json!({ "list": list, "total": total });
    Ok(Json(ApiResponse::success(value)))
}

/// 创建职业健康体检档案
pub async fn create_health_exam(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(mut req): Json<CreateHealthExamRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = OccupationalHealthService::new(state.db.clone());
    let model = service.create_health_exam(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询体检档案列表
pub async fn list_health_exams(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<HealthExamQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OccupationalHealthService::new(state.db.clone());
    let (list, total) = service.list_health_exams(params).await?;
    let value = serde_json::json!({ "list": list, "total": total });
    Ok(Json(ApiResponse::success(value)))
}

/// 扫描在岗期间体检到期预警
pub async fn scan_exam_expiry_warnings(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OccupationalHealthService::new(state.db.clone());
    let warnings = service.scan_exam_expiry_warnings().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(warnings)?)))
}

/// 创建 PPE 发放记录
pub async fn create_ppe_distribution(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(mut req): Json<CreatePpeDistributionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = OccupationalHealthService::new(state.db.clone());
    let model = service.create_ppe_distribution(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询 PPE 发放记录列表
pub async fn list_ppe_distributions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PpeDistributionQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OccupationalHealthService::new(state.db.clone());
    let (list, total) = service.list_ppe_distributions(params).await?;
    let value = serde_json::json!({ "list": list, "total": total });
    Ok(Json(ApiResponse::success(value)))
}

/// 回收 PPE
pub async fn return_ppe(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OccupationalHealthService::new(state.db.clone());
    let model = service.return_ppe(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 扫描已过期的 PPE
pub async fn scan_expired_ppe(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OccupationalHealthService::new(state.db.clone());
    let list = service.scan_expired_ppe().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}
