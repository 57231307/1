use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::net::SocketAddr;
use dashmap::DashMap;
use crate::utils::error::AppError;
use once_cell::sync::Lazy;
use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;

// =====================================================
// Redis 分布式限流器
// =====================================================

/// Redis 分布式速率限制器
pub struct RedisRateLimiter {
    pool: deadpool_redis::Pool,
    max_requests: usize,
    window_secs: u64,
}

impl RedisRateLimiter {
    pub fn new(redis_url: &str, max_requests: usize, window_secs: u64) -> Result<Self, String> {
        let cfg = deadpool_redis::Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .map_err(|e| format!("Redis 连接池创建失败: {}", e))?;

        Ok(Self {
            pool,
            max_requests,
            window_secs,
        })
    }

    /// 检查是否允许请求
    pub async fn check(&self, key: &str) -> Result<bool, AppError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::InternalError(format!("Redis 连接获取失败: {}", e)))?;

        let redis_key = format!("rate_limit:{}", key);
        let count: i64 = redis::AsyncCommands::incr(&mut conn, &redis_key, 1)
            .await
            .map_err(|e| AppError::InternalError(format!("Redis 操作失败: {}", e)))?;

        if count == 1 {
            let _: () = redis::AsyncCommands::expire(&mut conn, &redis_key, self.window_secs as i64)
                .await
                .map_err(|e| AppError::InternalError(format!("Redis 过期设置失败: {}", e)))?;
        }

        Ok(count <= self.max_requests as i64)
    }
}

// =====================================================
// 内存限流器（回退方案）
// =====================================================

/// 内存速率限制器（用于无 Redis 环境回退）
#[derive(Clone, Debug)]
pub struct MemoryRateLimiter {
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

impl MemoryRateLimiter {
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
                true
            } else {
                entry.count += 1;
                entry.count <= self.max_requests
            }
        } else {
            self.storage.insert(
                key.to_string(),
                RateLimitInfo {
                    count: 1,
                    reset_at: now + self.window,
                },
            );
            true
        }
    }
}

// 全局内存限流器实例（回退使用）
static GLOBAL_LIMITER: Lazy<MemoryRateLimiter> =
    Lazy::new(|| MemoryRateLimiter::new(100, Duration::from_secs(60)));
static BRUTE_FORCE_LIMITER: Lazy<MemoryRateLimiter> =
    Lazy::new(|| MemoryRateLimiter::new(5, Duration::from_secs(300)));

// =====================================================
// 中间件
// =====================================================

/// 基于 IP + UserID 的双维度速率限制中间件
/// 优先使用 Redis 分布式限流，如 Redis 不可用则回退到内存限流
pub async fn rate_limit_by_ip(
    State(state): State<AppState>,
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

    // 优先尝试 Redis 限流
    let allowed = if let Some(redis_limiter) = state.redis_limiter.as_ref() {
        match redis_limiter.check(&rate_key).await {
            Ok(allowed) => allowed,
            Err(e) => {
                tracing::warn!("Redis 限流检查失败，回退到内存限流: {}", e);
                GLOBAL_LIMITER.check(&rate_key)
            }
        }
    } else {
        // 无 Redis 配置，使用内存限流
        GLOBAL_LIMITER.check(&rate_key)
    };

    if !allowed {
        tracing::warn!("Rate limit exceeded for {}", rate_key);
        return Err(AppError::TooManyRequests {
            retry_after: Some(60),
            message: "请求过于频繁".to_string(),
        });
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
        return Err(AppError::TooManyRequests {
            retry_after: Some(300),
            message: "登录尝试次数过多，请5分钟后再试".to_string(),
        });
    }

    Ok(next.run(req).await)
}
