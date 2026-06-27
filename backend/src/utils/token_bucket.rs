//! 令牌桶限流算法（P4-2 安全加固）
//!
//! ## 与固定计数窗口的对比
//!
//! 现有 `rate_limit.rs` 使用固定窗口计数（窗口重置时清零），在窗口边界
//! 可能出现"2 倍突发"。令牌桶（Token Bucket）允许**一定程度的突发**，
//! 同时保证**长期平均速率**不超过配置值，更适合生产环境 API 限流。
//!
//! ## 算法
//!
//! ```text
//! 桶容量 = burst
//! 填充速率 = burst / window  (每秒补充的令牌数)
//! 当前令牌数 = min(桶容量, 当前令牌 + (now - last) * 速率)
//! 每次请求消耗 1 个令牌；令牌 < 1 时拒绝
//! ```
//!
//! ## 多租户隔离
//!
//! key 格式：`tenant:{tenant_id}:{scope}` 或 `ip:{ip_addr}:{scope}`

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 令牌桶（预留 API，待限流中间件接入）
#[allow(dead_code)] // TODO(tech-debt): 限流中间件接入后移除
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// 桶容量（最大突发）
    pub capacity: f64,
    /// 填充速率（每秒）
    pub refill_rate: f64,
    /// 上次更新时间
    pub last_refill: Instant,
    /// 当前令牌数
    pub tokens: f64,
}

impl TokenBucket {
    /// 创建新桶（满载）
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            refill_rate,
            last_refill: Instant::now(),
            tokens: capacity,
        }
    }

    /// 尝试获取 1 个令牌
    pub fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// 补充令牌（自上次更新到现在的时长 × 速率）
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    /// 当前可用令牌数（用于 X-RateLimit-Remaining 响应头）
    pub fn available(&mut self) -> f64 {
        self.refill();
        self.tokens
    }
}

/// 令牌桶限流器（预留 API，待限流中间件接入）
#[allow(dead_code)] // TODO(tech-debt): 限流中间件接入后移除
#[derive(Clone, Debug)]
pub struct TokenBucketLimiter {
    storage: Arc<DashMap<String, TokenBucket>>,
    /// 桶容量
    pub capacity: f64,
    /// 时间窗（用于计算 refill_rate = capacity / window）
    pub window: Duration,
}

impl TokenBucketLimiter {
    /// 创建限流器
    ///
    /// # 参数
    /// - `capacity`: 桶容量（即最大突发请求数）
    /// - `window`: 时间窗口（容量允许在该窗口内突发）
    pub fn new(capacity: u32, window: Duration) -> Self {
        let cap = capacity as f64;
        Self {
            storage: Arc::new(DashMap::new()),
            capacity: cap,
            window,
        }
    }

    /// 限流检查
    pub fn check(&self, key: &str) -> bool {
        let refill_rate = self.capacity / self.window.as_secs_f64();
        if let Some(mut entry) = self.storage.get_mut(key) {
            entry.try_acquire()
        } else {
            // 新 key：创建满载桶并消耗 1 个令牌
            let mut bucket = TokenBucket::new(self.capacity, refill_rate);
            let allowed = bucket.try_acquire();
            self.storage.insert(key.to_string(), bucket);
            allowed
        }
    }

    /// 查询剩余令牌数（用于响应头 X-RateLimit-Remaining）
    pub fn remaining(&self, key: &str) -> f64 {
        if let Some(mut entry) = self.storage.get_mut(key) {
            entry.available()
        } else {
            self.capacity
        }
    }

    /// 清理过期的桶（每 1000 次请求触发一次）
    #[allow(dead_code)] // TODO(tech-debt): 限流定时清理任务接入后移除
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.storage
            .retain(|_, v| now.duration_since(v.last_refill) < self.window * 10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn 测试_token_bucket_基本限流() {
        // 中文测试名：测试 token bucket 基础限流
        let limiter = TokenBucketLimiter::new(3, Duration::from_secs(1));
        // 满载：前 3 个请求通过
        assert!(limiter.check("k1"));
        assert!(limiter.check("k1"));
        assert!(limiter.check("k1"));
        // 第 4 个被拒
        assert!(!limiter.check("k1"));
    }

    #[test]
    fn 测试_token_bucket_令牌补充() {
        // 中文测试名：测试 token bucket 等待后令牌补充
        let limiter = TokenBucketLimiter::new(2, Duration::from_millis(200));
        // 满载
        assert!(limiter.check("k1"));
        assert!(limiter.check("k1"));
        assert!(!limiter.check("k1"));
        // 等待 250ms，速率 = 2 / 0.2 = 10/秒，250ms 内补充 2.5 个
        sleep(Duration::from_millis(250));
        assert!(limiter.check("k1"));
    }

    #[test]
    fn 测试_token_bucket_多key独立() {
        // 中文测试名：测试 token bucket 多 key 互不干扰
        let limiter = TokenBucketLimiter::new(2, Duration::from_secs(1));
        assert!(limiter.check("k1"));
        assert!(limiter.check("k1"));
        assert!(!limiter.check("k1"));
        // k2 独立计数
        assert!(limiter.check("k2"));
        assert!(limiter.check("k2"));
        assert!(!limiter.check("k2"));
    }

    #[test]
    fn 测试_remaining_查询() {
        // 中文测试名：测试 remaining 查询剩余令牌
        let limiter = TokenBucketLimiter::new(5, Duration::from_secs(1));
        let _ = limiter.check("k1");
        let _ = limiter.check("k1");
        let remaining = limiter.remaining("k1");
        // 满载 5 - 已用 2 = 3 附近（允许浮点误差）
        assert!(remaining < 5.0);
        assert!(remaining >= 2.0);
    }
}
