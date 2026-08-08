//! 认证处理器：Token 刷新 / TOTP / 用户信息 / CSRF
//!
//! 拆分自 auth_handler.rs：原 refresh_token + TOTP + get_current_user + get_csrf_token 业务独立成文件。

use super::auth_handler::UserInfo;
use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::auth_service::AuthService;
use crate::services::totp_service::TotpService;
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
use serde::{Deserialize, Serialize};
use time::Duration as CookieDuration;

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub csrf_token: String,
    pub expires_in: u64,
}

// P3 7-17 修复：已删除 CsrfTokenResponse（仅被 get_csrf_token 使用，一并清理）

// Wave 3 安全漏洞 #7 修复：CSRF IP 绑定 + 强制轮换；P1 7-1 修复：refresh_token 轮换
pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: axum_extra::extract::PrivateCookieJar,
) -> Result<axum::response::Response, AppError> {
    let token = extract_refresh_token(&state, &headers, &jar)?;
    let claims = validate_refresh_claims(&state, &token).await?;

    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());
    let (new_token, new_session_id, new_refresh_token) =
        generate_new_tokens(&state, &auth_service, &claims)?;
    revoke_old_token(&state, &token, &claims).await;

    let refresh_ip = extract_client_ip_from_headers(&headers);
    let csrf_token = rotate_csrf_token(&state, &claims, new_session_id, refresh_ip);
    let jar = build_refresh_cookies(jar, &new_token, &new_refresh_token, &csrf_token);

    Ok((
        jar,
        Json(ApiResponse::success(RefreshTokenResponse {
            csrf_token,
            // 与 access_token Cookie max_age(minutes(30)) = 1800 秒对齐
            expires_in: 1800,
        })),
    )
        .into_response())
}

// 从 refresh_token Cookie 或 Authorization Bearer 头提取令牌，并检查黑名单
fn extract_refresh_token(
    state: &AppState,
    headers: &HeaderMap,
    jar: &axum_extra::extract::PrivateCookieJar,
) -> Result<String, AppError> {
    let token_from_cookie = jar.get("refresh_token").map(|c| c.value().to_string());
    let token_from_header = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    let token = token_from_cookie
        .or(token_from_header)
        .ok_or(AppError::unauthorized("缺少认证令牌"))?;
    if state.cache.get_token_blacklist().get(&token).is_some() {
        return Err(AppError::unauthorized("令牌已被吊销，请重新登录"));
    }
    Ok(token)
}

// 验证 token 签名、JTI 吊销状态、用户活跃状态、刷新期有效性
async fn validate_refresh_claims(
    state: &AppState,
    token: &str,
) -> Result<crate::services::auth_service::AppClaims, AppError> {
    let claims = AuthService::validate_token_static(token, &state.jwt_secret)
        .map_err(|_| AppError::unauthorized("无效的令牌"))?;
    if crate::services::auth_service::is_jti_revoked(&claims.session_id).await {
        return Err(AppError::unauthorized("令牌已被吊销，请重新登录"));
    }
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
            ))
        }
    }
    if chrono::Utc::now() > claims.refresh_exp {
        return Err(AppError::unauthorized("刷新令牌已过期，请重新登录"));
    }
    Ok(claims)
}

// 生成新的 access_token 和 refresh_token（共享 session_id）
fn generate_new_tokens(
    state: &AppState,
    auth_service: &AuthService,
    claims: &crate::services::auth_service::AppClaims,
) -> Result<(String, String, String), AppError> {
    let new_token = auth_service
        .generate_token(claims.sub, &claims.username, claims.role_id)
        .map_err(|e| AppError::internal(format!("生成令牌失败：{}", e)))?;
    let new_claims =
        AuthService::validate_token_static(&new_token, &state.jwt_secret).map_err(|e| {
            tracing::error!("Failed to decode new JWT token: {}", e);
            AppError::internal("Internal server error")
        })?;
    let new_session_id = new_claims.session_id;
    let new_refresh_token = auth_service
        .generate_refresh_token(
            claims.sub,
            &claims.username,
            claims.role_id,
            &new_session_id,
        )
        .map_err(|e| AppError::internal(format!("生成刷新令牌失败：{}", e)))?;
    Ok((new_token, new_session_id, new_refresh_token))
}

// 吊销旧 token：将 JTI 加入黑名单 + 将 token 加入黑名单缓存
async fn revoke_old_token(
    state: &AppState,
    token: &str,
    claims: &crate::services::auth_service::AppClaims,
) {
    let expires_at = claims.exp.timestamp();
    crate::services::auth_service::revoke_jti(&claims.session_id, expires_at).await;
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
}

// 提取客户端 IP：X-Real-IP → X-Forwarded-For(first, trim) → "unknown"
fn extract_client_ip_from_headers(headers: &HeaderMap) -> String {
    headers
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
        .unwrap_or_else(|| "unknown".to_string())
}

// 强制轮换 CSRF Token：清除旧 token + 写入新 token（IP 绑定，TTL 1800s）
fn rotate_csrf_token(
    state: &AppState,
    claims: &crate::services::auth_service::AppClaims,
    new_session_id: String,
    refresh_ip: String,
) -> String {
    if state.cache.clear_old_csrf_token_for_user(claims.sub) {
        tracing::info!(
            user_id = claims.sub,
            username = %claims.username,
            "Token 刷新：已清除该用户的旧 CSRF Token（强制轮换）"
        );
    }
    let csrf_token = uuid::Uuid::new_v4().to_string();
    state.cache.set_csrf_token(
        csrf_token.clone(),
        new_session_id,
        refresh_ip,
        claims.sub,
        None,
    );
    csrf_token
}

// 构建刷新响应 Cookie：access_token / refresh_token / csrf_token。
// B03-P2-1 修复：已移除 legacy "jwt" Cookie 双写，仅刷新 access_token，避免双 Cookie 鉴权不一致。
fn build_refresh_cookies(
    jar: axum_extra::extract::PrivateCookieJar,
    new_token: &str,
    new_refresh_token: &str,
    csrf_token: &str,
) -> axum_extra::extract::PrivateCookieJar {
    let is_production = crate::utils::config::is_production();
    let new_access =
        axum_extra::extract::cookie::Cookie::build(("access_token", new_token.to_string()))
            .path("/")
            .http_only(true)
            .secure(is_production)
            .same_site(SameSite::Strict)
            .max_age(CookieDuration::minutes(30))
            .build();
    let new_refresh = axum_extra::extract::cookie::Cookie::build((
        "refresh_token",
        new_refresh_token.to_string(),
    ))
    .path("/")
    .http_only(true)
    .secure(is_production)
    .same_site(SameSite::Strict)
    .max_age(CookieDuration::days(2))
    .build();
    let new_csrf =
        axum_extra::extract::cookie::Cookie::build(("csrf_token", csrf_token.to_string()))
            .path("/")
            .http_only(false)
            .secure(is_production)
            .same_site(SameSite::Strict)
            .max_age(CookieDuration::days(7))
            .build();
    jar.add(new_access).add(new_refresh).add(new_csrf)
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

/// P1-08-1：用户确认同意用户协议与隐私政策；记录同意时间到 users.agreed_to_terms_at，满足《个人信息保护法》第 14 条同意要求。
#[derive(Debug, Deserialize)]
pub struct AgreeToTermsRequest {
    pub user_agreement_version: Option<String>,
    pub privacy_policy_version: Option<String>,
}

pub async fn agree_to_terms(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<AgreeToTermsRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::user::{self, ActiveModel, Column};
    use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

    let now = chrono::Utc::now();
    let update_result = user::Entity::update_many()
        .filter(Column::Id.eq(auth.user_id))
        .set(ActiveModel {
            agreed_to_terms_at: ActiveValue::Set(Some(now)),
            ..Default::default()
        })
        .exec(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to update terms agreement: {}", e);
            AppError::internal("更新用户协议同意状态失败")
        })?;

    if update_result.rows_affected == 0 {
        return Err(AppError::not_found("用户不存在"));
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "agreed": true,
        "agreed_at": now.to_rfc3339(),
        "user_agreement_version": req.user_agreement_version,
        "privacy_policy_version": req.privacy_policy_version,
    }))))
}

// P3 7-17 修复：已删除 get_csrf_token 死代码接口
// 原实现生成 token 不存缓存，前端拿到后无法通过 CSRF 中间件校验。
// CSRF token 已通过 login/refresh 的 Set-Cookie 头下发，前端从 cookie 读取。
