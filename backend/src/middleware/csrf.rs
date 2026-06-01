use crate::utils::app_state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Method, Request},
    middleware::Next,
    response::Response,
};
use tracing::debug;

/// CSRF 中间件
///
/// 对于 API 后端 + JWT 认证的架构，CSRF 保护已经通过 JWT 实现：
/// 1. JWT Token 存储在 HttpOnly Cookie 中，JavaScript 无法读取
/// 2. 或者通过 Authorization Header 发送，需要 CORS 预检
///
/// 因此，此中间件仅记录日志，不阻止请求
pub async fn csrf_middleware(
    State(_state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method();
    let path = request.uri().path();

    // 只对状态变更方法记录日志
    if method == Method::POST
        || method == Method::PUT
        || method == Method::DELETE
        || method == Method::PATCH
    {
        // 检查是否有认证信息
        let has_auth = request
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.starts_with("Bearer "))
            .unwrap_or(false);

        let has_cookie = request
            .headers()
            .get("cookie")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.contains("jwt="))
            .unwrap_or(false);

        if !has_auth && !has_cookie {
            debug!("未认证的状态变更请求: {} {}", method, path);
        }
    }

    Ok(next.run(request).await)
}
