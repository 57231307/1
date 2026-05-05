use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
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
use tracing::{info, warn};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();

    if is_public_path(&path) {
        return Ok(next.run(request).await);
    }

    let token = match extract_token(&request) {
        Some(t) => t,
        None => {
            warn!("[AUTH_MW] 缺少认证令牌: path={}", path);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    if state.cache.get_token_blacklist().get(&token).is_some() {
        warn!("[AUTH_MW] 令牌已被撤销: path={}", path);
        return Err(StatusCode::UNAUTHORIZED);
    }

    let claims = match AuthService::validate_token_static(&token, &state.jwt_secret) {
        Ok(claims) => claims,
        Err(_) => {
            match state.previous_jwt_secret.as_ref() {
                Some(prev_secret) => match AuthService::validate_token_static(&token, prev_secret) {
                    Ok(claims) => claims,
                    Err(_) => {
                        warn!("[AUTH_MW] 无效的令牌(新旧密钥均验证失败): path={}", path);
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                },
                None => {
                    warn!("[AUTH_MW] 无效的令牌: path={}", path);
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    };

    info!("[AUTH_MW] 认证成功: user_id={}, username={}, role_id={:?}", claims.sub, claims.username, claims.role_id);

    let auth_context = AuthContext::from_claims(claims);
    request.extensions_mut().insert(auth_context);

    let response = next.run(request).await;
    Ok(response)
}

fn extract_token(request: &Request<Body>) -> Option<String> {
    if let Some(auth_header) = request.headers().get("Authorization") {
        if let Ok(header_value) = auth_header.to_str() {
            if let Some(token) = header_value.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    if let Some(cookie_header) = request.headers().get("Cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for part in cookie_str.split(';') {
                let trimmed = part.trim();
                if let Some(value) = trimmed.strip_prefix("jwt=") {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}
