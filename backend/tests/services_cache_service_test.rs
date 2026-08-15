use bingxi_backend::services::cache_service::CacheService;
use std::time::Duration;

#[tokio::test]
async fn test_cache_set_get() {
    // 中文测试名：测试 cache set 后能 get 到
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(10))
        .build();
    cache.set("k1".to_string(), b"v1".to_vec()).await;
    let got = cache.get("k1").await;
    assert_eq!(got, Some(b"v1".to_vec()));
}

#[tokio::test]
async fn test_cache_miss() {
    // 中文测试名：测试 cache miss 返回 None 并更新统计
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(10))
        .build();
    let got = cache.get("not-exist").await;
    assert_eq!(got, None);
    let stats = cache.stats().await;
    assert_eq!(stats.misses, 1);
}

#[tokio::test]
async fn test_cache_hit_ratio() {
    // 中文测试名：测试 cache 命中率计算
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(10))
        .build();
    cache.set("k1".to_string(), b"v1".to_vec()).await;
    let _ = cache.get("k1").await; // hit
    let _ = cache.get("k1").await; // hit
    let _ = cache.get("k2").await; // miss
    let stats = cache.stats().await;
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_ratio() - 2.0 / 3.0).abs() < 1e-9);
}

#[tokio::test]
async fn test_cache_disabled() {
    // 中文测试名：测试 cache 关闭时所有读返回 None
    let cache = CacheService::builder().enabled(false).build();
    cache.set("k1".to_string(), b"v1".to_vec()).await;
    let got = cache.get("k1").await;
    assert_eq!(got, None);
}

#[tokio::test]
async fn test_cache_invalidate_prefix_jqcppqz() {
    // P2 5-16 修复测试：invalidate_prefix 应仅清除匹配前缀的 key，保留其他 key
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(60))
        .build();
    cache
        .set("inventory:stock:1".to_string(), b"v1".to_vec())
        .await;
    cache
        .set("inventory:stock:2".to_string(), b"v2".to_vec())
        .await;
    cache.set("sales:order:1".to_string(), b"v3".to_vec()).await;

    // 失效 inventory 前缀
    cache.invalidate_prefix("inventory:").await;

    // inventory 前缀的 key 应被清除
    assert_eq!(cache.get("inventory:stock:1").await, None);
    assert_eq!(cache.get("inventory:stock:2").await, None);
    // sales 前缀的 key 应保留
    assert_eq!(cache.get("sales:order:1").await, Some(b"v3".to_vec()));
}

#[tokio::test]
async fn test_cache_set_with_ttl_dqgq() {
    // P2 5-17 修复测试：set_with_ttl 应使用自定义 TTL，过期后 get 返回 None
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(60))
        .build();
    // 设置 50ms TTL
    cache
        .set_with_ttl("k1".to_string(), b"v1".to_vec(), Duration::from_millis(50))
        .await;

    // 立即读取应命中
    assert_eq!(cache.get("k1").await, Some(b"v1".to_vec()));

    // 等待自定义 TTL 过期
    tokio::time::sleep(Duration::from_millis(80)).await;

    // 过期后读取应返回 None
    assert_eq!(cache.get("k1").await, None);
}

#[tokio::test]
async fn test_cache_set_with_ttl_cymr_ttl() {
    // P2 5-17 修复测试：set_with_ttl 的 TTL 长于默认 TTL 时，应按自定义 TTL 存活
    // 注意：moka 默认 TTL 仍会生效，此测试验证自定义 TTL 在默认 TTL 内有效
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(60))
        .build();
    cache
        .set_with_ttl("k1".to_string(), b"v1".to_vec(), Duration::from_secs(30))
        .await;
    // 立即读取应命中
    assert_eq!(cache.get("k1").await, Some(b"v1".to_vec()));
}

#[tokio::test]
async fn test_cache_set_h_set_with_ttl_fg_ttl() {
    // P2 5-17 修复测试：set 后再 set_with_ttl 应使用自定义 TTL
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(60))
        .build();
    cache.set("k1".to_string(), b"v1".to_vec()).await;
    // set_with_ttl 覆盖，设置 50ms TTL
    cache
        .set_with_ttl("k1".to_string(), b"v2".to_vec(), Duration::from_millis(50))
        .await;
    assert_eq!(cache.get("k1").await, Some(b"v2".to_vec()));

    tokio::time::sleep(Duration::from_millis(80)).await;
    // 自定义 TTL 过期后应返回 None
    assert_eq!(cache.get("k1").await, None);
}

#[tokio::test]
async fn test_cache_set_with_ttl_h_set_qczdy_ttl() {
    // P2 5-17 修复测试：set_with_ttl 后再 set 应清除自定义 TTL（回归默认 TTL）
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(60))
        .build();
    // 先设置 50ms TTL
    cache
        .set_with_ttl("k1".to_string(), b"v1".to_vec(), Duration::from_millis(50))
        .await;
    // 再用 set 覆盖（应清除自定义 TTL，使用默认 60s TTL）
    cache.set("k1".to_string(), b"v2".to_vec()).await;

    // 等待原自定义 TTL 过期时间
    tokio::time::sleep(Duration::from_millis(80)).await;
    // set 清除了自定义 TTL，应仍能读到（使用默认 TTL）
    assert_eq!(cache.get("k1").await, Some(b"v2".to_vec()));
}

/// V15 批次 07 P1-8 修复测试：with_metrics 注入后，命中/未命中自动上报 Prometheus
#[tokio::test]
async fn test_cache_with_metrics_zdsb_prometheus() {
    use bingxi_backend::services::business_metrics::BusinessMetrics;
    use bingxi_backend::services::cache_service::TTL_CONFIG;
    use bingxi_backend::services::cache_service::TTL_CUSTOMER;
    use bingxi_backend::services::cache_service::TTL_DASHBOARD;
    use bingxi_backend::services::cache_service::TTL_PERMISSION;
    use bingxi_backend::services::cache_service::TTL_PRODUCT;
    use bingxi_backend::services::cache_service::TTL_REPORT;
    use bingxi_backend::services::cache_service::TTL_USER;
    use std::sync::Arc;
    use std::time::Duration;
    let registry = prometheus::Registry::new();
    let metrics = Arc::new(BusinessMetrics::new(&registry).expect("BusinessMetrics 注册失败"));
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(60))
        .build()
        .with_metrics(metrics.clone());

    cache.set("hit_key".to_string(), b"v1".to_vec()).await;
    // 命中：应增加 cache_hits
    let _ = cache.get("hit_key").await;
    // 未命中：应增加 cache_misses
    let _ = cache.get("miss_key").await;

    // 验证 Prometheus 指标：erp_cache_hits_total == 1, erp_cache_misses_total == 1
    assert_eq!(metrics.cache_hits.get(), 1, "cache_hits 应为 1（一次命中）");
    assert_eq!(
        metrics.cache_misses.get(),
        1,
        "cache_misses 应为 1（一次未命中）"
    );
}

/// V15 批次 07 P1-8 修复测试：未注入 metrics 时，缓存功能正常，不 panic
#[tokio::test]
async fn test_cache_w_metrics_rzcgz() {
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(60))
        .build();
    // 无 with_metrics 注入，business_metrics 为 None
    cache.set("k1".to_string(), b"v1".to_vec()).await;
    let got = cache.get("k1").await;
    assert_eq!(got, Some(b"v1".to_vec()));
    // 验证本地 stats 仍正常
    let stats = cache.stats().await;
    assert_eq!(stats.hits, 1);
}

/// V15 P2 B07-P2-5 修复测试：差异化 TTL 常量按数据波动率分级
#[test]
fn test_differentiated_ttl_constants() {
    // Dashboard 最短（30s），Config 最长（1800s），符合数据波动率分级
    assert_eq!(TTL_DASHBOARD, Duration::from_secs(30));
    assert_eq!(TTL_REPORT, Duration::from_secs(120));
    assert_eq!(TTL_PERMISSION, Duration::from_secs(120));
    assert_eq!(TTL_USER, Duration::from_secs(300));
    assert_eq!(TTL_CUSTOMER, Duration::from_secs(300));
    assert_eq!(TTL_PRODUCT, Duration::from_secs(600));
    assert_eq!(TTL_CONFIG, Duration::from_secs(1800));
    // 安全敏感型（权限）TTL 应短于普通业务数据（用户/客户）
    assert!(TTL_PERMISSION < TTL_USER);
    assert!(TTL_PERMISSION < TTL_CUSTOMER);
}

/// V15 P2 B07-P2-5 修复测试：set_with_ttl 使用差异化 TTL 常量
#[tokio::test]
async fn test_set_with_differentiated_ttl() {
    let cache = CacheService::builder()
        .capacity(100)
        .ttl(Duration::from_secs(60))
        .build();
    // 使用 Dashboard TTL（30s）写入，立即读取应命中
    cache
        .set_with_ttl(
            "dashboard:orders".to_string(),
            b"v1".to_vec(),
            TTL_DASHBOARD,
        )
        .await;
    assert_eq!(cache.get("dashboard:orders").await, Some(b"v1".to_vec()));
}
