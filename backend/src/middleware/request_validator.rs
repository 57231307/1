use crate::middleware::public_routes::is_public_path;
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

    if is_public_path(path) {
        return Ok(next.run(request).await);
    }

    let x_requested_with = request.headers().get("X-Requested-With")
        .and_then(|header| header.to_str().ok());

    match x_requested_with {
        Some(value) if value == "XMLHttpRequest" => Ok(next.run(request).await),
        _ => Err(StatusCode::FORBIDDEN),
    }
}
