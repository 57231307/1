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
use tracing::warn;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    let public_paths = [
        "/health",
        "/ready",
        "/live",
        "/init",
        "/api/v1/erp/health",
        "/api/v1/erp/ready",
        "/api/v1/erp/live",
        "/api/v1/erp/init",
        "/api/v1/erp/auth/login",
        "/api/v1/erp/auth/refresh",
        "/api/v1/erp/auth/logout",
        "/api/v1/erp/dashboard",
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

            let claims = AuthService::validate_token_static(token, &state.jwt_secret);
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
