// TODO(tech-debt): 此文件已开启 dead_code 检查；后续接入时如出现未使用项，应按模板逐项评估。
// 当前所有 pub API 均已被业务引用（AppCache/MemoryCache/Cache trait/CacheStats）。
// 私有项 CachedValue<T> 内部使用。如未来新增 API 暂时未接入，应使用项级 #[allow(dead_code)] + TODO 标注。

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub writes: u64,
    pub size: usize,
    pub max_size: Option<usize>,
}

impl CacheStats {
    /// 获取命中率（百分比）
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// 获取统计摘要
    pub fn summary(&self) -> String {
        format!(
            "命中: {}, 未命中: {}, 淘汰: {}, 写入: {}, 命中率: {:.1}%",
            self.hits,
            self.misses,
            self.evictions,
            self.writes,
            self.hit_rate()
        )
    }
}

/// 缓存值结构体，包含值和过期时间
struct CachedValue<T> {
    value: T,
    expires_at: Option<Instant>,
    created_at: Instant,
}

/// 缓存接口
pub trait Cache<K, V> {
    fn get(&self, key: &K) -> Option<V>;
    fn set(&self, key: K, value: V, ttl: Option<Duration>);
    fn remove(&self, key: &K);
    fn clear(&self);
    fn contains_key(&self, key: &K) -> bool;
    fn stats(&self) -> CacheStats;
    fn cleanup_expired(&self);
    fn evict_oldest(&self, target_size: usize);
}

/// 内存缓存实现
pub struct MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    storage: DashMap<K, CachedValue<V>>,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    writes: AtomicU64,
    max_size: Option<usize>,
}

impl<K, V> Default for MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            storage: DashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            writes: AtomicU64::new(0),
            max_size: None,
        }
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            storage: DashMap::with_capacity(max_size),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            writes: AtomicU64::new(0),
            max_size: Some(max_size),
        }
    }

    pub fn arc() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn arc_with_capacity(max_size: usize) -> Arc<Self> {
        Arc::new(Self::with_capacity(max_size))
    }

    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            size: self.storage.len(),
            max_size: self.max_size,
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.writes.store(0, Ordering::Relaxed);
    }

    pub fn cleanup(&self) {
        let now = Instant::now();
        let mut removed = 0u64;
        self.storage.retain(|_, v| {
            let keep = v.expires_at.is_none_or(|exp| now <= exp);
            if !keep {
                removed += 1;
            }
            keep
        });
        self.evictions.fetch_add(removed, Ordering::Relaxed);
    }
}

impl<K, V> Cache<K, V> for MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    fn get(&self, key: &K) -> Option<V> {
        let entry = match self.storage.get(key) {
            Some(e) => e,
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        let expired = entry.expires_at.is_some_and(|exp| Instant::now() > exp);
        if expired {
            drop(entry);
            self.storage.remove(key);
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        self.hits.fetch_add(1, Ordering::Relaxed);
        Some(entry.value.clone())
    }

    fn set(&self, key: K, value: V, ttl: Option<Duration>) {
        let expires_at = ttl.map(|duration| Instant::now() + duration);
        let cached_value = CachedValue {
            value,
            expires_at,
            created_at: Instant::now(),
        };

        self.storage.insert(key.clone(), cached_value);
        self.writes.fetch_add(1, Ordering::Relaxed);

        if let Some(max_size) = self.max_size {
            let current_size = self.storage.len();
            if current_size > max_size {
                self.evict_oldest(max_size);
            }
        }
    }

    fn evict_oldest(&self, target_size: usize) {
        let mut removed = 0u64;

        self.storage.retain(|_, _v| {
            if target_size <= self.storage.len() - removed as usize {
                removed += 1;
                false
            } else {
                true
            }
        });

        self.evictions.fetch_add(removed, Ordering::Relaxed);
    }

    fn remove(&self, key: &K) {
        self.storage.remove(key);
    }

    fn clear(&self) {
        self.storage.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.writes.store(0, Ordering::Relaxed);
    }

    fn contains_key(&self, key: &K) -> bool {
        self.storage.contains_key(key)
    }

    fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            size: self.storage.len(),
            max_size: self.max_size,
        }
    }

    fn cleanup_expired(&self) {
        self.cleanup();
    }
}

/// 全局缓存实例
pub struct AppCache {
    pub dashboard_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub product_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub inventory_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub sales_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub purchase_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub customer_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub supplier_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub warehouse_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub token_blacklist: Arc<MemoryCache<String, bool>>,
    /// CSRF Token 缓存：key 为 session_id，value 为 csrf_token
    pub csrf_token_cache: Arc<MemoryCache<String, String>>,
}

impl Default for AppCache {
    fn default() -> Self {
        Self::new()
    }
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            dashboard_cache: MemoryCache::arc(),
            product_cache: MemoryCache::arc(),
            inventory_cache: MemoryCache::arc(),
            sales_cache: MemoryCache::arc(),
            purchase_cache: MemoryCache::arc(),
            customer_cache: MemoryCache::arc(),
            supplier_cache: MemoryCache::arc(),
            warehouse_cache: MemoryCache::arc(),
            token_blacklist: MemoryCache::arc(),
            csrf_token_cache: MemoryCache::arc(),
        }
    }

    pub fn arc() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// 获取仪表板缓存
    pub fn get_dashboard_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.dashboard_cache.clone()
    }

    /// 获取产品缓存
    pub fn get_product_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.product_cache.clone()
    }

    /// 获取库存缓存
    pub fn get_inventory_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.inventory_cache.clone()
    }

    /// 获取销售缓存
    pub fn get_sales_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.sales_cache.clone()
    }

    /// 获取采购缓存
    pub fn get_purchase_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.purchase_cache.clone()
    }

    /// 获取客户缓存
    pub fn get_customer_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.customer_cache.clone()
    }

    /// 获取供应商缓存
    pub fn get_supplier_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.supplier_cache.clone()
    }

    /// 获取仓库缓存
    pub fn get_warehouse_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.warehouse_cache.clone()
    }

    /// 获取 Token 黑名单缓存
    pub fn get_token_blacklist(&self) -> Arc<MemoryCache<String, bool>> {
        self.token_blacklist.clone()
    }

    /// 获取 CSRF Token 缓存
    pub fn get_csrf_token_cache(&self) -> Arc<MemoryCache<String, String>> {
        self.csrf_token_cache.clone()
    }

    /// 清除所有缓存
    pub fn clear_all(&self) {
        self.dashboard_cache.clear();
        self.product_cache.clear();
        self.inventory_cache.clear();
        self.sales_cache.clear();
        self.purchase_cache.clear();
        self.customer_cache.clear();
        self.supplier_cache.clear();
        self.warehouse_cache.clear();
        // Do not clear token blacklist on general clear_all
    }

    /// 获取所有缓存的全局统计信息
    pub fn global_stats(&self) -> CacheStats {
        let mut total_hits = 0u64;
        let mut total_misses = 0u64;
        let mut total_evictions = 0u64;
        let mut total_writes = 0u64;
        let mut total_size = 0usize;

        // 统计所有业务缓存
        let caches: Vec<&Arc<MemoryCache<String, serde_json::Value>>> = vec![
            &self.dashboard_cache,
            &self.product_cache,
            &self.inventory_cache,
            &self.sales_cache,
            &self.purchase_cache,
            &self.customer_cache,
            &self.supplier_cache,
            &self.warehouse_cache,
        ];

        for cache in caches {
            let stats = cache.get_stats();
            total_hits += stats.hits;
            total_misses += stats.misses;
            total_evictions += stats.evictions;
            total_writes += stats.writes;
            total_size += stats.size;
        }

        CacheStats {
            hits: total_hits,
            misses: total_misses,
            evictions: total_evictions,
            writes: total_writes,
            size: total_size,
            max_size: None,
        }
    }

    /// 获取所有缓存的统计摘要
    pub fn global_summary(&self) -> String {
        self.global_stats().summary()
    }

    /// 重置所有缓存的统计信息
    pub fn reset_all_stats(&self) {
        self.dashboard_cache.reset_stats();
        self.product_cache.reset_stats();
        self.inventory_cache.reset_stats();
        self.sales_cache.reset_stats();
        self.purchase_cache.reset_stats();
        self.customer_cache.reset_stats();
        self.supplier_cache.reset_stats();
        self.warehouse_cache.reset_stats();
        self.token_blacklist.reset_stats();
        self.csrf_token_cache.reset_stats();
    }
}
