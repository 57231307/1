use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use std::time::Duration;
use tracing::warn;

const TIMEOUT_SECONDS: u64 = 30;

pub async fn timeout_middleware(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().clone();

    match tokio::time::timeout(Duration::from_secs(TIMEOUT_SECONDS), next.run(request)).await {
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
                    // 批次 117 P1-5 修复：原 fallback 中 `.unwrap()` 改为 `expect` + 不变量注释。
                    // `Response::builder().body(Body::empty())` 在 status 合法时永远成功，
                    // INTERNAL_SERVER_ERROR (500) 是标准状态码，此处为不变量，expect 不会触发 panic。
                    Response::builder()
                        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .expect("不变量：Response::builder with valid status 500 永远成功")
                })
        }
    }
}
