//! API 网关中间件
//!
//! 提供限流、熔断、请求转换等功能

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


