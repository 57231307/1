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
};
use axum_extra::extract::cookie::SameSite;
use chrono::Utc;
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

/// 提取并处理 Bearer Token：验证、加入进程内黑名单、吊销 JTI
async fn process_logout_token(
    state: &AppState,
    headers: &HeaderMap,
) -> (Option<i32>, Option<String>) {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "));
    let mut logout_user_id: Option<i32> = None;
    let mut logout_username: Option<String> = None;
    if let Some(auth_header) = auth_header {
        let token = &auth_header[7..];
        match AuthService::validate_token_static(token, &state.jwt_secret) {
            Ok(claims) => {
                logout_user_id = Some(claims.sub);
                logout_username = Some(claims.username.clone());
                blacklist_and_revoke_token(state, token, &claims).await;
            }
            Err(e) => {
                tracing::warn!("Logout attempted with invalid token: {:?}", e);
            }
        }
    }
    (logout_user_id, logout_username)
}

/// 将 Token 加入进程内黑名单 + 吊销 JTI 到 Redis 分布式黑名单
///
/// P1 7-3 修复：原 logout 仅写进程内 state.cache 黑名单，多实例部署时
/// 登出后 token 在其他实例仍可用（最长 2 小时）。
/// 修复方案：与 refresh_token 流程对齐，调用 revoke_jti 写入 Redis，
/// 使所有实例通过 is_jti_revoked 检测到吊销状态。
async fn blacklist_and_revoke_token(
    state: &AppState,
    token: &str,
    claims: &crate::services::auth_service::AppClaims,
) {
    let now = chrono::Utc::now().timestamp() as usize;
    let exp = claims.exp.timestamp() as usize;
    if exp > now {
        let ttl = std::time::Duration::from_secs((exp - now) as u64);
        state
            .cache
            .get_token_blacklist()
            .set(token.to_string(), true, Some(ttl));
        crate::services::auth_service::revoke_jti(
            &claims.session_id,
            claims.exp.timestamp(),
        )
        .await;
        tracing::info!(
            "Token blacklisted + JTI revoked for user {} (session_id={})",
            claims.username,
            claims.session_id
        );
    }
}

/// 异步记录登出审计日志（P13 批 1 P3-2）
fn record_logout_audit(
    state: &AppState,
    audit_ctx: Option<AuditContext>,
    logout_user_id: Option<i32>,
    logout_username: Option<String>,
) {
    let logout_event = AuditEvent {
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
    svc.record_async(logout_event, audit_ctx);
}

/// 构建清除登录态的 Cookie（max_age 设为 0 即立刻过期）
fn build_removal_cookie(
    name: &'static str,
    http_only: bool,
    is_production: bool,
) -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((name, ""))
        .path("/")
        .http_only(http_only)
        .secure(is_production)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::seconds(0))
        .build()
}

/// 清除所有登录态 Cookie
///
/// - access_token / refresh_token: httpOnly，防 XSS
/// - csrf_token: 非 httpOnly，CSRF 头读取使用
/// - jwt: 旧版兼容 Cookie
fn clear_auth_cookies(
    jar: axum_extra::extract::PrivateCookieJar,
    is_production: bool,
) -> axum_extra::extract::PrivateCookieJar {
    jar.add(build_removal_cookie("access_token", true, is_production))
        .add(build_removal_cookie("refresh_token", true, is_production))
        .add(build_removal_cookie("csrf_token", false, is_production))
        .add(build_removal_cookie("jwt", true, is_production))
}

pub async fn logout(
    State(state): State<AppState>,
    audit_ctx: Option<Extension<AuditContext>>,
    jar: axum_extra::extract::PrivateCookieJar,
    headers: HeaderMap,
) -> Result<axum::response::Response, AppError> {
    let (logout_user_id, logout_username) = process_logout_token(&state, &headers).await;
    record_logout_audit(
        &state,
        audit_ctx.map(|e| e.0),
        logout_user_id,
        logout_username.clone(),
    );
    // 漏洞 #12 修复：统一从 `crate::utils::config::is_production()` 读取 APP_ENV
    let jar = clear_auth_cookies(jar, crate::utils::config::is_production());
    Ok((
        jar,
        axum::Json(ApiResponse::success(LogoutResponse { success: true })),
    )
        .into_response())
}
