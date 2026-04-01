use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 缓存值结构体，包含值和过期时间
struct CachedValue<T> {
    value: T,
    expires_at: Option<Instant>,
}

/// 缓存接口
pub trait Cache<K, V> {
    fn get(&self, key: &K) -> Option<V>;
    fn set(&self, key: K, value: V, ttl: Option<Duration>);
    fn remove(&self, key: &K);
    fn clear(&self);
    fn contains_key(&self, key: &K) -> bool;
}

/// 内存缓存实现
pub struct MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    storage: DashMap<K, CachedValue<V>>,
}

impl<K, V> MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            storage: DashMap::new(),
        }
    }

    pub fn arc() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl<K, V> Cache<K, V> for MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    fn get(&self, key: &K) -> Option<V> {
        if let Some(mut entry) = self.storage.get_mut(key) {
            // 检查是否过期
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    // 过期了，移除该条目
                    self.storage.remove(key);
                    return None;
                }
            }
            // 返回值的克隆
            Some(entry.value.clone())
        } else {
            None
        }
    }

    fn set(&self, key: K, value: V, ttl: Option<Duration>) {
        let expires_at = ttl.map(|duration| Instant::now() + duration);
        self.storage.insert(key, CachedValue {
            value,
            expires_at,
        });
    }

    fn remove(&self, key: &K) {
        self.storage.remove(key);
    }

    fn clear(&self) {
        self.storage.clear();
    }

    fn contains_key(&self, key: &K) -> bool {
        self.storage.contains_key(key)
    }
}

/// 缓存键类型
pub enum CacheKey {
    // 仪表板数据
    DashboardOverview(String), // 时间范围
    SalesStatistics(String), // 时间范围
    InventoryStatistics(String), // 时间范围
    LowStockAlerts,
    
    // 产品相关
    ProductsList(String), // 查询参数
    ProductDetails(i32), // 产品ID
    ProductColors(i32), // 产品ID
    ProductCategories,
    ProductCategoryTree,
    
    // 库存相关
    InventoryStock(String), // 查询参数
    InventorySummary(String), // 查询参数
    InventoryTransactions(String), // 查询参数
    
    // 销售相关
    SalesOrders(String), // 查询参数
    SalesOrderDetails(i32), // 订单ID
    
    // 采购相关
    PurchaseOrders(String), // 查询参数
    PurchaseOrderDetails(i32), // 订单ID
    
    // 客户相关
    CustomersList(String), // 查询参数
    CustomerDetails(i32), // 客户ID
    
    // 供应商相关
    SuppliersList(String), // 查询参数
    SupplierDetails(i32), // 供应商ID
    
    // 仓库相关
    WarehousesList,
    WarehouseDetails(i32), // 仓库ID
    WarehouseLocations(i32), // 仓库ID
}

impl CacheKey {
    pub fn to_string(&self) -> String {
        match self {
            CacheKey::DashboardOverview(range) => format!("dashboard:overview:{}", range),
            CacheKey::SalesStatistics(range) => format!("dashboard:sales:{}", range),
            CacheKey::InventoryStatistics(range) => format!("dashboard:inventory:{}", range),
            CacheKey::LowStockAlerts => "inventory:low_stock".to_string(),
            CacheKey::ProductsList(params) => format!("products:list:{}", params),
            CacheKey::ProductDetails(id) => format!("products:details:{}", id),
            CacheKey::ProductColors(id) => format!("products:colors:{}", id),
            CacheKey::ProductCategories => "products:categories".to_string(),
            CacheKey::ProductCategoryTree => "products:category_tree".to_string(),
            CacheKey::InventoryStock(params) => format!("inventory:stock:{}", params),
            CacheKey::InventorySummary(params) => format!("inventory:summary:{}", params),
            CacheKey::InventoryTransactions(params) => format!("inventory:transactions:{}", params),
            CacheKey::SalesOrders(params) => format!("sales:orders:{}", params),
            CacheKey::SalesOrderDetails(id) => format!("sales:order:{}", id),
            CacheKey::PurchaseOrders(params) => format!("purchase:orders:{}", params),
            CacheKey::PurchaseOrderDetails(id) => format!("purchase:order:{}", id),
            CacheKey::CustomersList(params) => format!("customers:list:{}", params),
            CacheKey::CustomerDetails(id) => format!("customers:details:{}", id),
            CacheKey::SuppliersList(params) => format!("suppliers:list:{}", params),
            CacheKey::SupplierDetails(id) => format!("suppliers:details:{}", id),
            CacheKey::WarehousesList => "warehouses:list".to_string(),
            CacheKey::WarehouseDetails(id) => format!("warehouses:details:{}", id),
            CacheKey::WarehouseLocations(id) => format!("warehouses:locations:{}", id),
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
    }
}