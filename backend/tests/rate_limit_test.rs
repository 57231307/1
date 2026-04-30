//! 速率限制中间件单元测试

use bingxi_backend::middleware::rate_limit::RateLimiter;
use std::time::Duration;

/// 测试速率限制器基本功能
#[tokio::test]
async fn test_rate_limiter_basic_operations() {
    // 创建一个限制器，每分钟10个请求
    let rate_limiter = RateLimiter::new(10, Duration::from_secs(60));
    let key = "test_ip_123";

    // 测试前10个请求应该通过
    for i in 0..10 {
        assert!(rate_limiter.check(key), "Request {} should pass", i);
    }

    // 第11个请求应该被拒绝
    assert!(!rate_limiter.check(key), "Request 10 should be rate limited");
}

/// 测试速率限制器过期功能
#[tokio::test]
async fn test_rate_limiter_expiration() {
    // 创建一个限制器，200毫秒内2个请求
    let rate_limiter = RateLimiter::new(2, Duration::from_millis(200));
    let key = "test_ip_456";

    // 测试前2个请求通过
    assert!(rate_limiter.check(key), "First request should pass");
    assert!(rate_limiter.check(key), "Second request should pass");

    // 第3个请求应该被拒绝
    assert!(!rate_limiter.check(key), "Third request should be rate limited");

    // 等待300毫秒让限制过期
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 新的请求应该通过
    assert!(rate_limiter.check(key), "Request after expiration should pass");
}

/// 测试不同IP的速率限制隔离
#[tokio::test]
async fn test_rate_limiter_ip_isolation() {
    let rate_limiter = RateLimiter::new(2, Duration::from_secs(60));

    // 测试IP1的限制
    let key1 = "ip_1";
    assert!(rate_limiter.check(key1), "IP1 first request should pass");
    assert!(rate_limiter.check(key1), "IP1 second request should pass");
    assert!(!rate_limiter.check(key1), "IP1 third request should be rate limited");

    // 测试IP2应该不受IP1限制影响
    let key2 = "ip_2";
    assert!(rate_limiter.check(key2), "IP2 first request should pass");
    assert!(rate_limiter.check(key2), "IP2 second request should pass");
    assert!(!rate_limiter.check(key2), "IP2 third request should be rate limited");
}

/// 测试速率限制器清理功能
#[tokio::test]
async fn test_rate_limiter_cleanup() {
    let rate_limiter = RateLimiter::new(2, Duration::from_millis(100));
    let key = "test_ip_789";

    // 添加一些请求
    assert!(rate_limiter.check(key));
    assert!(rate_limiter.check(key));

    // 等待过期
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 清理过期条目
    rate_limiter.cleanup();

    // 新的请求应该通过
    assert!(rate_limiter.check(key), "Request after cleanup should pass");
}