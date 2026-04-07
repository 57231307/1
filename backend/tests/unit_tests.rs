//! 单元测试集合

use bingxi_backend::middleware::rate_limit::RateLimiter;
use bingxi_backend::utils::cache::{AppCache, Cache, CacheKey, MemoryCache};
use std::time::Duration;

/// 测试内存缓存基本功能
#[tokio::test]
async fn test_memory_cache_basic_operations() {
    let cache = MemoryCache::new();

    // 测试设置和获取值
    cache.set("key1", "value1", None);
    assert_eq!(cache.get(&"key1"), Some("value1"));

    // 测试覆盖值
    cache.set("key1", "value2", None);
    assert_eq!(cache.get(&"key1"), Some("value2"));

    // 测试删除值
    cache.remove(&"key1");
    assert_eq!(cache.get(&"key1"), None);

    // 测试包含键
    cache.set("key2", "value2", None);
    assert!(cache.contains_key(&"key2"));
    assert!(!cache.contains_key(&"key3"));

    // 测试清空缓存
    cache.clear();
    assert_eq!(cache.get(&"key2"), None);
}

/// 测试内存缓存过期功能
#[tokio::test]
async fn test_memory_cache_expiration() {
    let cache = MemoryCache::new();

    // 设置一个100毫秒后过期的值
    cache.set("key1", "value1", Some(Duration::from_millis(100)));
    assert_eq!(cache.get(&"key1"), Some("value1"));

    // 等待150毫秒让缓存过期
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 验证值已过期
    assert_eq!(cache.get(&"key1"), None);
}

/// 测试缓存键转换
#[tokio::test]
async fn test_cache_key_to_string() {
    // 测试仪表板相关缓存键
    let overview_key = CacheKey::DashboardOverview("7d".to_string());
    assert_eq!(overview_key.to_string(), "dashboard:overview:7d");

    let sales_key = CacheKey::SalesStatistics("30d".to_string());
    assert_eq!(sales_key.to_string(), "dashboard:sales:30d");

    // 测试产品相关缓存键
    let product_key = CacheKey::ProductDetails(123);
    assert_eq!(product_key.to_string(), "products:details:123");

    let categories_key = CacheKey::ProductCategories;
    assert_eq!(categories_key.to_string(), "products:categories");

    // 测试库存相关缓存键
    let stock_key = CacheKey::InventoryStock("warehouse=1".to_string());
    assert_eq!(stock_key.to_string(), "inventory:stock:warehouse=1");

    let low_stock_key = CacheKey::LowStockAlerts;
    assert_eq!(low_stock_key.to_string(), "inventory:low_stock");
}

/// 测试全局缓存管理
#[tokio::test]
async fn test_app_cache_management() {
    let app_cache = AppCache::new();

    // 测试获取各个缓存实例
    let dashboard_cache = app_cache.get_dashboard_cache();
    let product_cache = app_cache.get_product_cache();
    let inventory_cache = app_cache.get_inventory_cache();

    // 测试在不同缓存中存储数据
    dashboard_cache.set(
        "overview".to_string(),
        serde_json::json!("dashboard_data"),
        None,
    );
    product_cache.set(
        "product_1".to_string(),
        serde_json::json!("product_data"),
        None,
    );
    inventory_cache.set(
        "stock_1".to_string(),
        serde_json::json!("inventory_data"),
        None,
    );

    // 验证所有缓存都能正常获取
    assert_eq!(
        dashboard_cache.get(&"overview".to_string()),
        Some(serde_json::json!("dashboard_data"))
    );
    assert_eq!(
        product_cache.get(&"product_1".to_string()),
        Some(serde_json::json!("product_data"))
    );
    assert_eq!(
        inventory_cache.get(&"stock_1".to_string()),
        Some(serde_json::json!("inventory_data"))
    );

    // 清除所有缓存
    app_cache.clear_all();

    // 验证所有缓存都已被清除
    assert_eq!(dashboard_cache.get(&"overview".to_string()), None);
    assert_eq!(product_cache.get(&"product_1".to_string()), None);
    assert_eq!(inventory_cache.get(&"stock_1".to_string()), None);
}

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
    assert!(
        !rate_limiter.check(key),
        "Request 10 should be rate limited"
    );
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
    assert!(
        !rate_limiter.check(key),
        "Third request should be rate limited"
    );

    // 等待300毫秒让限制过期
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 新的请求应该通过
    assert!(
        rate_limiter.check(key),
        "Request after expiration should pass"
    );
}

/// 测试不同IP的速率限制隔离
#[tokio::test]
async fn test_rate_limiter_ip_isolation() {
    let rate_limiter = RateLimiter::new(2, Duration::from_secs(60));

    // 测试IP1的限制
    let key1 = "ip_1";
    assert!(rate_limiter.check(key1), "IP1 first request should pass");
    assert!(rate_limiter.check(key1), "IP1 second request should pass");
    assert!(
        !rate_limiter.check(key1),
        "IP1 third request should be rate limited"
    );

    // 测试IP2应该不受IP1限制影响
    let key2 = "ip_2";
    assert!(rate_limiter.check(key2), "IP2 first request should pass");
    assert!(rate_limiter.check(key2), "IP2 second request should pass");
    assert!(
        !rate_limiter.check(key2),
        "IP2 third request should be rate limited"
    );
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
