use crate::middleware::auth_context::AuthContext;
use crate::services::omni_audit_query_service::{AuditQueryFilter, AuditStats};
use crate::services::omni_audit_service::OmniAuditMessage;
use crate::utils::admin_checker::is_admin_role;
use crate::container::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::sql_escape::safe_like_pattern;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use validator::Validate;

/// P0 8-5 修复：omni_audit 查询接口要求 admin 角色
async fn require_admin_role(state: &AppState, auth: &AuthContext) -> Result<(), AppError> {
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

// P3 8-19 修复：添加 validator 长度校验，防止超长字段污染审计日志或触发 DB 错误
#[derive(Debug, Deserialize, validator::Validate)]
pub struct TrackEventRequest {
    #[validate(length(max = 64))]
    pub event_type: String,
    #[validate(length(max = 128))]
    pub event_name: String,
    #[validate(length(max = 64))]
    pub resource: String,
    #[validate(length(max = 64))]
    pub action: String,
    /// payload 上限 10KB（在 handler 中校验序列化后字节数）
    pub payload: Option<serde_json::Value>,
    pub duration_ms: Option<i32>,
    #[validate(length(max = 32))]
    pub status: Option<String>,
}

/// 接收前端发来的 UI 埋点事件
pub async fn track_event(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<TrackEventRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // P3 8-19 修复：字段长度校验
    req.validate()
        .map_err(|e| AppError::validation(format!("埋点事件字段校验失败: {}", e)))?;
    // P3 8-19 修复：payload 上限 10KB
    if let Some(ref payload) = req.payload {
        let payload_size = serde_json::to_string(payload)
            .map(|s| s.len())
            .unwrap_or(usize::MAX);
        if payload_size > 10_240 {
            return Err(AppError::validation("payload 超过 10KB 上限"));
        }
    }

    let trace_id = uuid::Uuid::new_v4().to_string();

    state.omni_audit.log(OmniAuditMessage {
        trace_id,
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
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
        // V15 P0-S19 补齐：track_event 手动上报事件无 query string，condition 为 None
        condition: None,
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

    // P2 8-11 修复：单条 SQL 一次性统计 4 个指标，避免 4 次往返
    let sql = "SELECT
        (SELECT COUNT(*) FROM omni_audit_logs WHERE created_at > NOW() - INTERVAL '24 hours') AS total_events_today,
        (SELECT COUNT(*) FROM omni_audit_logs WHERE created_at > NOW() - INTERVAL '24 hours' AND request_method IS NULL) AS ui_clicks_today,
        (SELECT COUNT(*) FROM omni_audit_logs WHERE created_at > NOW() - INTERVAL '24 hours' AND request_method IS NOT NULL) AS api_calls_today,
        (SELECT COUNT(*) FROM omni_audit_logs WHERE created_at > NOW() - INTERVAL '24 hours' AND (response_status = 403 OR response_status >= 500)) AS security_alerts_today";
    let result = (*state.db)
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        ))
        .await?;

    let stats = match result {
        Some(r) => AuditStats {
            total_events_today: r.try_get::<i64>("", "total_events_today")?,
            ui_clicks_today: r.try_get::<i64>("", "ui_clicks_today")?,
            api_calls_today: r.try_get::<i64>("", "api_calls_today")?,
            security_alerts_today: r.try_get::<i64>("", "security_alerts_today")?,
        },
        None => AuditStats {
            total_events_today: 0,
            ui_clicks_today: 0,
            api_calls_today: 0,
            security_alerts_today: 0,
        },
    };

    Ok(Json(ApiResponse::success(stats)))
}

// 复杂条件检索审计日志（P2 8-10/8-12 修复：动态 WHERE + 显式字段列表，敏感字段按需返回）
pub async fn search_logs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(filter): Query<AuditQueryFilter>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_admin_role(&state, &auth).await?;
    use sea_orm::ConnectionTrait;

    let (_page, page_size, offset) = compute_pagination(&filter);
    let (start_date, end_date) = compute_date_range(&filter);
    let (where_sql, where_params, param_idx) = build_where_clause(&filter, start_date, end_date);
    let select_fields = build_select_fields(filter.include_sensitive);

    let list_sql = format!(
        "SELECT {} FROM omni_audit_logs{} ORDER BY id DESC LIMIT ${} OFFSET ${}",
        select_fields,
        where_sql,
        param_idx,
        param_idx + 1
    );
    let mut list_params = where_params.clone();
    list_params.push(page_size.into());
    list_params.push(offset.into());

    let rows = (*state.db)
        .query_all(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            list_sql,
            list_params,
        ))
        .await?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row_to_json(&row, filter.include_sensitive)?);
    }

    let total = query_total_count(&state, &where_sql, where_params).await?;
    let res = serde_json::json!({ "items": items, "total": total });
    Ok(Json(ApiResponse::success(res)))
}

// 计算分页参数：page 上限 1000，page_size 上限 100，防止深度分页全表扫描
fn compute_pagination(filter: &AuditQueryFilter) -> (u64, u64, u64) {
    let page: u64 = filter.page.unwrap_or(1).clamp(1, 1000);
    let page_size: u64 = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset: u64 = page.saturating_sub(1) * page_size;
    (page, page_size, offset)
}

// 计算日期范围：默认近 30 天，防止全表扫描
fn compute_date_range(filter: &AuditQueryFilter) -> (chrono::NaiveDate, chrono::NaiveDate) {
    let now = chrono::Utc::now().date_naive();
    let default_start = now - chrono::Duration::days(30);
    let start_date = filter.start_date.unwrap_or(default_start);
    let end_date = filter.end_date.unwrap_or(now);
    (start_date, end_date)
}

// 根据 filter 动态构造 WHERE 子句和绑定参数，返回 (where_sql, where_params, next_param_idx)
fn build_where_clause(
    filter: &AuditQueryFilter,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> (String, Vec<sea_orm::Value>, u32) {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut where_params: Vec<sea_orm::Value> = Vec::new();
    let mut param_idx = 1u32;
    if let Some(user_id) = filter.user_id {
        where_clauses.push(format!("user_id = ${}", param_idx));
        where_params.push(user_id.into());
        param_idx += 1;
    }
    if let Some(ref event_type) = filter.event_type {
        where_clauses.push(format!("module = ${}", param_idx));
        where_params.push(event_type.clone().into());
        param_idx += 1;
    }
    where_clauses.push(format!("created_at >= ${}::date", param_idx));
    where_params.push(start_date.into());
    param_idx += 1;
    where_clauses.push(format!(
        "created_at < (${}::date + INTERVAL '1 day')",
        param_idx
    ));
    where_params.push(end_date.into());
    param_idx += 1;
    if let Some(ref keyword) = filter.keyword {
        // 三个 ILIKE 共用同一占位符，只需绑定一次
        where_clauses.push(format!(
            "(description ILIKE ${} OR resource_name ILIKE ${} OR username ILIKE ${})",
            param_idx, param_idx, param_idx
        ));
        let kw = safe_like_pattern(keyword);
        where_params.push(kw.into());
        param_idx += 1;
    }
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };
    (where_sql, where_params, param_idx)
}

// 构建查询字段列表：敏感字段仅在 include_sensitive=true 时返回
fn build_select_fields(include_sensitive: bool) -> String {
    let sensitive_fields = if include_sensitive {
        ", request_body, user_agent, ip_address"
    } else {
        ""
    };
    format!(
        "id, trace_id, user_id, username, module, action, resource_type, resource_id, \
         resource_name, description, request_method, request_path, response_status, \
         duration_ms, created_at{}",
        sensitive_fields
    )
}

// 读取可选字符串字段，失败时返回带字段名的错误
fn get_opt_string(row: &sea_orm::QueryResult, field: &str) -> Result<String, AppError> {
    row.try_get::<Option<String>>("", field)
        .map_err(|e| AppError::internal(format!("审计日志读取 {} 失败: {}", field, e)))
        .map(|v| v.unwrap_or_default())
}

// 读取可选整数字段，失败时返回带字段名的错误
fn get_opt_int(row: &sea_orm::QueryResult, field: &str) -> Result<i32, AppError> {
    row.try_get::<Option<i32>>("", field)
        .map_err(|e| AppError::internal(format!("审计日志读取 {} 失败: {}", field, e)))
        .map(|v| v.unwrap_or(0))
}

// 将查询行转为 JSON 对象，include_sensitive 控制是否包含敏感字段
fn row_to_json(
    row: &sea_orm::QueryResult,
    include_sensitive: bool,
) -> Result<serde_json::Value, AppError> {
    let id = row
        .try_get_by_index::<i64>(0)
        .map_err(|e| AppError::internal(format!("审计日志读取 id 失败: {}", e)))?;
    let module = row
        .try_get::<String>("", "module")
        .map_err(|e| AppError::internal(format!("审计日志读取 module 失败: {}", e)))?;
    let action = row
        .try_get::<String>("", "action")
        .map_err(|e| AppError::internal(format!("审计日志读取 action 失败: {}", e)))?;
    let created_at = row
        .try_get::<String>("", "created_at")
        .map_err(|e| AppError::internal(format!("审计日志读取 created_at 失败: {}", e)))?;
    let mut item = serde_json::json!({
        "id": id,
        "trace_id": get_opt_string(row, "trace_id")?,
        "user_id": get_opt_int(row, "user_id")?,
        "username": get_opt_string(row, "username")?,
        "module": module,
        "action": action,
        "resource_type": get_opt_string(row, "resource_type")?,
        "resource_id": get_opt_string(row, "resource_id")?,
        "resource_name": get_opt_string(row, "resource_name")?,
        "description": get_opt_string(row, "description")?,
        "request_method": get_opt_string(row, "request_method")?,
        "request_path": get_opt_string(row, "request_path")?,
        "response_status": get_opt_int(row, "response_status")?,
        "duration_ms": get_opt_int(row, "duration_ms")?,
        "created_at": created_at,
    });
    if include_sensitive {
        item["request_body"] = serde_json::Value::String(get_opt_string(row, "request_body")?);
        item["user_agent"] = serde_json::Value::String(get_opt_string(row, "user_agent")?);
        item["ip_address"] = serde_json::Value::String(get_opt_string(row, "ip_address")?);
    }
    Ok(item)
}

// 执行 COUNT 查询返回总数，复用 WHERE 子句和参数确保分页 total 与列表数据一致
async fn query_total_count(
    state: &AppState,
    where_sql: &str,
    where_params: Vec<sea_orm::Value>,
) -> Result<u64, AppError> {
    use sea_orm::ConnectionTrait;
    let count_sql = format!("SELECT COUNT(*) as total FROM omni_audit_logs{}", where_sql);
    let count_result = (*state.db)
        .query_one(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            count_sql,
            where_params,
        ))
        .await?;
    Ok(match count_result {
        Some(r) => r.try_get::<i64>("", "total").unwrap_or(0) as u64,
        None => 0,
    })
}
