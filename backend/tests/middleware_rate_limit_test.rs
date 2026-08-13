use bingxi_backend::middleware::rate_limit::*;
use chrono::Duration;


/// 漏洞 #6 修复单元测试：未配置 Redis 时，check_redis_rate_limit 返回 Ok(None)；验证：默认（无 REDIS_URL /
/// RATE_LIMIT_REDIS_URL）环境下， Redis 限流器应返回 `Ok(None)`，由调用方（`check_rate_limit`）回退到内存限流
#[tokio::test]
async fn test_redis_rate_limiter_disabled_when_no_url() {
    // 确保没有 RATE_LIMIT_REDIS_URL / REDIS_URL
    unsafe {
        std::env::remove_var("RATE_LIMIT_REDIS_URL");
        std::env::remove_var("REDIS_URL");
    }

    let result = check_redis_rate_limit("test:key", 5, Duration::from_secs(60)).await;
    assert!(
        result.is_ok(),
        "未配置 Redis URL 时 check_redis_rate_limit 应返回 Ok"
    );
    assert!(
        result.unwrap().is_none(),
        "未配置 Redis URL 时应返回 Ok(None) 指示调用方回退内存限流"
    );
}

/// 漏洞 #6 修复单元测试：check_rate_limit 在无 Redis 时回退内存；验证：check_rate_limit 优先 Redis，未配置时回退到内存限流器
#[tokio::test]
async fn test_check_rate_limit_falls_back_to_memory() {
    unsafe {
        std::env::remove_var("RATE_LIMIT_REDIS_URL");
        std::env::remove_var("REDIS_URL");
    }

    let limiter = MemoryRateLimiter::new(2, Duration::from_secs(60));
    let key = "test:fallback:key";

    // 前 2 次允许
    assert!(check_rate_limit(key, 2, Duration::from_secs(60), &limiter).await);
    assert!(check_rate_limit(key, 2, Duration::from_secs(60), &limiter).await);
    // 第 3 次拒绝（内存限流器 max=2）
    assert!(
        !check_rate_limit(key, 2, Duration::from_secs(60), &limiter).await,
        "回退内存限流器后 max=2 时第 3 次应被拒绝"
    );
}

/// 漏洞 #6 修复单元测试：MemoryRateLimiter 基础功能
#[tokio::test]
async fn test_memory_rate_limiter_basic() {
    let limiter = MemoryRateLimiter::new(3, Duration::from_millis(100));
    let key = "test:basic";

    // 前 3 次允许
    assert!(limiter.check(key));
    assert!(limiter.check(key));
    assert!(limiter.check(key));
    // 第 4 次拒绝
    assert!(!limiter.check(key));

    // 等待窗口过期
    tokio::time::sleep(Duration::from_millis(150)).await;
    // 窗口重置后又允许
    assert!(limiter.check(key), "窗口过期后计数应重置并放行");
}