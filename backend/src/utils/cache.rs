#![allow(dead_code)]

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
    pub size: usize,
    pub max_size: Option<usize>,
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
            max_size: None,
        }
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            storage: DashMap::with_capacity(max_size),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
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
            size: self.storage.len(),
            max_size: self.max_size,
        }
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
    }

    fn contains_key(&self, key: &K) -> bool {
        self.storage.contains_key(key)
    }

    fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            size: self.storage.len(),
            max_size: self.max_size,
        }
    }

    fn cleanup_expired(&self) {
        self.cleanup();
    }
}

/// 缓存键类型
pub enum CacheKey {
    // 仪表板数据
    DashboardOverview(String),   // 时间范围
    SalesStatistics(String),     // 时间范围
    InventoryStatistics(String), // 时间范围
    LowStockAlerts,

    // 产品相关
    ProductsList(String), // 查询参数
    ProductDetails(i32),  // 产品ID
    ProductColors(i32),   // 产品ID
    ProductCategories,
    ProductCategoryTree,

    // 库存相关
    InventoryStock(String),        // 查询参数
    InventorySummary(String),      // 查询参数
    InventoryTransactions(String), // 查询参数

    // 销售相关
    SalesOrders(String),    // 查询参数
    SalesOrderDetails(i32), // 订单ID

    // 采购相关
    PurchaseOrders(String),    // 查询参数
    PurchaseOrderDetails(i32), // 订单ID

    // 客户相关
    CustomersList(String), // 查询参数
    CustomerDetails(i32),  // 客户ID

    // 供应商相关
    SuppliersList(String), // 查询参数
    SupplierDetails(i32),  // 供应商ID

    // 仓库相关
    WarehousesList,
    WarehouseDetails(i32),   // 仓库ID
    WarehouseLocations(i32), // 仓库ID
}

impl std::fmt::Display for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheKey::DashboardOverview(range) => write!(f, "dashboard:overview:{}", range),
            CacheKey::SalesStatistics(range) => write!(f, "dashboard:sales:{}", range),
            CacheKey::InventoryStatistics(range) => write!(f, "dashboard:inventory:{}", range),
            CacheKey::LowStockAlerts => write!(f, "inventory:low_stock"),
            CacheKey::ProductsList(params) => write!(f, "products:list:{}", params),
            CacheKey::ProductDetails(id) => write!(f, "products:details:{}", id),
            CacheKey::ProductColors(id) => write!(f, "products:colors:{}", id),
            CacheKey::ProductCategories => write!(f, "products:categories"),
            CacheKey::ProductCategoryTree => write!(f, "products:category_tree"),
            CacheKey::InventoryStock(params) => write!(f, "inventory:stock:{}", params),
            CacheKey::InventorySummary(params) => write!(f, "inventory:summary:{}", params),
            CacheKey::InventoryTransactions(params) => {
                write!(f, "inventory:transactions:{}", params)
            }
            CacheKey::SalesOrders(params) => write!(f, "sales:orders:{}", params),
            CacheKey::SalesOrderDetails(id) => write!(f, "sales:order:{}", id),
            CacheKey::PurchaseOrders(params) => write!(f, "purchase:orders:{}", params),
            CacheKey::PurchaseOrderDetails(id) => write!(f, "purchase:order:{}", id),
            CacheKey::CustomersList(params) => write!(f, "customers:list:{}", params),
            CacheKey::CustomerDetails(id) => write!(f, "customers:details:{}", id),
            CacheKey::SuppliersList(params) => write!(f, "suppliers:list:{}", params),
            CacheKey::SupplierDetails(id) => write!(f, "suppliers:details:{}", id),
            CacheKey::WarehousesList => write!(f, "warehouses:list"),
            CacheKey::WarehouseDetails(id) => write!(f, "warehouses:details:{}", id),
            CacheKey::WarehouseLocations(id) => write!(f, "warehouses:locations:{}", id),
        }
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
}
