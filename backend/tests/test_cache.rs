//! MemoryCache 单元测试

use bingxi_backend::utils::cache::{Cache, CacheKey, CacheStats, MemoryCache};
use std::thread;
use std::time::Duration;

#[test]
fn test_cache_basic_get_set() {
    let cache = MemoryCache::new();

    cache.set("key1".to_string(), "value1".to_string(), None);
    assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    assert_eq!(cache.get(&"key2".to_string()), None);
}

#[test]
fn test_cache_overwrite() {
    let cache = MemoryCache::new();

    cache.set("key1".to_string(), "value1".to_string(), None);
    cache.set("key1".to_string(), "value2".to_string(), None);
    assert_eq!(cache.get(&"key1".to_string()), Some("value2".to_string()));
}

#[test]
fn test_cache_remove() {
    let cache = MemoryCache::new();

    cache.set("key1".to_string(), "value1".to_string(), None);
    assert!(cache.contains_key(&"key1".to_string()));

    cache.remove(&"key1".to_string());
    assert!(!cache.contains_key(&"key1".to_string()));
    assert_eq!(cache.get(&"key1".to_string()), None);
}

#[test]
fn test_cache_clear() {
    let cache = MemoryCache::new();

    cache.set("key1".to_string(), "value1".to_string(), None);
    cache.set("key2".to_string(), "value2".to_string(), None);
    assert_eq!(cache.stats().size, 2);

    cache.clear();
    assert_eq!(cache.stats().size, 0);
    assert_eq!(cache.get(&"key1".to_string()), None);
    assert_eq!(cache.get(&"key2".to_string()), None);
}

#[test]
fn test_cache_contains_key() {
    let cache = MemoryCache::new();

    assert!(!cache.contains_key(&"key1".to_string()));
    cache.set("key1".to_string(), "value1".to_string(), None);
    assert!(cache.contains_key(&"key1".to_string()));
}

#[test]
fn test_cache_ttl_expiration() {
    let cache = MemoryCache::new();

    cache.set(
        "key1".to_string(),
        "value1".to_string(),
        Some(Duration::from_millis(50)),
    );
    assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));

    thread::sleep(Duration::from_millis(100));
    assert_eq!(cache.get(&"key1".to_string()), None);
}

#[test]
fn test_cache_ttl_not_expired() {
    let cache = MemoryCache::new();

    cache.set(
        "key1".to_string(),
        "value1".to_string(),
        Some(Duration::from_secs(10)),
    );
    assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
}

#[test]
fn test_cache_stats() {
    let cache = MemoryCache::new();

    cache.set("key1".to_string(), "value1".to_string(), None);
    cache.get(&"key1".to_string()); // hit
    cache.get(&"key2".to_string()); // miss

    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.size, 1);
}

#[test]
fn test_cache_stats_hits_misses() {
    let cache = MemoryCache::new();

    cache.set("a".to_string(), 1, None);
    cache.set("b".to_string(), 2, None);

    cache.get(&"a".to_string());
    cache.get(&"a".to_string());
    cache.get(&"b".to_string());
    cache.get(&"c".to_string());

    let stats = cache.stats();
    assert_eq!(stats.hits, 3);
    assert_eq!(stats.misses, 1);
}

#[test]
fn test_cache_with_capacity() {
    let cache: MemoryCache<String, i32> = MemoryCache::with_capacity(10);
    let stats = cache.stats();
    assert_eq!(stats.max_size, Some(10));
}

#[test]
fn test_cache_cleanup_expired() {
    let cache = MemoryCache::new();

    cache.set(
        "key1".to_string(),
        "value1".to_string(),
        Some(Duration::from_millis(1)),
    );
    cache.set("key2".to_string(), "value2".to_string(), None);

    thread::sleep(Duration::from_millis(50));
    cache.cleanup_expired();

    assert_eq!(cache.get(&"key1".to_string()), None);
    assert_eq!(cache.get(&"key2".to_string()), Some("value2".to_string()));
}

#[test]
fn test_cache_arc() {
    let cache = MemoryCache::<String, String>::arc();
    cache.set("key1".to_string(), "value1".to_string(), None);
    assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
}

#[test]
fn test_cache_thread_safety() {
    let cache = MemoryCache::<String, i32>::arc();
    let cache_clone = cache.clone();

    let handle = thread::spawn(move || {
        for i in 0..100 {
            cache_clone.set(format!("key{}", i), i, None);
        }
    });

    handle.join().unwrap();

    for i in 0..100 {
        assert_eq!(cache.get(&format!("key{}", i)), Some(i));
    }
}

#[test]
fn test_cache_key_to_string() {
    assert_eq!(
        CacheKey::DashboardOverview("7d".to_string()).to_string(),
        "dashboard:overview:7d"
    );
    assert_eq!(CacheKey::LowStockAlerts.to_string(), "inventory:low_stock");
    assert_eq!(
        CacheKey::ProductDetails(42).to_string(),
        "products:details:42"
    );
    assert_eq!(CacheKey::WarehousesList.to_string(), "warehouses:list");
}

#[test]
fn test_cache_default() {
    let cache = MemoryCache::<String, String>::default();
    assert_eq!(cache.stats().size, 0);
}

#[test]
fn test_cache_get_stats() {
    let cache = MemoryCache::<String, String>::new();
    let stats = cache.get_stats();
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.evictions, 0);
    assert_eq!(stats.size, 0);
    assert_eq!(stats.max_size, None);
}
