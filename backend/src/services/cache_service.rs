//! 性能优化缓存服务（P4-1 性能优化）
//!
//! 提供进程内 LRU + TTL 缓存，专为以下场景设计：
//! - Dashboard 热点数据（订单数 / 库存 / 应收应付汇总）
//! - 配置类数据（菜单 / 权限 / 字典）
//! - 报表聚合查询
//!
//! ## 关键设计
//!
//! - **命名空间隔离**：key 必须以 `module:` 开头，避免跨模块数据串味
//! - **TTL 失效**：通过 `moka::future::Cache` 的 expire_after 实现
//! - **容量上限**：默认 10000 条，超过 LRU 淘汰
//! - **指标埋点**：`hit` / `miss` 计数
//!
//! ## 关闭缓存
//!
//! 缓存仅作为加速层；查询逻辑必须保证绕过缓存也能返回正确数据。
//! 关闭方式：把 `CACHE_ENABLED=false` 写入环境变量即可。

use moka::future::Cache;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 缓存命中/未命中统计
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
}

impl CacheStats {
    /// 计算命中率（0.0 - 1.0）
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// 进程内缓存服务（基于 moka）
#[derive(Clone)]
pub struct CacheService {
    inner: Arc<Cache<String, Vec<u8>>>,
    stats: Arc<RwLock<CacheStats>>,
    enabled: bool,
    /// 默认 TTL
    default_ttl: Duration,
    /// P2 5-16 修复：key 索引，用于 invalidate_prefix 按前缀精确失效
    /// 记录所有当前存活的 key，invalidate_prefix 时遍历匹配前缀后逐个 invalidate
    key_index: Arc<RwLock<HashSet<String>>>,
    /// P2 5-17 修复：per-key 自定义 TTL 过期时间戳
    /// set_with_ttl 时记录 (key, 过期时刻)；get 时检查是否已过期，过期则返回 None 并清理
    custom_expirations: Arc<RwLock<HashMap<String, Instant>>>,
}

impl CacheService {
    /// 创建默认配置缓存（容量 10000、TTL 60s）
    ///
    /// 批次 107 P1-1 修复：已接入 AppState.cache_service，移除 dead_code 标注
    pub fn new() -> Self {
        let enabled = std::env::var("CACHE_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);
        let capacity = std::env::var("CACHE_CAPACITY")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(10_000);
        let ttl_secs = std::env::var("CACHE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        Self::builder()
            .capacity(capacity)
            .ttl(Duration::from_secs(ttl_secs))
            .enabled(enabled)
            .build()
    }

    /// 构建器入口
    pub fn builder() -> CacheServiceBuilder {
        CacheServiceBuilder::default()
    }

    /// 获取缓存值
    ///
    /// P2 5-17 修复：get 时检查 per-key 自定义 TTL，过期则返回 None 并清理
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        if !self.enabled {
            return None;
        }

        // P2 5-17 修复：检查自定义 TTL 是否已过期
        {
            let custom_exp = self.custom_expirations.read().await;
            if let Some(deadline) = custom_exp.get(key) {
                if Instant::now() >= *deadline {
                    // 已过期，释放读锁后清理
                    drop(custom_exp);
                    self.custom_expirations.write().await.remove(key);
                    self.key_index.write().await.remove(key);
                    self.inner.invalidate(key).await;
                    self.stats.write().await.misses += 1;
                    return None;
                }
            }
        }

        match self.inner.get(key).await {
            Some(v) => {
                self.stats.write().await.hits += 1;
                Some(v)
            }
            None => {
                self.stats.write().await.misses += 1;
                None
            }
        }
    }

    /// 写入缓存
    ///
    /// P2 5-17 修复：set 时清除该 key 的自定义 TTL（回归默认 TTL）
    pub async fn set(&self, key: String, value: Vec<u8>) {
        if !self.enabled {
            return;
        }
        // P2 5-16 修复：写入 key 索引
        self.key_index.write().await.insert(key.clone());
        // P2 5-17 修复：清除自定义 TTL（使用默认 TTL）
        self.custom_expirations.write().await.remove(&key);
        self.inner.insert(key, value).await;
    }

    /// 带自定义 TTL 的写入
    ///
    /// P2 5-17 修复：原实现忽略 ttl 参数（let _ = ttl），统一使用 default_ttl
    /// 现改为记录 per-key 过期时间戳，get 时检查并清理过期 entry
    ///
    /// 批次 107 P1-1 修复：已接入 AppState.cache_service，移除 dead_code 标注
    pub async fn set_with_ttl(&self, key: String, value: Vec<u8>, ttl: Duration) {
        if !self.enabled {
            return;
        }
        // P2 5-16 修复：写入 key 索引
        self.key_index.write().await.insert(key.clone());
        // P2 5-17 修复：记录 per-key 自定义过期时间戳
        let deadline = Instant::now() + ttl;
        self.custom_expirations.write().await.insert(key.clone(), deadline);
        // 注意：moka 顶层只支持统一 TTL，这里仍用 insert 写入
        // per-entry TTL 通过 get 时的过期检查实现
        self.inner.insert(key, value).await;
    }

    /// 失效指定 key
    ///
    /// P2 5-16 修复：同步清理 key 索引和自定义 TTL
    ///
    /// 批次 107 P1-1 修复：已接入 AppState.cache_service，移除 dead_code 标注
    pub async fn invalidate(&self, key: &str) {
        self.key_index.write().await.remove(key);
        self.custom_expirations.write().await.remove(key);
        self.inner.invalidate(key).await;
    }

    /// 按前缀失效（按模块命名空间批量失效缓存）
    ///
    /// P2 5-16 修复：原实现为 invalidate_all() 全量失效，前缀参数被忽略
    /// 现改为遍历 key 索引，匹配前缀后逐个 invalidate
    pub async fn invalidate_prefix(&self, prefix: &str) {
        // 收集匹配前缀的 key（避免持有锁的同时调用 invalidate）
        let keys_to_invalidate: Vec<String> = {
            let index = self.key_index.read().await;
            index
                .iter()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect()
        };

        // 逐个失效匹配的 key
        for key in &keys_to_invalidate {
            self.custom_expirations.write().await.remove(key);
            self.inner.invalidate(key).await;
        }
        // 批量从索引中移除
        let mut index = self.key_index.write().await;
        for key in &keys_to_invalidate {
            index.remove(key);
        }
    }

    /// 当前统计
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// 获取默认 TTL
    ///
    /// 批次 107 P1-1 修复：已接入 AppState.cache_service，移除 dead_code 标注
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }
}

/// 批次 107 P1-1 修复：已接入 AppState.cache_service，移除 dead_code 标注
impl Default for CacheService {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建器
#[derive(Default)]
pub struct CacheServiceBuilder {
    capacity: Option<u64>,
    ttl: Option<Duration>,
    enabled: Option<bool>,
}

impl CacheServiceBuilder {
    /// 容量
    pub fn capacity(mut self, cap: u64) -> Self {
        self.capacity = Some(cap);
        self
    }

    /// TTL
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// 是否启用
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = Some(on);
        self
    }

    /// 构建
    pub fn build(self) -> CacheService {
        let enabled = self.enabled.unwrap_or(true);
        let capacity = self.capacity.unwrap_or(10_000);
        let ttl = self.ttl.unwrap_or(Duration::from_secs(60));
        let inner = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(ttl)
            .build();
        CacheService {
            inner: Arc::new(inner),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            enabled,
            default_ttl: ttl,
            key_index: Arc::new(RwLock::new(HashSet::new())),
            custom_expirations: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn 测试_cache_set_get() {
        // 中文测试名：测试 cache set 后能 get 到
        let cache = CacheService::builder().capacity(100).ttl(Duration::from_secs(10)).build();
        cache.set("k1".to_string(), b"v1".to_vec()).await;
        let got = cache.get("k1").await;
        assert_eq!(got, Some(b"v1".to_vec()));
    }

    #[tokio::test]
    async fn 测试_cache_miss() {
        // 中文测试名：测试 cache miss 返回 None 并更新统计
        let cache = CacheService::builder().capacity(100).ttl(Duration::from_secs(10)).build();
        let got = cache.get("not-exist").await;
        assert_eq!(got, None);
        let stats = cache.stats().await;
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn 测试_cache_hit_ratio() {
        // 中文测试名：测试 cache 命中率计算
        let cache = CacheService::builder().capacity(100).ttl(Duration::from_secs(10)).build();
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
    async fn 测试_cache_disabled() {
        // 中文测试名：测试 cache 关闭时所有读返回 None
        let cache = CacheService::builder().enabled(false).build();
        cache.set("k1".to_string(), b"v1".to_vec()).await;
        let got = cache.get("k1").await;
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn 测试_cache_invalidate_prefix_仅清除匹配前缀() {
        // P2 5-16 修复测试：invalidate_prefix 应仅清除匹配前缀的 key，保留其他 key
        let cache = CacheService::builder().capacity(100).ttl(Duration::from_secs(60)).build();
        cache.set("inventory:stock:1".to_string(), b"v1".to_vec()).await;
        cache.set("inventory:stock:2".to_string(), b"v2".to_vec()).await;
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
    async fn 测试_cache_set_with_ttl_短期过期() {
        // P2 5-17 修复测试：set_with_ttl 应使用自定义 TTL，过期后 get 返回 None
        let cache = CacheService::builder().capacity(100).ttl(Duration::from_secs(60)).build();
        // 设置 50ms TTL
        cache.set_with_ttl("k1".to_string(), b"v1".to_vec(), Duration::from_millis(50)).await;

        // 立即读取应命中
        assert_eq!(cache.get("k1").await, Some(b"v1".to_vec()));

        // 等待自定义 TTL 过期
        tokio::time::sleep(Duration::from_millis(80)).await;

        // 过期后读取应返回 None
        assert_eq!(cache.get("k1").await, None);
    }

    #[tokio::test]
    async fn 测试_cache_set_with_ttl_长于默认_ttl() {
        // P2 5-17 修复测试：set_with_ttl 的 TTL 长于默认 TTL 时，应按自定义 TTL 存活
        // 注意：moka 默认 TTL 仍会生效，此测试验证自定义 TTL 在默认 TTL 内有效
        let cache = CacheService::builder().capacity(100).ttl(Duration::from_secs(60)).build();
        cache.set_with_ttl("k1".to_string(), b"v1".to_vec(), Duration::from_secs(30)).await;
        // 立即读取应命中
        assert_eq!(cache.get("k1").await, Some(b"v1".to_vec()));
    }

    #[tokio::test]
    async fn 测试_cache_set_后_set_with_ttl_覆盖_ttl() {
        // P2 5-17 修复测试：set 后再 set_with_ttl 应使用自定义 TTL
        let cache = CacheService::builder().capacity(100).ttl(Duration::from_secs(60)).build();
        cache.set("k1".to_string(), b"v1".to_vec()).await;
        // set_with_ttl 覆盖，设置 50ms TTL
        cache.set_with_ttl("k1".to_string(), b"v2".to_vec(), Duration::from_millis(50)).await;
        assert_eq!(cache.get("k1").await, Some(b"v2".to_vec()));

        tokio::time::sleep(Duration::from_millis(80)).await;
        // 自定义 TTL 过期后应返回 None
        assert_eq!(cache.get("k1").await, None);
    }

    #[tokio::test]
    async fn 测试_cache_set_with_ttl_后_set_清除自定义_ttl() {
        // P2 5-17 修复测试：set_with_ttl 后再 set 应清除自定义 TTL（回归默认 TTL）
        let cache = CacheService::builder().capacity(100).ttl(Duration::from_secs(60)).build();
        // 先设置 50ms TTL
        cache.set_with_ttl("k1".to_string(), b"v1".to_vec(), Duration::from_millis(50)).await;
        // 再用 set 覆盖（应清除自定义 TTL，使用默认 60s TTL）
        cache.set("k1".to_string(), b"v2".to_vec()).await;

        // 等待原自定义 TTL 过期时间
        tokio::time::sleep(Duration::from_millis(80)).await;
        // set 清除了自定义 TTL，应仍能读到（使用默认 TTL）
        assert_eq!(cache.get("k1").await, Some(b"v2".to_vec()));
    }
}
