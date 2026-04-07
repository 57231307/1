use crate::utils::error::AppError;
use axum::{body::Body, http::Request, middleware::Next, response::Response};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 速率限制器
#[derive(Clone, Debug)]
pub struct RateLimiter {
    storage: Arc<DashMap<String, RateLimitInfo>>,
    max_requests: usize,
    window: Duration,
}

/// 速率限制信息
#[derive(Debug)]
struct RateLimitInfo {
    count: usize,
    reset_at: Instant,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            storage: Arc::new(DashMap::new()),
            max_requests,
            window,
        }
    }

    /// 检查是否允许请求
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        if let Some(mut entry) = self.storage.get_mut(key) {
            // 检查是否需要重置
            if now >= entry.reset_at {
                // 重置计数
                entry.count = 1;
                entry.reset_at = now + self.window;
                return true;
            } else {
                // 增加计数
                entry.count += 1;
                return entry.count <= self.max_requests;
            }
        } else {
            // 新条目
            self.storage.insert(
                key.to_string(),
                RateLimitInfo {
                    count: 1,
                    reset_at: now + self.window,
                },
            );
            return true;
        }
    }

    /// 清理过期的条目
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.storage.retain(|_, info| now < info.reset_at);
    }
}

static RATE_LIMITER: Lazy<RateLimiter> =
    Lazy::new(|| RateLimiter::new(100, Duration::from_secs(60)));
static USER_RATE_LIMITER: Lazy<RateLimiter> =
    Lazy::new(|| RateLimiter::new(50, Duration::from_secs(60)));
static BRUTE_FORCE_LIMITER: Lazy<RateLimiter> =
    Lazy::new(|| RateLimiter::new(5, Duration::from_secs(300)));

/// 基于IP的速率限制中间件
pub async fn rate_limit_by_ip(req: Request<Body>, next: Next) -> Result<Response, AppError> {
    // 从请求中获取IP地址
    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // 使用全局速率限制器
    let rate_limiter = &RATE_LIMITER;

    // 检查速率限制
    if !rate_limiter.check(&ip) {
        return Err(AppError::TooManyRequests);
    }

    // 继续处理请求
    Ok(next.run(req).await)
}

/// 基于用户ID的速率限制中间件
pub async fn rate_limit_by_user(req: Request<Body>, next: Next) -> Result<Response, AppError> {
    // 从请求中获取用户ID（这里需要根据实际的认证机制来实现）
    // 暂时使用IP作为替代
    let user_id = req
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // 使用全局速率限制器
    let rate_limiter = &USER_RATE_LIMITER;

    // 检查速率限制
    if !rate_limiter.check(&user_id) {
        return Err(AppError::TooManyRequests);
    }

    // 继续处理请求
    Ok(next.run(req).await)
}

/// 防暴力攻击中间件（针对登录端点）
pub async fn anti_brute_force(req: Request<Body>, next: Next) -> Result<Response, AppError> {
    // 从请求中获取IP地址
    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // 使用全局防暴力攻击限制器
    let rate_limiter = &BRUTE_FORCE_LIMITER;

    // 检查速率限制
    if !rate_limiter.check(&ip) {
        return Err(AppError::TooManyRequests);
    }

    // 继续处理请求
    Ok(next.run(req).await)
}
