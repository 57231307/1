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
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDate;
use serde::Deserialize;

/// 更新固废处置状态请求体
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateWasteStatusRequest {
    pub status: String,
    pub disposal_date: Option<NaiveDate>,
}

/// 创建污染物监测记录（自动判定是否超标）
pub async fn create_monitoring_record(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateMonitoringRecordRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 登记人身份取会话（AuthContext.user_id），请求体不承载操作人，落库无兜底。
    let service = PollutionMonitoringService::new(state.db.clone());
    let model = service.create_monitoring_record(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询监测记录列表（分页）
pub async fn list_monitoring_records(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<MonitoringRecordQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = PollutionMonitoringService::new(state.db.clone());
    let (list, total) = service.list_monitoring_records(params, Some(&ctx)).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": serde_json::to_value(list)?,
        "total": total,
    }))))
}

/// 创建固废处置联单
pub async fn create_solid_waste_disposal(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSolidWasteDisposalRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 建单人取服务端会话（AuthContext.user_id），请求体不承载身份。
    let service = PollutionMonitoringService::new(state.db.clone());
    let model = service
        .create_solid_waste_disposal(req, auth.user_id)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 更新固废处置状态
pub async fn update_waste_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWasteStatusRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = PollutionMonitoringService::new(state.db.clone());
    // 行级归属门：固废处置联单表无 department_id 列，按成员集合判定
    let waste = service.get_waste_by_id(id).await?;
    if !crate::utils::data_scope::check_resource_owner_by_member_scope(&ctx, waste.created_by) {
        return Err(AppError::permission_denied(
            "无权访问该固废处置记录（数据范围限制）".to_string(),
        ));
    }
    let model = service
        .update_waste_status(id, &req.status, req.disposal_date)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 扫描超标记录并生成预警
pub async fn scan_exceedance_alerts(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = PollutionMonitoringService::new(state.db.clone());
    let alerts = service.scan_exceedance_alerts(Some(&ctx)).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(alerts)?)))
}
