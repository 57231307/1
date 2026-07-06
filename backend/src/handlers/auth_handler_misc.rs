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

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub csrf_token: String,
    pub expires_in: u64,
}

// P3 7-17 修复：已删除 CsrfTokenResponse（仅被 get_csrf_token 使用，一并清理）

// Wave 3 安全漏洞 #7 修复：CSRF IP 绑定 + 强制轮换。
// 仅抑制 `clippy::redundant_clone`：`token.clone()` / `csrf_token.clone()`
// 用于 Cookie 构建 + 缓存写入，clone 都是必要消费。
#[allow(clippy::redundant_clone)]
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

    // M-3 修复：检查 JTI（session_id）是否已被吊销
    // 登出或用户被封禁时会吊销该用户的所有 JTI
    if crate::services::auth_service::is_jti_revoked(&claims.session_id).await {
        return Err(AppError::unauthorized("令牌已被吊销，请重新登录"));
    }

    // M-3 修复：检查用户账号是否仍处于激活状态
    // 用户被禁用后，refresh_token 也应失效，防止通过旧 token 续签
    use crate::models::user;
    use sea_orm::EntityTrait;
    let user = user::Entity::find_by_id(claims.sub)
        .one(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("刷新令牌时查询用户失败: {}", e);
            AppError::internal("服务器内部错误")
        })?;
    match user {
        Some(u) if u.is_active => {}
        _ => {
            return Err(AppError::unauthorized(
                "账号已被禁用，请联系管理员".to_string(),
            ));
        }
    }

    // 检查是否在刷新期内（7天）
    let now = chrono::Utc::now();
    if now > claims.refresh_exp {
        return Err(AppError::unauthorized("刷新令牌已过期，请重新登录"));
    }

    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());
    let new_token = auth_service
        .generate_token(claims.sub, &claims.username, claims.role_id)
        .map_err(|e| AppError::internal(format!("生成令牌失败：{}", e)))?;

    // 提取新 access_token 的 session_id，用于生成新 refresh_token（P1 7-1 修复：refresh_token 轮换）
    let new_claims_for_session =
        AuthService::validate_token_static(&new_token, &state.jwt_secret).map_err(|e| {
            tracing::error!("Failed to decode new JWT token: {}", e);
            AppError::internal("Internal server error")
        })?;
    let new_session_id = new_claims_for_session.session_id;

    // 生成新的 refresh_token（JWT 形式，与新 access_token 共享 session_id）
    let new_refresh_token = auth_service
        .generate_refresh_token(
            claims.sub,
            &claims.username,
            claims.role_id,
            &new_session_id,
        )
        .map_err(|e| AppError::internal(format!("生成刷新令牌失败：{}", e)))?;

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
    // 注：new_session_id 已在上方提取（P1 7-1 修复：refresh_token 轮换时复用）

    // 提取客户端 IP（Wave 3 #7：IP 绑定到 CSRF Token）
    // P2-12c 修复（批次 83 v1 复审）：IP 提取统一优先级（X-Real-IP → X-Forwarded-For）
    // 原实现优先 X-Forwarded-For（可被客户端伪造），且不 split/trim，与 audit_context 不一致
    // 本 handler 仅接收 HeaderMap（无 ConnectInfo extension），与 audit_context::extract_client_ip
    // 优先级保持一致：X-Real-IP → X-Forwarded-For(first, trim) → "unknown"
    let refresh_ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split(',').next().map(|s| s.trim().to_string()))
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
    // 漏洞 #12 修复：统一从 `crate::utils::config::is_production()` 读取 APP_ENV
    let is_production = crate::utils::config::is_production();

    let new_access = axum_extra::extract::cookie::Cookie::build(("access_token", new_token.clone()))
        .path("/")
        .http_only(true)
        .secure(is_production)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::minutes(30))
        .build();
    // P1 7-1 修复：refresh_token 轮换，写入新 refresh_token（JWT 形式）
    let new_refresh = axum_extra::extract::cookie::Cookie::build((
        "refresh_token",
        new_refresh_token,
    ))
    .path("/")
    .http_only(true)
    .secure(is_production)
    .same_site(SameSite::Strict)
    .max_age(CookieDuration::days(7))
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
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::minutes(30))
        .build();

    let jar = jar
        .add(new_access)
        .add(new_refresh)
        .add(new_csrf)
        .add(legacy_jwt);

    Ok((
        jar,
        Json(ApiResponse::success(RefreshTokenResponse {
            csrf_token,
            // 批次 24 v6 P1-1 修复：expires_in 从 7200 改为 1800，
            // 与上方 access_token Cookie max_age(minutes(30)) = 1800 秒对齐。
            // 原 7200 秒（2 小时）会导致前端误以为 token 有效期 2 小时，
            // 实际 30 分钟就过期，在 30min~2h 之间的请求会收到 401。
            expires_in: 1800,
            // 批次 29 v7 P0-2 修复：移除 token 字段，对齐批次 24 LoginResponse 决策。
            // access_token 通过 httpOnly Cookie 传递，前端不可读也不应读。
            // 原字段返回 access_token 会导致前端用旧 token 覆盖 Cookie 中的新 token，
            // 绕过 httpOnly 保护，增加 XSS 窃取风险。
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

/// 3. 生成 2FA 恢复码 (v11 批次 141 新增)
///
/// POST /api/v1/erp/auth/totp/recovery-codes
///
/// 生成 10 个 8 字符的恢复码（明文仅此一次返回），哈希后存入用户表。
/// 前端在 enableTotp 成功后调用，展示恢复码给用户保存。
pub async fn generate_recovery_codes(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<Vec<String>>>, AppError> {
    let totp_service = TotpService::new(state.db.clone());
    let codes = totp_service
        .generate_recovery_codes(auth.user_id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(Json(ApiResponse::success_with_message(
        codes,
        "恢复码已生成，请妥善保存（仅此一次展示）",
    )))
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
        // 批次 24 v6 P0-2 修复：使用 build_with_permissions 补全 role_name 和 permissions，
        // 解决前端刷新页面后 role_name/permissions 缺失导致路由守卫 admin 绕过失效 + 403 跳转问题。
        Some(u) => {
            let user_info = UserInfo::build_with_permissions(state.db.as_ref(), &u).await;
            Ok(Json(ApiResponse::success(user_info)))
        }
        None => Err(AppError::not_found("用户不存在")),
    }
}

// P3 7-17 修复：已删除 get_csrf_token 死代码接口
// 原实现生成 token 不存缓存，前端拿到后无法通过 CSRF 中间件校验。
// CSRF token 已通过 login/refresh 的 Set-Cookie 头下发，前端从 cookie 读取。
