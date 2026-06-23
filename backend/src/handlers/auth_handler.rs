use crate::middleware::audit_context::AuditContext;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::auth_service::AuthService;
use crate::services::enhanced_logger::{
    self, DeviceInfo, FailureInfo, LoginAttempt, LoginSecurityLog, SecurityInfo,
};
use crate::services::totp_service::TotpService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use super::auth_handler_session::record_login_attempt;
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{Duration as ChronoDuration, Utc};
use time::Duration as CookieDuration;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use validator::Validate;

const MAX_FAILED_ATTEMPTS: i32 = 5;
const LOCKOUT_DURATION_MINUTES: i64 = 30;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(length(min = 3, max = 50, message = "用户名长度必须在3到50个字符之间"))]
    pub username: String,
    pub password: String,
    // 可选：如果用户开启了 TOTP，则必须在登录时传入此项
    pub totp_token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserPermissionDto {
    pub resource: String,
    pub action: String,
    pub resource_id: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub csrf_token: String,
    pub user: UserInfo,
    pub permissions: Vec<UserPermissionDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub role_id: Option<i32>,
}

#[utoipa::path(
    post,
    path = "/api/v1/erp/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<LoginResponse>),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "未授权或密码错误")
    ),
    tags = ["Auth"]
)]
// 防御性 allow：
// - redundant_clone：函数内多处 `.clone()`（payload.username / client_ip / csrf_token / token
//   等）虽然当前都是必要消费，但 wave 3+ 接入新审计字段 / 多设备 session 跟踪时
//   局部 clone 形态变化可能误报，预先抑制避免 CI 抖动。
// - unused_variables：`rotated` / `csrf_ip` / `csrf_token` 在 wave 3 #7 强制轮换分支中
//   短期作为中间值使用，未来若拆分到 helper 函数时可能暂时未消费，保留标注。
// - needless_pass_by_value：axum Json 提取器要求 owned LoginRequest，无法改为引用。
// login 函数
// 保留 `clippy::redundant_clone` 抑制：
// `audit_ctx.clone()` 与 `csrf_token.clone()` 用于
// Option<Extension<T>> 的 Option::map 闭包 + Cookie 构建。
#[allow(clippy::redundant_clone)]
pub async fn login(
    State(state): State<AppState>,
    audit_ctx: Option<Extension<AuditContext>>,
    jar: axum_extra::extract::PrivateCookieJar,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    if let Err(errors) = payload.validate() {
        let error_msgs: Vec<String> = errors
            .field_errors()
            .iter()
            .map(|(field, errs)| {
                let msgs: Vec<String> = errs
                    .iter()
                    .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
                    .collect();
                format!("{}: {}", field, msgs.join(", "))
            })
            .collect();

        return Err(AppError::bad_request(format!(
            "输入验证失败: {}",
            error_msgs.join("; ")
        )));
    }

    // Extract client IP for logging
    let client_ip = headers
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .or_else(|| headers.get("X-Real-IP").and_then(|h| h.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Check account lockout before authentication (per username+IP to prevent DoS)
    let since = Utc::now() - ChronoDuration::minutes(LOCKOUT_DURATION_MINUTES);
    use crate::models::log_login;
    use sea_orm::PaginatorTrait;

    // Per-IP lockout: prevents an attacker from locking out a legitimate user
    let recent_ip_failures = log_login::Entity::find()
        .filter(log_login::Column::Username.eq(payload.username.as_str()))
        .filter(log_login::Column::Status.eq("FAILED"))
        .filter(log_login::Column::LoginTime.gte(since))
        .filter(log_login::Column::IpAddress.eq(client_ip.as_str()))
        .count(state.db.as_ref())
        .await
        .unwrap_or_default();

    // Per-username global lockout with higher threshold (10 attempts from any IP)
    let recent_user_failures = log_login::Entity::find()
        .filter(log_login::Column::Username.eq(payload.username.as_str()))
        .filter(log_login::Column::Status.eq("FAILED"))
        .filter(log_login::Column::LoginTime.gte(since))
        .count(state.db.as_ref())
        .await
        .unwrap_or_default();

    if recent_ip_failures >= MAX_FAILED_ATTEMPTS as u64 {
        tracing::warn!(
            "Account locked due to too many failed attempts from IP: {} for user {}",
            client_ip,
            payload.username
        );
        return Err(AppError::too_many_requests(
            "登录失败次数过多，请30分钟后再试",
        ));
    }

    if recent_user_failures >= (MAX_FAILED_ATTEMPTS * 2) as u64 {
        tracing::warn!(
            "Account globally locked due to too many failed attempts: {}",
            payload.username
        );
        return Err(AppError::too_many_requests("账号已被锁定，请30分钟后再试"));
    }

    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());

    match auth_service
        .authenticate(&payload.username, &payload.password)
        .await
    {
        Ok((token, user)) => {
            // 验证 TOTP 逻辑 (如已开启)
            if user.is_totp_enabled {
                let totp_token = match payload.totp_token {
                    Some(ref t) => t,
                    None => {
                        // Record failed login (missing TOTP)
                        record_login_attempt(
                            &state,
                            &payload.username,
                            user.id,
                            &client_ip,
                            &user_agent,
                            "FAILED",
                            Some("TOTP token missing"),
                        )
                        .await;
                        return Err(AppError::unauthorized("需要提供两步验证码"));
                    }
                };

                let totp_service = TotpService::new(state.db.clone());
                match totp_service.verify_login_totp(user.id, totp_token).await {
                    Ok(true) => {} // 验证通过
                    _ => {
                        record_login_attempt(
                            &state,
                            &payload.username,
                            user.id,
                            &client_ip,
                            &user_agent,
                            "FAILED",
                            Some("TOTP verification failed"),
                        )
                        .await;
                        return Err(AppError::unauthorized("两步验证码错误"));
                    }
                }
            }

            // Record successful login
            record_login_attempt(
                &state,
                &payload.username,
                user.id,
                &client_ip,
                &user_agent,
                "SUCCESS",
                None,
            )
            .await;

            // 记录增强登录安全日志
            let security_log = LoginSecurityLog {
                event: "LOGIN_SUCCESS".to_string(),
                attempt: LoginAttempt {
                    username: payload.username.clone(),
                    ip_address: client_ip.clone(),
                    user_agent: user_agent.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                    method: "password".to_string(),
                    login_type: "web".to_string(),
                },
                failure_info: None,
                security_info: SecurityInfo {
                    risk_level: "LOW".to_string(),
                    risk_factors: Vec::new(),
                    blocked: false,
                    block_reason: None,
                    require_captcha: false,
                    notify_user: false,
                },
                geo_info: None,
                device_info: DeviceInfo {
                    os: None,
                    browser: None,
                    device_type: "unknown".to_string(),
                    is_mobile: false,
                },
            };
            enhanced_logger::EnhancedLogger::log_login_security(&security_log);

            // 异步记录审计日志：登录成功（P13 批 1 P3-2）
            // 登录事件无 tenant_id（登录前尚未确定租户），写入系统级日志
            let login_event = AuditEvent {
                tenant_id: None,
                user_id: Some(user.id),
                username: Some(payload.username.clone()),
                operation_type: OperationType::Login,
                severity: Severity::Info,
                resource_type: Some("auth".to_string()),
                resource_id: Some(user.id.to_string()),
                resource_name: Some(user.username.clone()),
                description: Some("用户登录成功".to_string()),
                request_method: Some("POST".to_string()),
                request_path: Some("/api/v1/erp/auth/login".to_string()),
                before_snapshot: None,
                after_snapshot: None,
            };
            let svc = Arc::new(AuditLogService::new(state.db.clone()));
            svc.record_async(login_event, audit_ctx.clone().map(|e| e.0));

            // Update last login timestamp
            let user_svc = crate::services::user_service::UserService::new(state.db.clone());
            let _ = user_svc.update_last_login(user.id).await;

            let mut permissions = vec![];
            if let Some(role_id) = user.role_id {
                let role_perms = crate::models::role_permission::Entity::find()
                    .filter(crate::models::role_permission::Column::RoleId.eq(role_id))
                    .filter(crate::models::role_permission::Column::Allowed.eq(true))
                    .all(state.db.as_ref())
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to query role permissions: {}", e);
                        AppError::internal("查询权限失败")
                    })?;

                permissions = role_perms
                    .into_iter()
                    .map(|p| UserPermissionDto {
                        resource: p.resource_type,
                        action: p.action,
                        resource_id: p.resource_id,
                    })
                    .collect();
            }

            let user_info = UserInfo {
                id: user.id,
                username: user.username,
                email: user.email,
                role_id: user.role_id,
            };

            // 生成 CSRF Token，使用 JWT claims 中的 session_id 作为会话标识
            let claims =
                AuthService::validate_token_static(&token, &state.jwt_secret).map_err(|e| {
                    tracing::error!("Failed to decode JWT token: {}", e);
                    AppError::unauthorized("无效的认证令牌")
                })?;
            let session_id = claims.session_id;

            // 提取客户端 IP（Wave 3 安全漏洞 #7：IP 绑定到 CSRF Token）
            // 优先从 AuditContext 取（已处理 X-Real-IP / X-Forwarded-For 多级降级），
            // 缺失时回退到 local 提取（双保险，与 audit_log 一致）。
            let csrf_ip = audit_ctx
                .as_ref()
                .map(|e| e.0.ip_address.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| client_ip.clone());

            // 强制轮换：登录前清除该 user_id 关联的旧 CSRF Token（Wave 3 #7）
            let rotated = state.cache.clear_old_csrf_token_for_user(user.id);
            if rotated {
                tracing::info!(
                    user_id = user.id,
                    username = %payload.username,
                    "已清除该用户的旧 CSRF Token（强制轮换）"
                );
            }

            // 生成随机 CSRF Token 并存储到缓存中（Wave 3 #7）：
            // - 缓存值 = (session_id, ip_address) 元组，IP 用于消费时校验
            // - 反向索引 user_id → csrf_token 支持强制轮换
            // - TTL = CSRF_TOKEN_DEFAULT_TTL_SECS (1800s = 30min)，与 access_token Cookie 对齐
            let csrf_token = uuid::Uuid::new_v4().to_string();
            state.cache.set_csrf_token(
                csrf_token.clone(),
                session_id,
                csrf_ip,
                user.id,
                None, // 使用默认 TTL (1800s)
            );

            // 生成 refresh_token (简单的随机字符串)
            let refresh_token = uuid::Uuid::new_v4().to_string();

            let response = LoginResponse {
                token: token.clone(),
                refresh_token,
                csrf_token,
                user: user_info,
                permissions,
            };

            // 创建 HttpOnly Cookie
            // 开发环境下关闭 secure 标志，允许 HTTP 传输；生产环境必须开启 HTTPS
            let is_production =
                std::env::var("ENV").unwrap_or_else(|_| "development".to_string()) == "production";

            // access_token: httpOnly（防 XSS 窃取），SameSite=Strict 防止跨站请求携带
            let access_cookie = Cookie::build(("access_token", token.clone()))
                .path("/")
                .http_only(true)
                .secure(is_production)
                .same_site(SameSite::Strict)
                .max_age(CookieDuration::minutes(30))
                .build();

            // refresh_token: httpOnly，7 天有效期（用于续签 access_token）
            let refresh_cookie = Cookie::build(("refresh_token", response.refresh_token.clone()))
                .path("/")
                .http_only(true)
                .secure(is_production)
                .same_site(SameSite::Strict)
                .max_age(CookieDuration::days(7))
                .build();

            // csrf_token: 必须可被前端 JS 读取以注入 X-CSRF-Token 头，
            // 故 http_only=false；CSRF 防护依赖"攻击者无法读取跨域 Cookie"的同源策略
            let csrf_cookie = Cookie::build(("csrf_token", response.csrf_token.clone()))
                .path("/")
                .http_only(false)
                .secure(is_production)
                .same_site(SameSite::Strict)
                .max_age(CookieDuration::days(7))
                .build();

            // 兼容旧版客户端：保留 jwt Cookie（httpOnly）。新代码优先读取 access_token。
            let legacy_jwt_cookie = Cookie::build(("jwt", token.clone()))
                .path("/")
                .http_only(true)
                .secure(is_production)
                .same_site(SameSite::Lax)
                .max_age(CookieDuration::minutes(30))
                .build();

            let jar = jar
                .add(access_cookie)
                .add(refresh_cookie)
                .add(csrf_cookie)
                .add(legacy_jwt_cookie);

            Ok((jar, Json(ApiResponse::success(response))).into_response())
        }
        Err(e) => {
            // Record failed login attempt
            record_login_attempt(
                &state,
                &payload.username,
                0,
                &client_ip,
                &user_agent,
                "FAILED",
                Some(&e.to_string()),
            )
            .await;

            // 记录增强登录安全日志
            let security_log = LoginSecurityLog {
                event: "LOGIN_FAILURE".to_string(),
                attempt: LoginAttempt {
                    username: payload.username.clone(),
                    ip_address: client_ip.clone(),
                    user_agent: user_agent.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                    method: "password".to_string(),
                    login_type: "web".to_string(),
                },
                failure_info: Some(FailureInfo {
                    reason: e.to_string(),
                    attempts_today: recent_user_failures as i32 + 1,
                    attempts_total: 0,
                    last_success: None,
                    last_failure: Some(Utc::now().to_rfc3339()),
                }),
                security_info: SecurityInfo {
                    risk_level: if recent_user_failures >= 3 {
                        "HIGH".to_string()
                    } else {
                        "MEDIUM".to_string()
                    },
                    risk_factors: {
                        let mut factors = Vec::new();
                        if recent_user_failures >= 3 {
                            factors.push("多次失败".to_string());
                        }
                        factors
                    },
                    blocked: recent_ip_failures >= MAX_FAILED_ATTEMPTS as u64,
                    block_reason: if recent_ip_failures >= MAX_FAILED_ATTEMPTS as u64 {
                        Some("登录失败次数过多".to_string())
                    } else {
                        None
                    },
                    require_captcha: recent_user_failures >= 2,
                    notify_user: false,
                },
                geo_info: None,
                device_info: DeviceInfo {
                    os: None,
                    browser: None,
                    device_type: "unknown".to_string(),
                    is_mobile: false,
                },
            };
            enhanced_logger::EnhancedLogger::log_login_security(&security_log);

            // 异步记录审计日志：登录失败（P13 批 1 P3-2）
            let failure_event = AuditEvent {
                tenant_id: None,
                user_id: None,
                username: Some(payload.username.clone()),
                operation_type: OperationType::Login,
                severity: Severity::Warn,
                resource_type: Some("auth".to_string()),
                resource_id: None,
                resource_name: None,
                description: Some(format!("用户登录失败：{}", e)),
                request_method: Some("POST".to_string()),
                request_path: Some("/api/v1/erp/auth/login".to_string()),
                before_snapshot: None,
                after_snapshot: None,
            };
            let svc = Arc::new(AuditLogService::new(state.db.clone()));
            svc.record_async(failure_event, audit_ctx.map(|e| e.0));

            Err(AppError::unauthorized(e.to_string()))
        }
    }
}
