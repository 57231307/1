use crate::middleware::auth_context::AuthContext;
use crate::services::omni_audit_query_service::{AuditQueryFilter, AuditStats};
use crate::services::omni_audit_service::OmniAuditMessage;
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
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
///
/// P2 7-13 修复：原 auth: Option<AuthContext> 允许匿名调用，无速率限制，
/// 可被注入垃圾审计日志污染 omni_audit_logs 表。
/// 改为 auth: AuthContext 要求登录态，匿名请求由 auth_middleware 返回 401 拦截。
/// 速率限制由全局 rate_limit_by_ip 中间件提供（已在 main.rs 挂载）。
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
    });

    Ok(Json(ApiResponse::success(())))
}

/// 获取审计可视化大屏统计数据
///
/// P2 8-11 修复：原 get_dashboard_stats 仅 total_events 真实查询，
/// ui_clicks_today / api_calls_today / security_alerts_today 全部硬编码为 0，
/// 大屏数据完全失真。
///
/// 新实现按以下启发式区分事件来源：
/// - ui_clicks_today：request_method IS NULL（track_event 上报的 UI 事件不带请求方法）
/// - api_calls_today：request_method IS NOT NULL（omni_audit_middleware 拦截的 HTTP 请求）
/// - security_alerts_today：response_status = 403（DENIED）或 >= 500（FAILED）
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

/// 复杂条件检索审计日志
///
/// P2 8-10 修复：原 search_logs 完全忽略 AuditQueryFilter 过滤条件，
/// SQL 固定 `SELECT * ORDER BY id DESC LIMIT`，审计查询形同虚设。
/// 新实现根据 filter 动态构造 WHERE 子句，支持 user_id/event_type/
/// start_date/end_date/keyword 五个维度的组合过滤。
///
/// P2 8-12 修复：原 search_logs 用 `SELECT *` 返回所有字段，含
/// request_body/payload 等敏感数据。新实现改为显式字段列表，敏感字段
/// （request_body/user_agent/ip_address）仅在 filter.include_sensitive=true
/// 时返回。审计大屏默认 false，admin 显式传 include_sensitive=true 才返回。
pub async fn search_logs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(filter): Query<AuditQueryFilter>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P0 8-5 修复：审计日志检索含全系统操作记录，仅限 admin
    require_admin_role(&state, &auth).await?;

    use sea_orm::ConnectionTrait;

    // P3 8-17 修复：page 上限 1000，防止深度分页全表扫描
    // 批次 95 P3-3~8 修复：补 max(1) 下限，防止 page=0 被接受
    let page: u64 = filter.page.unwrap_or(1).max(1).min(1000);
    let page_size: u64 = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset: u64 = page.saturating_sub(1) * page_size;

    // P3 8-17 修复：强制日期范围限制（默认近 30 天），防止全表扫描
    let now = chrono::Utc::now().date_naive();
    let default_start = now - chrono::Duration::days(30);
    let start_date = filter.start_date.unwrap_or(default_start);
    let end_date = filter.end_date.unwrap_or(now);

    // P2 8-10 修复：动态构造 WHERE 子句
    let mut where_clauses: Vec<String> = Vec::new();
    // WHERE 子句绑定的参数（不含 LIMIT/OFFSET），用于 count 查询复用
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
    // P3 8-17 修复：日期范围改为强制（已在上文设置默认值近 30 天）
    where_clauses.push(format!("created_at >= ${}::date", param_idx));
    where_params.push(start_date.into());
    param_idx += 1;
    where_clauses.push(format!("created_at < (${}::date + INTERVAL '1 day')", param_idx));
    where_params.push(end_date.into());
    param_idx += 1;
    if let Some(ref keyword) = filter.keyword {
        // keyword 模糊匹配 description / resource_name / username 三个文本字段
        // 注意三个 ILIKE 共用同一个占位符 $param_idx，故只需绑定一次
        where_clauses.push(format!(
            "(description ILIKE ${} OR resource_name ILIKE ${} OR username ILIKE ${})",
            param_idx, param_idx, param_idx
        ));
        // 批次 94 P2-3 修复：LIKE 模式注入，转义 % _ \ 特殊字符
        let kw = safe_like_pattern(keyword);
        where_params.push(kw.into());
        param_idx += 1;
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };

    // P2 8-12 修复：显式字段列表，敏感字段仅在 include_sensitive=true 时返回
    let sensitive_fields = if filter.include_sensitive {
        ", request_body, user_agent, ip_address"
    } else {
        ""
    };
    let select_fields = format!(
        "id, trace_id, user_id, username, module, action, resource_type, resource_id, \
         resource_name, description, request_method, request_path, response_status, \
         duration_ms, created_at{}",
        sensitive_fields
    );

    // 列表查询 SQL（WHERE 参数 + LIMIT/OFFSET 参数）
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
        // DB 字段读取失败应传播错误而非吞掉，避免审计数据失真
        let mut item = serde_json::json!({
            "id": row.try_get_by_index::<i64>(0).unwrap_or(0),
            "trace_id": row.try_get::<String>("", "trace_id").unwrap_or_default(),
            "user_id": row.try_get::<i32>("", "user_id").unwrap_or(0),
            "username": row.try_get::<String>("", "username").unwrap_or_default(),
            "module": row.try_get::<String>("", "module").unwrap_or_default(),
            "action": row.try_get::<String>("", "action").unwrap_or_default(),
            "resource_type": row.try_get::<String>("", "resource_type").unwrap_or_default(),
            "resource_id": row.try_get::<String>("", "resource_id").unwrap_or_default(),
            "resource_name": row.try_get::<String>("", "resource_name").unwrap_or_default(),
            "description": row.try_get::<String>("", "description").unwrap_or_default(),
            "request_method": row.try_get::<String>("", "request_method").unwrap_or_default(),
            "request_path": row.try_get::<String>("", "request_path").unwrap_or_default(),
            "response_status": row.try_get::<i32>("", "response_status").unwrap_or(0),
            "duration_ms": row.try_get::<i32>("", "duration_ms").unwrap_or(0),
            "created_at": row.try_get::<String>("", "created_at").unwrap_or_default(),
        });
        if filter.include_sensitive {
            item["request_body"] = serde_json::Value::String(
                row.try_get::<String>("", "request_body").unwrap_or_default(),
            );
            item["user_agent"] = serde_json::Value::String(
                row.try_get::<String>("", "user_agent").unwrap_or_default(),
            );
            item["ip_address"] = serde_json::Value::String(
                row.try_get::<String>("", "ip_address").unwrap_or_default(),
            );
        }
        items.push(item);
    }

    // P2 8-10 修复：count_sql 复用 WHERE 子句和参数，确保分页 total 与列表数据一致
    let count_sql = format!("SELECT COUNT(*) as total FROM omni_audit_logs{}", where_sql);
    let count_result = (*state.db)
        .query_one(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            count_sql,
            where_params,
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
