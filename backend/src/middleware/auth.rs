use crate::middleware::public_routes::is_public_path;
use crate::middleware::auth_context::AuthContext;
use crate::services::auth_service::AuthService;
use crate::utils::app_state::AppState;
use crate::utils::cache::Cache;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::{PrivateCookieJar, Key};
use tracing::warn;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    // 公共路径跳过认证
    if is_public_path(path) {
        return Ok(next.run(request).await);
    }

    // 优先从 HttpOnly Cookie 中提取 jwt，兼容 Authorization Header
    let key = Key::derive_from(state.cookie_secret.as_bytes());
    let cookie_jar = PrivateCookieJar::from_headers(request.headers(), key);
    let token_from_cookie = cookie_jar.get("jwt").map(|c| c.value().to_string());
    
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let token = if let Some(cookie_token) = token_from_cookie {
        cookie_token
    } else if let Some(header_val) = auth_header {
        if !header_val.starts_with("Bearer ") {
            warn!("无效的认证头格式");
            return Err(StatusCode::UNAUTHORIZED);
        }
        header_val[7..].to_string()
    } else {
        warn!("缺少认证凭据 (Cookie 或 Header)");
        return Err(StatusCode::UNAUTHORIZED);
    };

    if token.is_empty() {
        warn!("认证失败: 令牌为空, path={}", path);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 检查 Token 是否在黑名单中
    let is_blacklisted = state.cache.get_token_blacklist().get(&token).is_some();
    if is_blacklisted {
        warn!("认证失败: Token已被吊销, path={}", path);
        return Err(StatusCode::UNAUTHORIZED);
    }

    let mut claims = AuthService::validate_token_static(&token, &state.jwt_secret);

    // API 密钥轮换机制：如果当前密钥验证失败，且配置了 previous_jwt_secret，尝试使用旧密钥验证
    if claims.is_err() {
        warn!("JWT验证失败，尝试使用旧密钥进行平滑过渡");
        if let Some(prev_secret) = &state.previous_jwt_secret {
            claims = AuthService::validate_token_static(&token, prev_secret);
        }
    }

    match claims {
        Ok(claims) => {
            let auth_context = AuthContext::from_claims(claims);
            request.extensions_mut().insert(auth_context);
            Ok(next.run(request).await)
        }
        Err(_) => {
            warn!("认证失败: 令牌验证失败, path={}", path);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
