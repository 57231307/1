//! API 网关中间件
//!
//! 提供限流、熔断、请求转换等功能
#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use crate::utils::app_state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::time::{Duration, Instant};

/// 限流存储（IP -> 请求计数）
pub struct RateLimitStore {
    requests: DashMap<String, Vec<Instant>>,
}

impl Default for RateLimitStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitStore {
    pub fn new() -> Self {
        Self {
            requests: DashMap::new(),
        }
    }

    /// 检查是否超过限流阈值
    pub fn is_allowed(&self, key: &str, max_requests: usize, window: Duration) -> bool {
        let now = Instant::now();

        // 获取或创建条目
        let mut entry = self.requests.entry(key.to_string()).or_default();

        // 清理过期的请求记录
        entry.retain(|&t| now.duration_since(t) < window);

        if entry.len() >= max_requests {
            false
        } else {
            entry.push(now);
            true
        }
    }
}

/// API 限流中间件
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从请求中获取客户端标识（IP 或 API Key）
    let client_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    // 检查限流（默认每分钟 100 请求）
    if !state
        .rate_limiter
        .is_allowed(&client_key, 100, Duration::from_secs(60))
    {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// API 版本中间件
pub async fn api_version_middleware(mut request: Request<Body>, next: Next) -> Response {
    let version = request
        .headers()
        .get("X-API-Version")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("v1")
        .to_string();

    request.extensions_mut().insert(version);
    next.run(request).await
}
