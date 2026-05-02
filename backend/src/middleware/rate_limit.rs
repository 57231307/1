use axum::{body::Body, http::Request, middleware::Next, response::Response};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::net::SocketAddr;
use dashmap::DashMap;
use crate::utils::error::AppError;
use once_cell::sync::Lazy;
use crate::middleware::auth_context::AuthContext;

/// 企业级 Redis 速率限制预留抽象（当前回退为内存实现以适配本地环境）
/// TODO(v1.1): 将 storage 切换为 deadpool-redis 连接池以支持分布式双维度限流
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
            if now >= entry.reset_at {
                entry.count = 1;
                entry.reset_at = now + self.window;
                return true;
            } else {
                entry.count += 1;
                return entry.count <= self.max_requests;
            }
        } else {
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
}

// 全局 100 次/分钟限制
static GLOBAL_LIMITER: Lazy<RateLimiter> = Lazy::new(|| RateLimiter::new(100, Duration::from_secs(60)));
static BRUTE_FORCE_LIMITER: Lazy<RateLimiter> = Lazy::new(|| RateLimiter::new(5, Duration::from_secs(300)));

/// 基于 IP + UserID 的双维度速率限制中间件
pub async fn rate_limit_by_ip(
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown_ip".to_string());

    let user_id = req
        .extensions()
        .get::<AuthContext>()
        .map(|auth| auth.user_id.to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    let rate_key = format!("rate:{}:{}", ip, user_id);

    if !GLOBAL_LIMITER.check(&rate_key) {
        tracing::warn!("Rate limit exceeded for {}", rate_key);
        return Err(AppError::TooManyRequests);
    }

    Ok(next.run(req).await)
}

/// 防暴力攻击中间件（针对登录端点）
pub async fn anti_brute_force(
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if !BRUTE_FORCE_LIMITER.check(&ip) {
        tracing::warn!("Brute force blocked for IP {}", ip);
        return Err(AppError::TooManyRequests);
    }

    Ok(next.run(req).await)
}
