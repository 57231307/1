//! 认证处理器：会话/登出/登录日志记录
//!
//! 拆分自 auth_handler.rs：原 record_login_attempt 私有 fn + logout 业务独立成文件。

use crate::middleware::audit_context::AuditContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::auth_service::AuthService;
use crate::utils::app_state::AppState;
use crate::utils::cache::Cache;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::SameSite;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use serde::Serialize;
use std::sync::Arc;
use time::Duration as CookieDuration;

/// Record login attempt to the login log table for security auditing
pub async fn record_login_attempt(
    state: &AppState,
    username: &str,
    user_id: i32,
    ip_address: &str,
    user_agent: &str,
    status: &str,
    fail_reason: Option<&str>,
) {
    use crate::models::log_login;
    use sea_orm::ActiveModelTrait;

    let active_log = log_login::ActiveModel {
        user_id: sea_orm::Set(Some(user_id)),
        username: sea_orm::Set(username.to_string()),
        login_type: sea_orm::Set(Some("password".to_string())),
        ip_address: sea_orm::Set(Some(ip_address.to_string())),
        user_agent: sea_orm::Set(Some(user_agent.to_string())),
        status: sea_orm::Set(status.to_string()),
        fail_reason: sea_orm::Set(fail_reason.map(|s| s.to_string())),
        login_time: sea_orm::Set(Some(Utc::now())),
        ..Default::default()
    };

    match active_log.insert(state.db.as_ref()).await {
        Ok(_) => {}
        Err(e) => tracing::error!("Failed to record login attempt: {}", e),
    }
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
}

pub async fn logout(
    State(state): State<AppState>,
    audit_ctx: Option<Extension<AuditContext>>,
    jar: axum_extra::extract::PrivateCookieJar,
    headers: HeaderMap,
) -> Result<axum::response::Response, AppError> {
    // 提取 Token
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "));

    let mut logout_user_id: Option<i32> = None;
    let mut logout_username: Option<String> = None;

    if let Some(auth_header) = auth_header {
        let token = &auth_header[7..];

        // 验证 Token 是否有效
        match AuthService::validate_token_static(token, &state.jwt_secret) {
            Ok(claims) => {
                logout_user_id = Some(claims.sub);
                logout_username = Some(claims.username.clone());
                let now = chrono::Utc::now().timestamp() as usize;
                let exp = claims.exp.timestamp() as usize;

                if exp > now {
                    let ttl = std::time::Duration::from_secs((exp - now) as u64);
                    // 将 Token 加入黑名单
                    state
                        .cache
                        .get_token_blacklist()
                        .set(token.to_string(), true, Some(ttl));
                    tracing::info!("Token blacklisted for user {}", claims.username);
                }
            }
            Err(e) => {
                tracing::warn!("Logout attempted with invalid token: {:?}", e);
            }
        }
    }

    // 异步记录审计日志：登出（P13 批 1 P3-2）
    let logout_event = AuditEvent {
        tenant_id: None,
        user_id: logout_user_id,
        username: logout_username.clone(),
        operation_type: OperationType::Logout,
        severity: Severity::Info,
        resource_type: Some("auth".to_string()),
        resource_id: logout_user_id.map(|i| i.to_string()),
        resource_name: logout_username,
        description: Some("用户登出".to_string()),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/erp/auth/logout".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(logout_event, audit_ctx.map(|e| e.0));

    let is_production =
        std::env::var("ENV").unwrap_or_else(|_| "development".to_string()) == "production";

    // 清除所有登录态 Cookie（max_age 设为 0 即立刻过期）
    // - access_token / refresh_token: httpOnly，防 XSS
    // - csrf_token: 非 httpOnly，CSRF 头读取使用
    // - jwt: 旧版兼容 Cookie
    let removal_access = axum_extra::extract::cookie::Cookie::build(("access_token", ""))
        .path("/")
        .http_only(true)
        .secure(is_production)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::seconds(0))
        .build();
    let removal_refresh = axum_extra::extract::cookie::Cookie::build(("refresh_token", ""))
        .path("/")
        .http_only(true)
        .secure(is_production)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::seconds(0))
        .build();
    let removal_csrf = axum_extra::extract::cookie::Cookie::build(("csrf_token", ""))
        .path("/")
        .http_only(false)
        .secure(is_production)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::seconds(0))
        .build();
    let removal_jwt = axum_extra::extract::cookie::Cookie::build(("jwt", ""))
        .path("/")
        .http_only(true)
        .secure(is_production)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(0))
        .build();

    let jar = jar
        .add(removal_access)
        .add(removal_refresh)
        .add(removal_csrf)
        .add(removal_jwt);

    Ok((
        jar,
        axum::Json(ApiResponse::success(LogoutResponse { success: true })),
    )
        .into_response())
}
