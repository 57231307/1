//! 缓存模块单元测试

use bingxi_backend::utils::cache::{AppCache, CacheKey, MemoryCache};
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
    dashboard_cache.set("overview", "dashboard_data", None);
    product_cache.set("product_1", "product_data", None);
    inventory_cache.set("stock_1", "inventory_data", None);

    // 测试获取数据
    assert_eq!(dashboard_cache.get(&"overview"), Some("dashboard_data"));
    assert_eq!(product_cache.get(&"product_1"), Some("product_data"));
    assert_eq!(inventory_cache.get(&"stock_1"), Some("inventory_data"));

    // 测试清空所有缓存
    app_cache.clear_all();
    assert_eq!(dashboard_cache.get(&"overview"), None);
    assert_eq!(product_cache.get(&"product_1"), None);
    assert_eq!(inventory_cache.get(&"stock_1"), None);
}