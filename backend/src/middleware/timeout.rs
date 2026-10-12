//! 请求超时中间件
//!
//! 到点短路时**必须**返回统一 `AppError` 信封（HTTP 408 + `code=TIMEOUT` + `trace_id` +
//! `timestamp`），不允许再手写 `Response::builder()` 裸文本体——仓库契约是「失败只有
//! `AppError` 一种形状」，前端与运维排查都按这四键解析。
//!
//! `trace_id` 由 `AppError::into_response` 从 `TRACE_ID` task-local 读取，
//! 与 `X-Trace-Id` 响应头同源（本层挂载在 `trace_context_middleware` 内层）。

use axum::{
    body::Body,
    extract::Request,
    http::Method,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Duration;

use crate::utils::error::AppError;

/// 生产默认超时阈值（秒）
const TIMEOUT_SECONDS: u64 = 30;

pub async fn timeout_middleware(request: Request<Body>, next: Next) -> Response {
    timeout_middleware_with(Duration::from_secs(TIMEOUT_SECONDS), request, next).await
}

/// [`timeout_middleware`] 的可注入阈值版本：真实短路逻辑只有一份，
/// 生产路径固定用 [`TIMEOUT_SECONDS`]，契约测试用毫秒级阈值验证 408 出参形状
/// （不是为了「让测试过」而另造一套响应构造）。
pub async fn timeout_middleware_with(
    timeout: Duration,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().clone();

    match tokio::time::timeout(timeout, next.run(request)).await {
        Ok(response) => response,
        Err(_) => timeout_error_response(&method, &path, timeout).into_response(),
    }
}

/// 超时短路的**唯一**构造点：返回 `AppError`（408 / `TIMEOUT`）。
///
/// 详情（方法 / 路径 / 阈值）只进 tracing 日志，HTTP 出参是固定脱敏文案 + 同源 trace_id，
/// 序列化与 `X-Trace-Id` 响应头补齐都由 `AppError::into_response` 统一完成。
pub fn timeout_error_response(method: &Method, path: &str, timeout: Duration) -> AppError {
    AppError::timeout(format!(
        "请求处理超过 {}ms 被超时中间件短路：method={} path={}",
        timeout.as_millis(),
        method,
        path
    ))
}
