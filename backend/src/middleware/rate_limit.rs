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

    /// 检查是否已经被锁定（用于防暴力攻击）
    pub fn is_locked(&self, key: &str) -> bool {
        let now = Instant::now();
        if let Some(entry) = self.storage.get(key) {
            // 如果还没到重置时间，且错误次数已经达到或超过最大允许次数，则被锁定
            if now < entry.reset_at && entry.count >= self.max_requests {
                return true;
            }
        }
        false
    }

    /// 记录一次失败尝试（用于防暴力攻击）
    pub fn record_failure(&self, key: &str) {
        let now = Instant::now();
        if let Some(mut entry) = self.storage.get_mut(key) {
            if now >= entry.reset_at {
                // 已经过了上一个窗口期，重新开始计数
                entry.count = 1;
                entry.reset_at = now + self.window;
            } else {
                // 在窗口期内，增加失败次数
                entry.count += 1;
                // 如果正好达到最大次数，刷新锁定时间，使其从现在开始冻结指定的窗口期（比如15分钟）
                if entry.count == self.max_requests {
                    entry.reset_at = now + self.window;
                }
            }
        } else {
            self.storage.insert(
                key.to_string(),
                RateLimitInfo {
                    count: 1,
                    reset_at: now + self.window,
                },
            );
        }
    }

    /// 成功时重置计数（用于防暴力攻击）
    pub fn reset(&self, key: &str) {
        self.storage.remove(key);
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
    Lazy::new(|| RateLimiter::new(5, Duration::from_secs(900))); // 5次错误锁定15分钟 (900秒)

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

    // 先检查是否已经被锁定
    if rate_limiter.is_locked(&ip) {
        return Err(AppError::TooManyRequests);
    }

    // 继续处理请求，等待响应
    let response = next.run(req).await;

    // 根据响应状态码判断登录是否成功
    if response.status() == axum::http::StatusCode::UNAUTHORIZED {
        // 记录一次失败尝试
        rate_limiter.record_failure(&ip);
    } else if response.status() == axum::http::StatusCode::OK {
        // 登录成功，重置失败计数
        rate_limiter.reset(&ip);
    }

    Ok(response)
}
