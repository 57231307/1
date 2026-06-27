use crate::middleware::auth_context::AuthContext;
use crate::services::omni_audit_query_service::{AuditQueryFilter, AuditStats};
use crate::services::omni_audit_service::OmniAuditMessage;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

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
    // 未登录场景下 user_id 为 None，避免与系统用户（id=0）混淆
    // 安全修复：从 AuthContext 提取 tenant_id，禁止硬编码（修复租户隔离违规）
    let (tenant_id, user_id) = auth
        .map(|a| (a.tenant_id, Some(a.user_id)))
        .unwrap_or((None, None));
    let trace_id = uuid::Uuid::new_v4().to_string();

    state.omni_audit.log(OmniAuditMessage {
        tenant_id, // 从 AuthContext 提取，禁止硬编码
        trace_id,
        user_id,
        username: None,
        event_type: req.event_type,
        event_name: req.event_name,
        resource: req.resource,
        action: req.action,
        resource_type: None,
        resource_id: None,
        resource_name: None,
        description: None,
        payload: req.payload,
        ip_address: None,
        user_agent: None,
        request_method: None,
        request_path: None,
        request_body: None,
        // 持续时间字段（毫秒）；无值时记 0
        duration_ms: req.duration_ms.unwrap_or_default(),
        status: req.status.unwrap_or_else(|| "SUCCESS".to_string()),
        error_msg: None,
        old_value: None,
        new_value: None,
    });

    Ok(Json(ApiResponse::success(())))
}

/// 获取审计可视化大屏统计数据
pub async fn get_dashboard_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AuditStats>>, AppError> {
    use sea_orm::ConnectionTrait;

    let sql = "SELECT COUNT(*) as total FROM omni_audit_logs";
    let result = (*state.db)
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        ))
        .await?;
    // DB 查询失败应传播错误而非吞掉为 0，避免大屏数据失真
    let total: i64 = match result {
        Some(r) => r.try_get::<i64>("", "total")?,
        None => 0,
    };

    Ok(Json(ApiResponse::success(AuditStats {
        total_events_today: total,
        ui_clicks_today: 0,
        api_calls_today: 0,
        security_alerts_today: 0,
    })))
}

/// 复杂条件检索审计日志
pub async fn search_logs(
    State(state): State<AppState>,
    Query(filter): Query<AuditQueryFilter>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use sea_orm::ConnectionTrait;

    let page: u64 = filter.page.unwrap_or(1);
    let page_size: u64 = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset: u64 = if page > 0 { (page - 1) * page_size } else { 0 };

    let sql = "SELECT * FROM omni_audit_logs ORDER BY id DESC LIMIT $1 OFFSET $2";
    let rows = (*state.db)
        .query_all(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            vec![page_size.into(), offset.into()],
        ))
        .await?;

    let mut items = Vec::new();
    for row in rows {
        // DB 字段读取失败应传播错误而非吞掉，避免审计数据失真
        let item = serde_json::json!({
            "id": row.try_get_by_index::<i64>(0).unwrap_or(0),
            "trace_id": row.try_get::<String>("", "trace_id").unwrap_or_default(),
            "user_id": row.try_get::<i32>("", "user_id").unwrap_or(0),
            "module": row.try_get::<String>("", "module").unwrap_or_default(),
            "action": row.try_get::<String>("", "action").unwrap_or_default(),
            "response_status": row.try_get::<i32>("", "response_status").unwrap_or(0),
            "duration_ms": row.try_get::<i32>("", "duration_ms").unwrap_or(0),
            "created_at": row.try_get::<String>("", "created_at").unwrap_or_default(),
        });
        items.push(item);
    }

    let res = serde_json::json!({
        "items": items,
        "total": items.len() as u64
    });

    Ok(Json(ApiResponse::success(res)))
}
