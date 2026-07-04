use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// P1-2e 修复（批次 81 v1 复审）：解锁账号请求 DTO
/// 替代 unlock_account 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Validate)]
pub struct UnlockAccountDto {
    /// 用户名：必填，长度至少 1
    #[validate(length(min = 1, max = 64, message = "用户名长度必须在1到64字符之间"))]
    pub username: String,
}

/// P0 7-1 修复：要求调用者具备 admin 角色，否则拒绝并记录审计日志
///
/// 安全原因：原 `check_lock_status`、`unlock_account`、`unlock_account_by_id`
/// 三个 handler 仅使用 `_auth: AuthContext`（下划线前缀表示参数被忽略），
/// 导致任意登录用户均可查询他人锁定状态或解锁任意账号，属水平/垂直越权。
async fn require_admin_role(
    state: &AppState,
    auth: &AuthContext,
) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        tracing::warn!(
            target: "security_audit",
            event = "AUTHORIZATION_DENIED",
            user_id = auth.user_id,
            username = %auth.username,
            "[SECURITY] 非 admin 用户调用登录安全敏感接口被拒绝"
        );
        return Err(AppError::permission_denied(
            "该操作仅限管理员（code=admin）执行",
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct LoginLogQuery {
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct LoginLogItem {
    pub id: i32,
    pub user_id: Option<i32>,
    pub username: String,
    pub login_type: String,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub status: String,
    pub fail_reason: Option<String>,
    pub login_time: String,
}

#[derive(Debug, Serialize)]
pub struct LockStatus {
    pub user_id: i32,
    pub username: String,
    pub is_locked: bool,
    pub failed_attempts: i32,
    pub locked_until: Option<String>,
    pub max_attempts: i32,
}

#[derive(Debug, Serialize)]
pub struct SecurityAlert {
    pub alert_type: String,
    pub user_id: i32,
    pub username: String,
    pub ip_address: String,
    pub location: Option<String>,
    pub detected_at: String,
    pub description: String,
}

const MAX_FAILED_ATTEMPTS: i32 = 5;
const LOCKOUT_DURATION_MINUTES: i64 = 30;

pub async fn list_login_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<LoginLogQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::log_login;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut query_builder = log_login::Entity::find();

    if let Some(user_id) = query.user_id {
        query_builder = query_builder.filter(log_login::Column::UserId.eq(user_id));
    }
    if let Some(username) = &query.username {
        query_builder = query_builder.filter(log_login::Column::Username.contains(username));
    }
    if let Some(status) = &query.status {
        query_builder = query_builder.filter(log_login::Column::Status.eq(status.clone()));
    }

    let total: u64 = query_builder.clone().count(state.db.as_ref()).await?;

    let logs = query_builder
        .order_by_desc(log_login::Column::LoginTime)
        .paginate(state.db.as_ref(), page_size)
        .fetch_page(page.saturating_sub(1))
        .await?;

    let items: Vec<LoginLogItem> = logs
        .into_iter()
        .map(|m| LoginLogItem {
            id: m.id as i32,
            user_id: m.user_id,
            username: m.username,
            login_type: m.login_type.unwrap_or_default(),
            ip_address: m.ip_address.unwrap_or_default(),
            user_agent: m.user_agent,
            status: m.status,
            fail_reason: m.fail_reason,
            login_time: m.login_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
        })
        .collect();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

pub async fn check_lock_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<LockStatus>>, AppError> {
    let username = params
        .get("username")
        .ok_or_else(|| AppError::bad_request("缺少 username 参数"))?;

    // P0 7-1 修复：水平越权防护
    // 普通用户仅能查询自己的锁定状态；admin 可查询任意用户。
    // 缺角色直接拒绝（避免 role_id=0 误匹配"超级管理员"角色）。
    let is_admin = if let Some(role_id) = auth.role_id {
        is_admin_role(&state.db, role_id).await
    } else {
        false
    };
    if !is_admin && auth.username != *username {
        tracing::warn!(
            target: "security_audit",
            event = "AUTHORIZATION_DENIED",
            user_id = auth.user_id,
            username = %auth.username,
            requested_username = %username,
            "[SECURITY] 非 admin 用户尝试查询他人锁定状态被拒绝"
        );
        return Err(AppError::permission_denied(
            "仅管理员可查询其他用户的锁定状态",
        ));
    }

    use crate::models::log_login;
    use chrono::{Duration, Utc};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let since = Utc::now() - Duration::minutes(LOCKOUT_DURATION_MINUTES);

    let recent_failures = log_login::Entity::find()
        .filter(log_login::Column::Username.eq(username.as_str()))
        .filter(log_login::Column::Status.eq("FAILED"))
        .filter(log_login::Column::LoginTime.gte(since))
        .order_by_desc(log_login::Column::LoginTime)
        .all(state.db.as_ref())
        .await?;

    let failed_count = recent_failures.len() as i32;
    let is_locked = failed_count >= MAX_FAILED_ATTEMPTS;

    let locked_until = if is_locked {
        recent_failures
            .first()
            .and_then(|f| f.login_time)
            .map(|t| (t + Duration::minutes(LOCKOUT_DURATION_MINUTES)).to_rfc3339())
    } else {
        None
    };

    Ok(Json(ApiResponse::success(LockStatus {
        user_id: 0,
        username: username.clone(),
        is_locked,
        failed_attempts: failed_count,
        locked_until,
        max_attempts: MAX_FAILED_ATTEMPTS,
    })))
}

pub async fn unlock_account(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(params): Json<UnlockAccountDto>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // P0 7-1 修复：解锁账号属敏感操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    // P1-2e 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    params
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    use crate::models::log_login;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    log_login::Entity::delete_many()
        .filter(log_login::Column::Username.eq(&params.username))
        .filter(log_login::Column::Status.eq("FAILED"))
        .exec(state.db.as_ref())
        .await?;

    tracing::info!("管理员手动解锁账号: {}", params.username);

    Ok(Json(ApiResponse::success_with_message((), "账号已解锁")))
}

pub async fn get_security_alerts(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(_params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<SecurityAlert>>>, AppError> {
    use crate::models::log_login;
    use chrono::{Duration, Utc};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let since = Utc::now() - Duration::hours(24);

    let recent_logins = log_login::Entity::find()
        .filter(log_login::Column::LoginTime.gte(since))
        .filter(log_login::Column::Status.eq("SUCCESS"))
        .order_by_desc(log_login::Column::LoginTime)
        .all(state.db.as_ref())
        .await?;

    let mut alerts = Vec::new();
    let mut user_ips: std::collections::HashMap<i32, Vec<String>> =
        std::collections::HashMap::new();

    for login in &recent_logins {
        if let Some(uid) = login.user_id {
            if let Some(ip) = &login.ip_address {
                user_ips.entry(uid).or_default().push(ip.clone());
            }
        }
    }

    for (user_id, ips) in &user_ips {
        let unique_ips: std::collections::HashSet<&String> = ips.iter().collect();
        if unique_ips.len() > 3 {
            let username = recent_logins
                .iter()
                .find(|l| l.user_id == Some(*user_id))
                .map(|l| l.username.clone())
                .unwrap_or_default();

            alerts.push(SecurityAlert {
                alert_type: "MULTI_IP_LOGIN".to_string(),
                user_id: *user_id,
                username,
                ip_address: ips.first().cloned().unwrap_or_default(),
                location: None,
                detected_at: Utc::now().to_rfc3339(),
                description: format!("24小时内从 {} 个不同 IP 登录", unique_ips.len()),
            });
        }
    }

    Ok(Json(ApiResponse::success(alerts)))
}

pub async fn get_login_statistics(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::log_login;
    use chrono::Utc;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let _today = Utc::now().date_naive();
    let today_start = crate::utils::date_utils::today_start_utc();

    let total_today = log_login::Entity::find()
        .filter(log_login::Column::LoginTime.gte(today_start))
        .count(state.db.as_ref())
        .await?;

    let success_today = log_login::Entity::find()
        .filter(log_login::Column::LoginTime.gte(today_start))
        .filter(log_login::Column::Status.eq("SUCCESS"))
        .count(state.db.as_ref())
        .await?;

    let failed_today = log_login::Entity::find()
        .filter(log_login::Column::LoginTime.gte(today_start))
        .filter(log_login::Column::Status.eq("FAILED"))
        .count(state.db.as_ref())
        .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "total_login_attempts": total_today,
        "successful_logins": success_today,
        "failed_logins": failed_today,
        "success_rate": if total_today > 0 { (success_today as f64 / total_today as f64 * 100.0).round() } else { 100.0 },
    }))))
}

pub async fn get_locked_accounts(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    use crate::models::log_login;
    use chrono::{Duration, Utc};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let since = Utc::now() - Duration::minutes(LOCKOUT_DURATION_MINUTES);

    let failed_logins = log_login::Entity::find()
        .filter(log_login::Column::Status.eq("FAILED"))
        .filter(log_login::Column::LoginTime.gte(since))
        .order_by_desc(log_login::Column::LoginTime)
        .all(state.db.as_ref())
        .await?;

    let mut locked_users: std::collections::HashMap<String, (Option<i32>, i32, Option<String>)> =
        std::collections::HashMap::new();

    for login in &failed_logins {
        // log_login.user_id 在用户不存在时为 None；保留 None 而非用 0 占位，
        // 避免与"系统用户 id=0"在审计中混淆
        let entry = locked_users.entry(login.username.clone()).or_insert((
            login.user_id,
            0,
            login.login_time.map(|t| t.to_rfc3339()),
        ));
        entry.1 += 1;
    }

    let locked_accounts: Vec<serde_json::Value> = locked_users
        .into_iter()
        .filter(|(_, (_, count, _))| *count >= MAX_FAILED_ATTEMPTS)
        .map(|(username, (user_id, attempts, last_attempt))| {
            serde_json::json!({
                "id": user_id,
                "username": username,
                "lock_reason": format!("{} 次登录失败", attempts),
                "locked_at": last_attempt,
                "unlock_at": None::<String>,
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(locked_accounts)))
}

pub async fn unlock_account_by_id(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // P0 7-1 修复：按 ID 解锁账号属敏感操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    use crate::models::{log_login, user};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let user = user::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("用户 {}", id)))?;

    log_login::Entity::delete_many()
        .filter(log_login::Column::Username.eq(&user.username))
        .filter(log_login::Column::Status.eq("FAILED"))
        .exec(state.db.as_ref())
        .await?;

    tracing::info!("管理员手动解锁账号: {} (ID: {})", user.username, id);

    Ok(Json(ApiResponse::success_with_message((), "账号已解锁")))
}

pub async fn resolve_alert(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(_id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    Ok(Json(ApiResponse::success_with_message((), "告警已处理")))
}

pub async fn export_login_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<LoginLogQuery>,
) -> Result<axum::response::Response, AppError> {
    use crate::models::log_login;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

    let mut query_builder = log_login::Entity::find();

    if let Some(user_id) = query.user_id {
        query_builder = query_builder.filter(log_login::Column::UserId.eq(user_id));
    }
    if let Some(username) = &query.username {
        query_builder = query_builder.filter(log_login::Column::Username.contains(username));
    }
    if let Some(status) = &query.status {
        query_builder = query_builder.filter(log_login::Column::Status.eq(status.clone()));
    }

    let logs = query_builder
        .order_by_desc(log_login::Column::LoginTime)
        .paginate(state.db.as_ref(), 10000)
        .fetch_page(0)
        .await?;

    let mut csv_content = "ID,用户名,登录类型,IP地址,浏览器,状态,失败原因,登录时间\n".to_string();
    for log in &logs {
        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            log.id,
            log.username,
            log.login_type.as_deref().unwrap_or(""),
            log.ip_address.as_deref().unwrap_or(""),
            log.user_agent.as_deref().unwrap_or(""),
            log.status,
            log.fail_reason.as_deref().unwrap_or(""),
            log.login_time.map(|t| t.to_rfc3339()).unwrap_or_default()
        ));
    }

    // P9-1 关键路径 unwrap 清理：HTTP 响应构建使用 map_err 把 I/O 错误转为 AppError
    // 失败时返回 500 + 中文错误信息，避免生产 panic 暴露给前端
    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header("Content-Disposition", "attachment; filename=login_logs.csv")
        .body(axum::body::Body::from(csv_content))
        .map_err(|e| {
            tracing::error!(
                error = %e,
                "P9-1: 登录日志 CSV 响应构建失败（HTTP builder 错误）"
            );
            AppError::internal(format!("P9-1: 登录日志导出响应构建失败: {e}"))
        })
}
