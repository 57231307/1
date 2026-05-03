use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use crate::utils::app_state::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::omni_audit_service::OmniAuditMessage;
use crate::services::omni_audit_query_service::{OmniAuditQueryService, AuditQueryFilter, AuditStats};
use crate::models::omni_audit_log;
use crate::utils::response::ApiResponse;
use crate::utils::error::AppError;

#[derive(Debug, Deserialize)]
pub struct TrackEventRequest {
    pub event_type: String,
    pub event_name: String,
    pub resource: String,
    pub action: String,
    pub payload: Option<serde_json::Value>,
    pub duration_ms: Option<i32>,
    pub status: Option<String>,
}

/// 接收前端发来的 UI 埋点事件
pub async fn track_event(
    auth: Option<AuthContext>,
    State(state): State<AppState>,
    Json(req): Json<TrackEventRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let user_id = auth.map(|a| a.user_id).unwrap_or(0);
    let trace_id = uuid::Uuid::new_v4().to_string();

    state.omni_audit.log(OmniAuditMessage {
        trace_id,
        user_id,
        event_type: req.event_type,
        event_name: req.event_name,
        resource: req.resource,
        action: req.action,
        payload: req.payload,
        ip_address: None, // 可以在中间件中获取并注入，这里简化
        user_agent: None,
        duration_ms: req.duration_ms.unwrap_or(0),
        status: req.status.unwrap_or_else(|| "SUCCESS".to_string()),
        error_msg: None,
    });

    Ok(Json(ApiResponse::success(())))
}

/// 获取审计可视化大屏统计数据
pub async fn get_dashboard_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AuditStats>>, AppError> {
    let service = OmniAuditQueryService::new(state.db.clone());
    let stats = service.get_dashboard_stats().await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// 复杂条件检索审计日志
pub async fn search_logs(
    State(state): State<AppState>,
    Query(filter): Query<AuditQueryFilter>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OmniAuditQueryService::new(state.db.clone());
    let (logs, total) = service.search_logs(filter).await?;
    
    let res = serde_json::json!({
        "items": logs,
        "total": total
    });
    
    Ok(Json(ApiResponse::success(res)))
}
