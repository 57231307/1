use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::warn;
use crate::middleware::auth_context::AuthContext;

pub async fn auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    let public_paths = [
        "/health",
        "/ready",
        "/live",
        "/api/v1/erp/auth/login",
        "/api/v1/erp/auth/refresh",
        "/api/v1/erp/auth/logout",
    ];

    if public_paths.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(auth_header) => {
            if !auth_header.starts_with("Bearer ") {
                warn!("无效的认证头格式");
                return Err(StatusCode::UNAUTHORIZED);
            }

            let token = &auth_header[7..];

            if token.is_empty() {
                warn!("令牌为空");
                return Err(StatusCode::UNAUTHORIZED);
            }

            let claims = validate_token(token);
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
        None => {
            warn!("缺少认证头");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn validate_token(token: &str) -> Result<crate::services::auth_service::AppClaims, String> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let secret_bytes = secret.into_bytes();

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 2 {
        return Err("无效的令牌格式".to_string());
    }

    let (encoded, signature) = (parts[0], parts[1]);

    let mut hasher = DefaultHasher::new();
    encoded.hash(&mut hasher);
    secret_bytes.hash(&mut hasher);
    let expected_signature = format!("{:x}", hasher.finish());

    if expected_signature != signature {
        return Err("签名验证失败".to_string());
    }

    let json = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encoded,
    ).map_err(|e| format!("{:?}", e))?;

    let claims: crate::services::auth_service::AppClaims =
        serde_json::from_slice(&json).map_err(|e| format!("{:?}", e))?;

    if claims.exp < chrono::Utc::now() {
        return Err("令牌已过期".to_string());
    }

    Ok(claims)
}
