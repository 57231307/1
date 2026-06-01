use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

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

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

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
        .fetch_page(page - 1)
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
    _auth: AuthContext,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<LockStatus>>, AppError> {
    let username = params
        .get("username")
        .ok_or_else(|| AppError::BadRequest("缺少 username 参数".to_string()))?;

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
    _auth: AuthContext,
    Json(params): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let username = params
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("缺少 username 参数".to_string()))?;

    use crate::models::log_login;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    log_login::Entity::delete_many()
        .filter(log_login::Column::Username.eq(username))
        .filter(log_login::Column::Status.eq("FAILED"))
        .exec(state.db.as_ref())
        .await?;

    tracing::info!("管理员手动解锁账号: {}", username);

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

    let today = Utc::now().date_naive();
    let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();

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
