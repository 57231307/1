use crate::middleware::public_routes::is_public_path;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn request_validator_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    if is_public_path(path) {
        return Ok(next.run(request).await);
    }

    let method = request.method().clone();
    if !is_state_changing_method(&method) {
        return Ok(next.run(request).await);
    }

    let origin = request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok());

    let referer = request
        .headers()
        .get("referer")
        .and_then(|v| v.to_str().ok());

    let allowed_origins = vec![
        "http://localhost:3000",
        "http://localhost:8080",
        "https://erp.example.com",
    ];

    let is_valid_origin = origin.map(|o| {
        allowed_origins.iter().any(|allowed| {
            o.starts_with(*allowed) || *allowed == "*"
        })
    }).unwrap_or(false);

    let is_valid_referer = referer.map(|r| {
        allowed_origins.iter().any(|allowed| {
            r.starts_with(*allowed) || *allowed == "*"
        })
    }).unwrap_or(false);

    if !is_valid_origin && !is_valid_referer {
        tracing::warn!("CSRF验证失败: 非法来源 origin={:?}, referer={:?}", origin, referer);
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

fn is_state_changing_method(method: &axum::http::Method) -> bool {
    matches!(
        method,
        axum::http::Method::POST |
        axum::http::Method::PUT |
        axum::http::Method::PATCH |
        axum::http::Method::DELETE
    )
}
