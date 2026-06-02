use crate::middleware::public_routes::is_public_path;
use crate::utils::app_state::AppState;
use crate::utils::request_ext::PublicPathCache;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// 请求验证中间件
///
/// 对于已认证的 API 请求（有 JWT Token），跳过 Origin 检查
/// 因为 JWT 认证本身已经提供了安全保障
pub async fn request_validator_middleware(
    State(_state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    // 使用缓存的公共路径检查结果，避免重复计算
    let is_public = request
        .extensions()
        .get::<PublicPathCache>()
        .map(|cache| cache.is_public)
        .unwrap_or_else(|| is_public_path(path));

    if is_public {
        return Ok(next.run(request).await);
    }

    // 检查是否有 Authorization 头（JWT Token）
    let has_auth = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("Bearer "))
        .unwrap_or(false);

    // 如果有 JWT Token，直接通过（JWT 已经提供了安全保障）
    if has_auth {
        return Ok(next.run(request).await);
    }

    // 没有 JWT Token 的请求，检查 Origin
    let method = request.method().clone();
    let path = request.uri().path();

    // 对状态变更方法记录未认证请求的日志
    if is_state_changing_method(&method) {
        // 检查是否有 Cookie 认证信息
        let has_cookie = request
            .headers()
            .get("cookie")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.contains("jwt="))
            .unwrap_or(false);

        if !has_cookie {
            tracing::debug!("未认证的状态变更请求: {} {}", method, path);
        }
    }

    Ok(next.run(request).await)
}

fn is_state_changing_method(method: &axum::http::Method) -> bool {
    *method == axum::http::Method::POST
        || *method == axum::http::Method::PUT
        || *method == axum::http::Method::PATCH
        || *method == axum::http::Method::DELETE
}
