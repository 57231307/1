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
                    // L-15 修复（批次 376 v13 复审）：消除 expect，改为 unwrap_or_else + 日志
                    // Response::builder with valid status 500 永远成功，理论不可达
                    tracing::error!("Response::builder REQUEST_TIMEOUT 失败（理论不可达），尝试 500 兜底");
                    Response::builder()
                        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap_or_else(|_| {
                            // 双重兜底（理论不可达）：直接返回默认响应
                            tracing::error!("Response::builder 500 也失败（理论不可达），使用默认响应");
                            Response::new(Body::from("内部错误"))
                        })
                })
        }
    }
}
