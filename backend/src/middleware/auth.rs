use crate::middleware::public_routes::is_public_path;
use crate::middleware::auth_context::AuthContext;
use crate::services::auth_service::AuthService;
use crate::utils::app_state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::{CookieJar, cookie::Key};
use tracing::warn;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    if is_public_path(path) {
        return Ok(next.run(request).await);
    }

    // 优先从 HttpOnly Cookie 中提取 jwt，兼容 Authorization Header
    let cookie_jar = CookieJar::from_headers(request.headers());
    let key = Key::derive_from(state.cookie_secret.as_bytes());
    let token_from_cookie = cookie_jar.private(&key).get("jwt").map(|c| c.value().to_string());
    
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
        warn!("令牌为空");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 检查 Token 是否在黑名单中
    let is_blacklisted = state.cache.get_token_blacklist().get(&token).await.is_some();
    if is_blacklisted {
        warn!("Token is blacklisted");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let mut claims = AuthService::validate_token_static(&token, &state.jwt_secret);
    
    // API 密钥轮换机制：如果当前密钥验证失败，且配置了 previous_jwt_secret，尝试使用旧密钥验证
    if claims.is_err() {
        if let Some(prev_secret) = &state.previous_jwt_secret {
            tracing::info!("使用新 JWT 密钥验证失败，尝试使用旧密钥进行平滑过渡");
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
            warn!("令牌验证失败");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
