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

    // 精简后的绝对白名单，只保留基础设施探针和必需的登录逻辑
    let public_paths = [
        "/health",
        "/ready",
        "/live",
        "/api/v1/erp/health",
        "/api/v1/erp/ready",
        "/api/v1/erp/live",
        "/api/v1/erp/auth/login",
        "/api/v1/erp/init", // 初始化接口由内部数据库状态锁死保护
        "/api/v1/erp/init/status",
    ];

    // 精确匹配或允许前缀匹配 init 路由
    if public_paths.iter().any(|&p| path == p) || path.starts_with("/api/v1/erp/init/") {
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
