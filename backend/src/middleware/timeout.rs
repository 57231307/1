use std::time::Duration;
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use tracing::warn;

const TIMEOUT_SECONDS: u64 = 30;

pub async fn timeout_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().clone();

    match tokio::time::timeout(
        Duration::from_secs(TIMEOUT_SECONDS),
        next.run(request),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => {
            warn!(
                method = %method,
                path = %path,
                timeout_secs = TIMEOUT_SECONDS,
                "请求超时"
            );
            Response::builder()
                .status(axum::http::StatusCode::REQUEST_TIMEOUT)
                .body(Body::from("请求超时"))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap()
                })
        }
    }
}
