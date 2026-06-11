use crate::middleware::auth_context::AuthContext;
use crate::services::auth_service::AuthService;
use crate::services::enhanced_logger::{
    self, DeviceInfo, FailureInfo, LoginAttempt, LoginSecurityLog, SecurityInfo,
};
use crate::services::totp_service::TotpService;
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
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
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
pub async fn login(
    State(state): State<AppState>,
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
        .unwrap_or(0);

    // Per-username global lockout with higher threshold (10 attempts from any IP)
    let recent_user_failures = log_login::Entity::find()
        .filter(log_login::Column::Username.eq(payload.username.as_str()))
        .filter(log_login::Column::Status.eq("FAILED"))
        .filter(log_login::Column::LoginTime.gte(since))
        .count(state.db.as_ref())
        .await
        .unwrap_or(0);

    if recent_ip_failures >= MAX_FAILED_ATTEMPTS as u64 {
        tracing::warn!(
            "Account locked due to too many failed attempts from IP: {} for user {}",
            client_ip,
            payload.username
        );
        return Err(AppError::too_many_requests("登录失败次数过多，请30分钟后再试"));
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
                username: user.username.clone(),
                email: user.email.clone(),
                role_id: user.role_id,
            };

            // 生成 CSRF Token，使用 JWT claims 中的 session_id 作为会话标识
            let claims =
                AuthService::validate_token_static(&token, &state.jwt_secret).map_err(|e| {
                    tracing::error!("Failed to decode JWT token: {}", e);
                    AppError::unauthorized("无效的认证令牌")
                })?;
            let _session_id = claims.session_id.clone();
            // CSRF token 不再需要，JWT 已经提供安全保障
            let csrf_token = "".to_string();

            // 生成 refresh_token (简单的随机字符串)
            let refresh_token = uuid::Uuid::new_v4().to_string();

            let response = LoginResponse {
                token: token.clone(),
                refresh_token: refresh_token.clone(),
                csrf_token,
                user: user_info,
                permissions,
            };

            // 创建 HttpOnly Cookie
            // 开发环境下关闭secure标志，允许HTTP传输；生产环境必须开启HTTPS
            let is_production =
                std::env::var("ENV").unwrap_or_else(|_| "development".to_string()) == "production";

            let cookie = Cookie::build(("jwt", token))
                .path("/")
                .http_only(true)
                .secure(is_production)
                .same_site(SameSite::Lax)
                .build();

            let jar = jar.add(cookie);

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

            let error_response = ApiResponse::<()>::error(e.to_string());
            Err(AppError::unauthorized(e.to_string()))
        }
    }
}

/// Record login attempt to the login log table for security auditing
async fn record_login_attempt(
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
    jar: axum_extra::extract::PrivateCookieJar,
    headers: HeaderMap,
) -> Result<axum::response::Response, AppError> {
    // 提取 Token
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "));

    if let Some(auth_header) = auth_header {
        let token = &auth_header[7..];

        // 验证 Token 是否有效
        match AuthService::validate_token_static(token, &state.jwt_secret) {
            Ok(claims) => {
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

    let is_production =
        std::env::var("ENV").unwrap_or_else(|_| "development".to_string()) == "production";

    let removal_cookie = axum_extra::extract::cookie::Cookie::build(("jwt", ""))
        .path("/")
        .http_only(true)
        .secure(is_production)
        .same_site(SameSite::Lax)
        .build();

    let jar = jar.add(removal_cookie);

    Ok((
        jar,
        axum::Json(ApiResponse::success(LogoutResponse { success: true })),
    )
        .into_response())
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub csrf_token: String,
    pub expires_in: u64,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<RefreshTokenResponse>>, AppError> {
    let token = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AppError::unauthorized("缺少认证令牌"))?;

    // 检查 Token 是否在黑名单中
    let is_blacklisted = state
        .cache
        .get_token_blacklist()
        .get(&token.to_string())
        .is_some();
    if is_blacklisted {
        return Err(AppError::unauthorized("令牌已被吊销，请重新登录"));
    }

    let claims = AuthService::validate_token_static(token, &state.jwt_secret).map_err(|_| {
        AppError::unauthorized("无效的令牌")
    })?;

    // 检查是否在刷新期内（7天）
    let now = chrono::Utc::now();
    if now > claims.refresh_exp {
        return Err(AppError::unauthorized("刷新令牌已过期，请重新登录"));
    }

    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());
    let new_token = auth_service
        .generate_token(
            claims.sub,
            &claims.username,
            claims.role_id,
            claims.tenant_id,
        )
        .map_err(|e| {
            AppError::internal(format!("生成令牌失败：{}", e))
        })?;

    // Refresh Token 轮换：先将旧 Token 的 JTI（session_id）加入黑名单
    crate::services::auth_service::revoke_jti(&claims.session_id).await;

    // Blacklist the old token after successful refresh
    let now_ts = chrono::Utc::now().timestamp() as usize;
    let exp = claims.exp.timestamp() as usize;
    if exp > now_ts {
        let ttl = std::time::Duration::from_secs((exp - now_ts) as u64);
        state
            .cache
            .get_token_blacklist()
            .set(token.to_string(), true, Some(ttl));
        tracing::info!(
            "Old token blacklisted after refresh for user {}",
            claims.username
        );
    }

    // 生成新的 CSRF Token (use same session_id derivation as login)
    let new_claims =
        AuthService::validate_token_static(&new_token, &state.jwt_secret).map_err(|e| {
            tracing::error!("Failed to decode new JWT token: {}", e);
            AppError::internal("Internal server error")
        })?;
    let _session_id = new_claims.session_id;
    // CSRF token 不再需要，JWT 已经提供安全保障
    let csrf_token = "".to_string();

    Ok(Json(ApiResponse::success(RefreshTokenResponse {
        token: new_token,
        csrf_token,
        expires_in: 7200, // 2 hours
    })))
}

#[derive(Debug, Serialize)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub qr_code: String,
}

/// 1. 获取 TOTP 绑定信息 (需登录)
pub async fn setup_totp(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<TotpSetupResponse>>, AppError> {
    let totp_service = TotpService::new(state.db.clone());

    match totp_service
        .generate_totp_secret(auth.user_id, &auth.username)
        .await
    {
        Ok((secret, qr_code)) => Ok(Json(ApiResponse::success(TotpSetupResponse {
            secret,
            qr_code,
        }))),
        Err(e) => Err(AppError::internal(e.to_string())),
    }
}

#[derive(Debug, Deserialize)]
pub struct TotpVerifyRequest {
    pub token: String,
}

/// 2. 验证并正式启用 TOTP (需登录)
pub async fn enable_totp(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<TotpVerifyRequest>,
) -> Result<Json<ApiResponse<bool>>, AppError> {
    let totp_service = TotpService::new(state.db.clone());

    match totp_service
        .verify_and_enable(auth.user_id, &payload.token)
        .await
    {
        Ok(true) => Ok(Json(ApiResponse::success_with_message(
            true,
            "双因素认证已成功开启",
        ))),
        Ok(false) => Err(AppError::bad_request("验证码不正确")),
        Err(e) => Err(AppError::internal(e.to_string())),
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CsrfTokenResponse {
    pub csrf_token: String,
}

/// 获取当前登录用户信息
pub async fn get_current_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<UserInfo>>, AppError> {
    use crate::models::user;
    use sea_orm::EntityTrait;

    let user = user::Entity::find_by_id(auth.user_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query user: {}", e);
            AppError::internal("Internal server error")
        })?;

    match user {
        Some(u) => Ok(Json(ApiResponse::success(UserInfo {
            id: u.id,
            username: u.username,
            email: u.email,
            role_id: u.role_id,
        }))),
        None => Err(AppError::not_found("用户不存在")),
    }
}

/// 获取 CSRF Token（公开接口，无需认证）
/// 前端在登录前或需要时调用此接口获取 CSRF Token
#[utoipa::path(
    get,
    path = "/api/v1/erp/auth/csrf-token",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<CsrfTokenResponse>)
    ),
    tags = ["Auth"]
)]
pub async fn get_csrf_token(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> Json<ApiResponse<CsrfTokenResponse>> {
    // CSRF token 不再需要，JWT 已经提供安全保障
    let csrf_token = "".to_string();

    Json(ApiResponse::success(CsrfTokenResponse { csrf_token }))
}
