use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn request_validator_middleware(
    request: Request<Body>,
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
    ];

    if public_paths.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(request).await);
    }

    let x_requested_with = request
        .headers()
        .get("X-Requested-With")
        .and_then(|header| header.to_str().ok());

    match x_requested_with {
        Some(value) if value == "XMLHttpRequest" => Ok(next.run(request).await),
        _ => Err(StatusCode::FORBIDDEN),
    }
}
