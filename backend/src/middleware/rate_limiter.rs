//! API 限流中间件
//! 防止恶意刷接口，保护系统资源
//! 使用滑动窗口算法实现限流

use axum::{
    body::Body,
    extract::{State},
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// 限流配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 时间窗口（秒）
    pub window_size_secs: u64,
    /// 时间窗口内允许的最大请求数
    pub max_requests: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            window_size_secs: 60, // 默认 1 分钟
            max_requests: 100,    // 默认 100 次请求
        }
    }
}

/// 限流器状态
#[derive(Debug, Clone)]
pub struct RateLimiterState {
    /// 客户端 IP 的请求记录
    requests: DashMap<String, Vec<Instant>>,
    /// 限流配置
    config: RateLimitConfig,
}

impl RateLimiterState {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            requests: DashMap::new(),
            config,
        }
    }

    /// 检查请求是否允许
    pub fn check_rate_limit(&self, client_ip: &str) -> Result<(), RateLimitError> {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(self.config.window_size_secs);

        // 获取或创建该 IP 的请求记录
        let mut requests = self
            .requests
            .entry(client_ip.to_string())
            .or_insert_with(Vec::new);

        // 清理过期的请求记录
        requests.retain(|&timestamp| timestamp > window_start);

        // 检查是否超过限制
        if requests.len() as u32 >= self.config.max_requests {
            return Err(RateLimitError::TooManyRequests {
                retry_after: self.config.window_size_secs,
            });
        }

        // 记录当前请求
        requests.push(now);

        Ok(())
    }

    /// 清理过期的请求记录（定期调用）
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(self.config.window_size_secs);

        self.requests.retain(|_, timestamps| {
            timestamps.retain(|&timestamp| timestamp > window_start);
            !timestamps.is_empty()
        });
    }
}

/// 限流错误
#[derive(Debug, Clone)]
pub enum RateLimitError {
    TooManyRequests { retry_after: u64 },
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::TooManyRequests { retry_after } => {
                write!(f, "请求过于频繁，请在 {} 秒后重试", retry_after)
            }
        }
    }
}

/// 限流中间件
pub async fn rate_limiter_middleware(
    State(state): State<Arc<RateLimiterState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 提取客户端 IP
    let client_ip = extract_client_ip(&request);

    // 检查限流
    match state.check_rate_limit(&client_ip) {
        Ok(_) => {
            // 允许请求
            Ok(next.run(request).await)
        }
        Err(RateLimitError::TooManyRequests { retry_after }) => {
            // 返回 429 错误
            let mut response = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Retry-After", retry_after.to_string())
                .header("X-RateLimit-Limit", state.config.max_requests.to_string())
                .header("X-RateLimit-Window", state.config.window_size_secs.to_string())
                .body(Body::from(
                    r#"{"success":false,"error":"请求过于频繁","retry_after":"#
                        .to_string()
                        + &retry_after.to_string()
                        + r#"}"#,
                ))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(response)
        }
    }
}

/// 从请求中提取客户端 IP
fn extract_client_ip(request: &Request<Body>) -> String {
    // 尝试从 X-Forwarded-For 头获取
    if let Some(forwarded) = request.headers().get("X-Forwarded-For") {
        if let Ok(value) = forwarded.to_str() {
            // 取第一个 IP（最外层客户端）
            if let Some(ip) = value.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // 尝试从 X-Real-IP 头获取
    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(value) = real_ip.to_str() {
            return value.trim().to_string();
        }
    }

    // 从连接地址获取
    if let Some(addr) = request.extensions().get::<std::net::SocketAddr>() {
        return addr.ip().to_string();
    }

    // 默认返回
    "unknown".to_string()
}

/// 创建限流器中间件（带默认配置）
pub fn create_rate_limiter() -> Arc<RateLimiterState> {
    Arc::new(RateLimiterState::new(RateLimitConfig::default()))
}

/// 创建限流器中间件（自定义配置）
pub fn create_rate_limiter_with_config(config: RateLimitConfig) -> Arc<RateLimiterState> {
    Arc::new(RateLimiterState::new(config))
}

/// 启动定期清理任务
pub async fn start_cleanup_task(state: Arc<RateLimiterState>, cleanup_interval_secs: u64) {
    loop {
        sleep(Duration::from_secs(cleanup_interval_secs)).await;
        state.cleanup_expired();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let config = RateLimitConfig {
            window_size_secs: 60,
            max_requests: 100,
        };
        let state = RateLimiterState::new(config);

        assert_eq!(state.config.window_size_secs, 60);
        assert_eq!(state.config.max_requests, 100);
    }

    #[test]
    fn test_rate_limiter_default_config() {
        let config = RateLimitConfig::default();

        assert_eq!(config.window_size_secs, 60);
        assert_eq!(config.max_requests, 100);
    }

    #[test]
    fn test_rate_limit_within_limit() {
        let config = RateLimitConfig {
            window_size_secs: 60,
            max_requests: 5,
        };
        let state = RateLimiterState::new(config);

        // 前 5 次请求应该成功
        for i in 0..5 {
            let result = state.check_rate_limit("192.168.1.1");
            assert!(result.is_ok(), "第 {} 次请求应该成功", i + 1);
        }
    }

    #[test]
    fn test_rate_limit_exceeded() {
        let config = RateLimitConfig {
            window_size_secs: 60,
            max_requests: 3,
        };
        let state = RateLimiterState::new(config);

        // 前 3 次请求成功
        for _ in 0..3 {
            assert!(state.check_rate_limit("192.168.1.2").is_ok());
        }

        // 第 4 次应该失败
        let result = state.check_rate_limit("192.168.1.2");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::TooManyRequests { .. }
        ));
    }

    #[test]
    fn test_rate_limit_different_ips() {
        let config = RateLimitConfig {
            window_size_secs: 60,
            max_requests: 2,
        };
        let state = RateLimiterState::new(config);

        // IP1 的 2 次请求
        assert!(state.check_rate_limit("192.168.1.1").is_ok());
        assert!(state.check_rate_limit("192.168.1.1").is_ok());

        // IP2 的请求应该独立计数
        assert!(state.check_rate_limit("192.168.1.2").is_ok());
        assert!(state.check_rate_limit("192.168.1.2").is_ok());

        // IP1 再次请求应该失败
        assert!(state.check_rate_limit("192.168.1.1").is_err());
    }

    #[test]
    fn test_error_display() {
        let error = RateLimitError::TooManyRequests { retry_after: 60 };
        let msg = error.to_string();

        assert!(msg.contains("请求过于频繁"));
        assert!(msg.contains("60"));
    }

    #[test]
    fn test_cleanup_expired() {
        let config = RateLimitConfig {
            window_size_secs: 1, // 1 秒窗口
            max_requests: 10,
        };
        let state = Arc::new(RateLimiterState::new(config));

        // 添加一些请求
        state.check_rate_limit("192.168.1.1").unwrap();
        state.check_rate_limit("192.168.1.1").unwrap();

        // 立即清理，应该没有变化
        state.cleanup_expired();
        assert_eq!(state.requests.len(), 1);

        // 等待窗口过期
        std::thread::sleep(Duration::from_secs(2));
        state.cleanup_expired();

        // 清理后应该为空
        assert_eq!(state.requests.len(), 0);
    }

    #[test]
    fn test_extract_client_ip_from_forwarded() {
        use axum::http::{Request, header};

        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.headers_mut().insert(
            "X-Forwarded-For",
            "203.0.113.195, 70.41.3.18, 150.172.238.178".parse().unwrap(),
        );

        let ip = extract_client_ip(&req);
        assert_eq!(ip, "203.0.113.195");
    }

    #[test]
    fn test_extract_client_ip_from_real_ip() {
        use axum::http::{Request, header};

        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.headers_mut()
            .insert("X-Real-IP", "192.168.1.100".parse().unwrap());

        let ip = extract_client_ip(&req);
        assert_eq!(ip, "192.168.1.100");
    }

    #[test]
    fn test_extract_client_ip_default() {
        use axum::http::Request;

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let ip = extract_client_ip(&req);
        assert_eq!(ip, "unknown");
    }
}
