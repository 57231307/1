// dead_code 检查已开启；当前所有 pub API 均已被业务引用（AppCache/MemoryCache/Cache trait/CacheStats）。
// 私有项 CachedValue<T> 内部使用。

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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
    // 批次 158 v11 真实接入：evict_oldest 使用此字段实现 LRU 淘汰策略
    created_at: Instant,
}

/// 缓存接口
pub trait Cache<K, V> {
    fn get(&self, key: &K) -> Option<V>;
    fn set(&self, key: K, value: V, ttl: Option<Duration>);
    fn clear(&self);
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

    /// 一次性获取并移除缓存项（rotation 模式用于 CSRF Token 等；与 get 不同返回同时删除键实现 token rotation 只能消费一次，键不存在或过期返回 None 计 miss）
    pub fn take(&self, key: &K) -> Option<V> {
        match self.storage.remove(key) {
            Some((_, cached)) => {
                let expired = cached.expires_at.is_some_and(|exp| Instant::now() > exp);
                if expired {
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(cached.value)
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
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
        self.writes.fetch_add(1, Ordering::Relaxed);

        if let Some(max_size) = self.max_size {
            let current_size = self.storage.len();
            if current_size > max_size {
                self.evict_oldest(max_size);
            }
        }
    }

    fn evict_oldest(&self, target_size: usize) {
        // 批次 158 v11 真实接入：基于 created_at 的 LRU 淘汰策略
        // 原实现使用 retain 任意淘汰，无法保证淘汰最旧缓存项；
        // 现按 created_at 升序排序后淘汰最旧的 N 项，符合 LRU 语义
        let current_size = self.storage.len();
        if current_size <= target_size {
            return;
        }
        let need_remove = current_size - target_size;

        // 收集所有 (key 引用, created_at) 并按 created_at 升序排序
        let mut entries: Vec<(K, Instant)> = self
            .storage
            .iter()
            .map(|e| (e.key().clone(), e.value().created_at))
            .collect();
        entries.sort_by_key(|(_, t)| *t);

        // 淘汰最旧的 need_remove 项
        let mut removed = 0u64;
        for (key, _) in entries.into_iter().take(need_remove) {
            if self.storage.remove(&key).is_some() {
                removed += 1;
            }
        }

        self.evictions.fetch_add(removed, Ordering::Relaxed);
    }

    fn clear(&self) {
        self.storage.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.writes.store(0, Ordering::Relaxed);
    }
}

// CSRF Token 缓存常量
// TODO(tech-debt): CSRF Token 默认 TTL，从 7200s（2h）缩短为 1800s（30min），
// 与 access_token Cookie 30min 有效期对齐，降低被窃取后的暴露窗口。
// Wave 3 安全漏洞 #7 修复引入。
pub const CSRF_TOKEN_DEFAULT_TTL_SECS: u64 = 1800;

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
    /// 缺陷 3.1 修复：BI 多维分析聚合结果缓存（5 分钟 TTL）
    pub bi_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub token_blacklist: Arc<MemoryCache<String, bool>>,
    /// CSRF Token 缓存：key=csrf_token, value=(session_id, ip_address)。
    /// IP 绑定用于防御 CSRF 窃取后的跨 IP 重放（Wave 3 安全漏洞 #7）。
    pub csrf_token_cache: Arc<MemoryCache<String, (String, String)>>,
    /// CSRF Token 反向索引（key=user_id, value=活跃 csrf_token；原始 DashMap 便于按 value 反查清理；登录时强制轮换防多设备旧 token 残留）
    pub csrf_user_index: DashMap<i32, String>,
}

/// CSRF Token 消费结果（Wave 3 安全漏洞 #7）
/// 区分 IP 不匹配、缺失/过期两种失败原因，前端可基于业务码差异化处理（IP 失配引导重新登录，缺失/过期提示刷新）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsrfConsumeResult {
    /// 消费成功（token 有效 + IP 匹配，已从缓存移除）
    Ok,
    /// IP 地址不匹配（token 存在但绑定到其他 IP，疑似盗用）
    IpMismatch,
    /// Token 不存在或已过期
    NotFound,
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
            bi_cache: MemoryCache::arc(),
            token_blacklist: MemoryCache::arc(),
            csrf_token_cache: MemoryCache::arc(),
            csrf_user_index: DashMap::new(),
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

    /// 缺陷 3.1 修复：获取 BI 多维分析缓存
    pub fn get_bi_cache(&self) -> Arc<MemoryCache<String, serde_json::Value>> {
        self.bi_cache.clone()
    }

    /// 获取 Token 黑名单缓存
    pub fn get_token_blacklist(&self) -> Arc<MemoryCache<String, bool>> {
        self.token_blacklist.clone()
    }

    /// 获取 CSRF Token 缓存（保留用于测试与内部维护）
    /// 优先使用 set_csrf_token / consume_csrf_token / clear_old_csrf_token_for_user 高层 API（封装 IP 绑定 + 强制轮换）
    pub fn get_csrf_token_cache(&self) -> Arc<MemoryCache<String, (String, String)>> {
        self.csrf_token_cache.clone()
    }

    /// 获取 CSRF Token 反向索引（user_id → csrf_token；保留供测试与内部维护，优先使用 clear_old_csrf_token_for_user 访问）
    pub fn get_csrf_user_index(&self) -> &DashMap<i32, String> {
        &self.csrf_user_index
    }

    /// 写入 CSRF Token（含 IP 绑定 + 反向索引维护，Wave 3 安全漏洞 #7 修复）
    /// 缓存值=(session_id, ip_address) 元组 IP 校验；反向索引 user_id→token 便于登录轮换；旧 token 由调用方写入前清除。参数：token/session_id/ip_address/user_id/ttl（None 用 CSRF_TOKEN_DEFAULT_TTL_SECS）
    // 批次 327 v10 复审 P3 修复：移除误报的 #[allow]
    // - too_many_arguments：仅 5 参数（token, session_id, ip_address, user_id, ttl），低于阈值 7
    // - needless_pass_by_value：owned String 来自上游调用方，保留 owned 形式避免生命周期污染
    pub fn set_csrf_token(
        &self,
        token: String,
        session_id: String,
        ip_address: String,
        user_id: i32,
        ttl: Option<Duration>,
    ) {
        let effective_ttl = ttl.unwrap_or(Duration::from_secs(CSRF_TOKEN_DEFAULT_TTL_SECS));
        self.csrf_token_cache
            .set(token.clone(), (session_id, ip_address), Some(effective_ttl));
        // 反向索引不显式 TTL：其生命周期由 csrf_token_cache 的 TTL 隐式决定
        // （每次 set_csrf_token 都会覆盖 user_id → token 映射；并发场景下后写覆盖前写）
        self.csrf_user_index.insert(user_id, token);
    }

    /// 校验并消费一次性 CSRF Token（含 IP 校验）
    /// 行为：找不到→NotFound；IP 不匹配→IpMismatch（保留原条目及其剩余 TTL，防 DoS 探测同时避免 TTL 刷新为永久）；IP 匹配→Ok（消费并清理反向索引）。参数：token(X-CSRF-Token 头)/client_ip
    pub fn consume_csrf_token(&self, token: &str, client_ip: &str) -> CsrfConsumeResult {
        // 先 get 校验、匹配后再 take 移除：
        // 避免"take 后回写 ttl=None 导致 30 分钟有效期变成永久条目"的内存泄漏。
        // 并发窗口内的重复消费由第二次 take 返回 None 兜底为 NotFound，语义安全。
        let bound = self.csrf_token_cache.get(&token.to_string());
        match bound {
            Some((session_id, bound_ip)) => {
                if bound_ip.as_str() != client_ip {
                    tracing::warn!(
                        client_ip = %client_ip,
                        bound_ip = %bound_ip,
                        "CSRF Token 绑定的 IP 与请求 IP 不一致（保留原 Token）"
                    );
                    return CsrfConsumeResult::IpMismatch;
                }
                // 二次 take 完成一次性消费语义；None 说明被并发请求抢先消费
                match self.csrf_token_cache.take(&token.to_string()) {
                    Some(_) => {}
                    None => return CsrfConsumeResult::NotFound,
                }
                let _ = session_id;
                // 清理反向索引（找到 user_id 并移除）。
                // 此处需要按 value 查找 key，DashMap 不直接支持；采用遍历策略。
                // 对于单次 CSRF 校验，遍历成本可接受（缓存条目数远小于用户会话数）。
                // 先在独立的代码块中收集 to_remove，避免与后面的 remove 借用冲突。
                let to_remove: Option<i32> = {
                    let mut found: Option<i32> = None;
                    for entry in self.csrf_user_index.iter() {
                        if entry.value() == token {
                            found = Some(*entry.key());
                            break;
                        }
                    }
                    found
                };
                if let Some(uid) = to_remove {
                    self.csrf_user_index.remove(&uid);
                }
                CsrfConsumeResult::Ok
            }
            None => CsrfConsumeResult::NotFound,
        }
    }

    /// 清除指定用户的旧 CSRF Token（强制轮换，Wave 3 安全漏洞 #7 修复）
    /// 重新登录时调用使历史 token 立即失效防多设备残留；返回 true=清除至少一个，false=无活跃 token（首次登录）
    ///
    /// 多会话共存说明：E2E CI 34 个分片共享同一 e2e_admin 并发登录，按 user_id 全清
    /// 会导致分片间互相踢 token（踢踏雪崩：被踢分片重登又踢别人）。改为保留
    /// csrf_token_cache 中同 TTL 的旧 token 主体（各自随 30min TTL 自然过期），
    /// 仅清除反向索引（index 只服务于"最近一次登录"语义），实现：
    /// - 单会话场景：旧行为等价（旧 token 仍消费即失效——一次性消费语义不变）
    /// - 多会话场景：各分片 token 独立有效，互不干扰
    pub fn clear_old_csrf_token_for_user(&self, user_id: i32) -> bool {
        // 仅移除反向索引映射，保留 csrf_token_cache 中的旧 token（TTL 自然过期）。
        // 旧 token 仍受一次性消费 + IP 绑定约束，安全性不变；
        // 多会话并发时不再互相清除对方的有效 token。
        self.csrf_user_index.remove(&user_id).is_some()
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
        // csrf_user_index 是原始 DashMap（无统计字段），无需重置
    }
}
