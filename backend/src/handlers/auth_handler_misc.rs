//! 认证处理器：Token 刷新 / TOTP / 用户信息 / CSRF
//!
//! 拆分自 auth_handler.rs：原 refresh_token + TOTP + get_current_user + get_csrf_token 业务独立成文件。

use crate::middleware::auth_context::AuthContext;
use crate::services::auth_service::AuthService;
use crate::services::totp_service::TotpService;
use crate::utils::app_state::AppState;
use crate::utils::cache::Cache;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use super::auth_handler::UserInfo;
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::SameSite;
use serde::{Deserialize, Serialize};
use time::Duration as CookieDuration;
use utoipa::ToSchema;

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub csrf_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CsrfTokenResponse {
    pub csrf_token: String,
    pub header_name: String,
}

// 防御性 allow（Wave 3 #7：CSRF IP 绑定 + 强制轮换）：
// - redundant_clone：`token.clone()` / `csrf_token.clone()` / `new_token.clone()`
//   用于 Cookie 构建 + 缓存写入，clone 都是必要消费。
// - unused_variables：`new_claims` / `new_session_id` / `refresh_ip` 在
//   wave 3+ 接入多设备 session / IP 审计模块时若暂时不消费，保留标注。
// - needless_pass_by_value：axum extractors / Cookie::build 要求 owned，
//   无法改为引用。
// - too_many_arguments：函数签名 4 个参数（State/headers/jar/...）与
//   axum 提取器语义强绑定，拆分不会带来收益。
#[allow(
    clippy::redundant_clone,
    unused_variables,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: axum_extra::extract::PrivateCookieJar,
) -> Result<axum::response::Response, AppError> {
    // 优先从 `refresh_token` Cookie 读取（httpOnly），兼容从 Authorization 头（Bearer）传入
    let token_from_cookie = jar.get("refresh_token").map(|c| c.value().to_string());

    let token_from_header = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let token = token_from_cookie
        .or(token_from_header)
        .ok_or(AppError::unauthorized("缺少认证令牌"))?;

    // 检查 Token 是否在黑名单中
    let is_blacklisted = state
        .cache
        .get_token_blacklist()
        .get(&token)
        .is_some();
    if is_blacklisted {
        return Err(AppError::unauthorized("令牌已被吊销，请重新登录"));
    }

    let claims = AuthService::validate_token_static(&token, &state.jwt_secret)
        .map_err(|_| AppError::unauthorized("无效的令牌"))?;

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
        .map_err(|e| AppError::internal(format!("生成令牌失败：{}", e)))?;

    // Refresh Token 轮换：先将旧 Token 的 JTI（session_id）加入黑名单
    let expires_at = claims.exp.timestamp();
    crate::services::auth_service::revoke_jti(&claims.session_id, expires_at).await;

    // Blacklist the old token after successful refresh
    let now_ts = chrono::Utc::now().timestamp() as usize;
    let exp = claims.exp.timestamp() as usize;
    if exp > now_ts {
        let ttl = std::time::Duration::from_secs((exp - now_ts) as u64);
        state
            .cache
            .get_token_blacklist()
            .set(token.clone(), true, Some(ttl));
        tracing::info!(
            "Old token blacklisted after refresh for user {}",
            claims.username
        );
    }

    // 生成新的 CSRF Token (use same session_id derivation as login)
    // Wave 3 安全漏洞 #7 修复：TTL 缩短到 1800s，IP 绑定，强制轮换
    let new_claims =
        AuthService::validate_token_static(&new_token, &state.jwt_secret).map_err(|e| {
            tracing::error!("Failed to decode new JWT token: {}", e);
            AppError::internal("Internal server error")
        })?;
    let new_session_id = new_claims.session_id;

    // 提取客户端 IP（Wave 3 #7：IP 绑定到 CSRF Token）
    // 优先从 X-Real-IP / X-Forwarded-For 取（与 audit_context 中间件一致）
    let refresh_ip = headers
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("X-Real-IP")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string());

    // 强制轮换：刷新 token 时清除该 user_id 关联的旧 CSRF Token（Wave 3 #7）
    if state.cache.clear_old_csrf_token_for_user(claims.sub) {
        tracing::info!(
            user_id = claims.sub,
            username = %claims.username,
            "Token 刷新：已清除该用户的旧 CSRF Token（强制轮换）"
        );
    }

    let csrf_token = uuid::Uuid::new_v4().to_string();
    // 使用默认 TTL (CSRF_TOKEN_DEFAULT_TTL_SECS = 1800s = 30min)
    state.cache.set_csrf_token(
        csrf_token.clone(),
        new_session_id,
        refresh_ip,
        claims.sub,
        None,
    );

    // 设置新 Cookie（同时写 access_token / csrf_token / 旧版 jwt 兼容）
    let is_production =
        std::env::var("ENV").unwrap_or_else(|_| "development".to_string()) == "production";

    let new_access = axum_extra::extract::cookie::Cookie::build(("access_token", new_token.clone()))
        .path("/")
        .http_only(true)
        .secure(is_production)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::minutes(30))
        .build();
    let new_csrf = axum_extra::extract::cookie::Cookie::build(("csrf_token", csrf_token.clone()))
        .path("/")
        .http_only(false)
        .secure(is_production)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::days(7))
        .build();
    let legacy_jwt = axum_extra::extract::cookie::Cookie::build(("jwt", new_token.clone()))
        .path("/")
        .http_only(true)
        .secure(is_production)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::minutes(30))
        .build();

    let jar = jar
        .add(new_access)
        .add(new_csrf)
        .add(legacy_jwt);

    Ok((
        jar,
        Json(ApiResponse::success(RefreshTokenResponse {
            token: new_token,
            csrf_token,
            expires_in: 7200,
        })),
    )
        .into_response())
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
pub async fn get_csrf_token() -> Result<Json<ApiResponse<CsrfTokenResponse>>, AppError> {
    // 简单的 CSRF token 生成
    let csrf_token = uuid::Uuid::new_v4().to_string();
    Ok(Json(ApiResponse::success(CsrfTokenResponse {
        csrf_token,
        header_name: "X-CSRF-Token".to_string(),
    })))
}
