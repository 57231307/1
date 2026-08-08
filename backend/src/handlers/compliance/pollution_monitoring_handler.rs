//! 环境监测与固废处置 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::pollution_monitoring_service::{
    CreateMonitoringRecordRequest, CreateSolidWasteDisposalRequest, MonitoringRecordQuery,
    PollutionMonitoringService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;

/// 更新固废处置状态请求体
#[derive(Debug, Deserialize)]
pub struct UpdateWasteStatusRequest {
    pub status: String,
    pub disposal_date: Option<NaiveDate>,
}

/// 创建污染物监测记录（自动判定是否超标）
pub async fn create_monitoring_record(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateMonitoringRecordRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionMonitoringService::new(state.db.clone());
    let model = service.create_monitoring_record(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询监测记录列表（分页）
pub async fn list_monitoring_records(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<MonitoringRecordQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionMonitoringService::new(state.db.clone());
    let (list, total) = service.list_monitoring_records(params).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": serde_json::to_value(list)?,
        "total": total,
    }))))
}

/// 创建固废处置联单
pub async fn create_solid_waste_disposal(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateSolidWasteDisposalRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionMonitoringService::new(state.db.clone());
    let model = service.create_solid_waste_disposal(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 更新固废处置状态
pub async fn update_waste_status(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWasteStatusRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionMonitoringService::new(state.db.clone());
    let model = service
        .update_waste_status(id, &req.status, req.disposal_date)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 扫描超标记录并生成预警
pub async fn scan_exceedance_alerts(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PollutionMonitoringService::new(state.db.clone());
    let alerts = service.scan_exceedance_alerts().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(alerts)?)))
}
