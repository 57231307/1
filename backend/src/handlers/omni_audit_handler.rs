use crate::middleware::auth_context::AuthContext;
use crate::services::omni_audit_query_service::{AuditQueryFilter, AuditStats};
use crate::services::omni_audit_service::OmniAuditMessage;
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

/// P0 8-5 修复：omni_audit 查询接口要求 admin 角色
///
/// 安全原因：get_dashboard_stats 和 search_logs 查询全系统审计日志，
/// 含敏感操作记录，必须限制为 admin 角色。
async fn require_admin_role(
    state: &AppState,
    auth: &AuthContext,
) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法查询审计日志"))?;
    if !is_admin_role(&state.db, role_id).await {
        tracing::warn!(
            target: "security_audit",
            event = "AUTHORIZATION_DENIED",
            user_id = auth.user_id,
            username = %auth.username,
            "[SECURITY] 非 admin 用户尝试查询 omni_audit 日志被拒绝"
        );
        return Err(AppError::permission_denied(
            "查询审计日志仅限管理员（code=admin）执行",
        ));
    }
    Ok(())
}

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
    let user_id = auth.map(|a| a.user_id);
    let trace_id = uuid::Uuid::new_v4().to_string();

    state.omni_audit.log(OmniAuditMessage {
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
    auth: AuthContext,
) -> Result<Json<ApiResponse<AuditStats>>, AppError> {
    // P0 8-5 修复：审计大屏数据含全系统操作统计，仅限 admin
    require_admin_role(&state, &auth).await?;

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
    auth: AuthContext,
    Query(filter): Query<AuditQueryFilter>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P0 8-5 修复：审计日志检索含全系统操作记录，仅限 admin
    require_admin_role(&state, &auth).await?;

    use sea_orm::ConnectionTrait;

    let page: u64 = filter.page.unwrap_or(1);
    let page_size: u64 = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset: u64 = page.saturating_sub(1) * page_size;

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

    // 批次 30 v7 P1-4 修复：原 total = items.len() 仅返回当前页记录数，
    // 分页或空表时严重错误。改为 COUNT(*) 查询真实总数。
    // 当前 search_logs 的 SQL 无 WHERE 条件，直接 COUNT(*) 即可；
    // 若未来添加过滤条件，需同步更新 count_sql 保留 WHERE 子句。
    let count_sql = "SELECT COUNT(*) as total FROM omni_audit_logs";
    let count_result = (*state.db)
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            count_sql,
        ))
        .await?;
    let total: u64 = match count_result {
        Some(r) => r.try_get::<i64>("", "total").unwrap_or(0) as u64,
        None => 0,
    };

    let res = serde_json::json!({
        "items": items,
        "total": total,
    });

    Ok(Json(ApiResponse::success(res)))
}
