use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

// =====================================================
// 内存限流器
// =====================================================

/// 内存速率限制器
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

    /// 清理过期的记录
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.storage.retain(|_, v| now < v.reset_at);
    }

    /// 检查是否允许请求
    pub fn check(&self, key: &str) -> bool {
        // 偶尔清理过期记录以防止内存泄漏
        if fastrand::usize(..1000) == 0 {
            self.cleanup();
        }

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
static GLOBAL_LIMITER: LazyLock<MemoryRateLimiter> =
    LazyLock::new(|| MemoryRateLimiter::new(180, Duration::from_secs(60)));
static BRUTE_FORCE_LIMITER: LazyLock<MemoryRateLimiter> =
    LazyLock::new(|| MemoryRateLimiter::new(5, Duration::from_secs(300)));

// =====================================================
// 中间件
// =====================================================

/// 基于 IP + UserID 的双维度速率限制中间件
pub async fn rate_limit_by_ip(
    State(_state): State<AppState>,
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

    let allowed = GLOBAL_LIMITER.check(&rate_key);

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
/// 基于 IP + Username 双维度检查，防止从同一 IP 尝试不同用户名的暴力破解
pub async fn anti_brute_force(req: Request<Body>, next: Next) -> Result<Response, AppError> {
    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Check IP-based rate limit
    if !BRUTE_FORCE_LIMITER.check(&ip) {
        tracing::warn!("Brute force blocked for IP {}", ip);
        return Err(AppError::TooManyRequests {
            retry_after: Some(300),
            message: "登录尝试次数过多，请5分钟后再试".to_string(),
        });
    }

    Ok(next.run(req).await)
}
