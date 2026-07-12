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

    /// 一次性获取并移除缓存项（rotation 模式：用于 CSRF Token 等一次性凭证）
    ///
    /// 与 [`get`](Cache::get) 不同，本方法在返回缓存值的同时会从底层存储中删除对应键，
    /// 用于实现 token rotation：同一 token 只能被消费一次。
    /// 若键不存在或已过期，则返回 `None` 并按 miss 计入统计。
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
    pub token_blacklist: Arc<MemoryCache<String, bool>>,
    /// CSRF Token 缓存：key=csrf_token, value=(session_id, ip_address)。
    /// IP 绑定用于防御 CSRF 窃取后的跨 IP 重放（Wave 3 安全漏洞 #7）。
    pub csrf_token_cache: Arc<MemoryCache<String, (String, String)>>,
    /// CSRF Token 反向索引：key=user_id, value=该用户当前活跃的 csrf_token。
    /// 使用原始 DashMap（不经过 MemoryCache 包装），便于按 value 反查与就地清理。
    /// 用于登录时强制轮换（清除旧 token），防止多设备登录时旧 token 长期残留。
    pub csrf_user_index: DashMap<i32, String>,
}

/// CSRF Token 消费结果
///
/// Wave 3 安全漏洞 #7 引入：消费时区分 IP 不匹配、缺失/过期两种失败原因，
/// 使前端能基于业务码做差异化处理（IP 失配可引导重新登录，缺失/过期则提示刷新）。
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

    /// 获取 Token 黑名单缓存
    pub fn get_token_blacklist(&self) -> Arc<MemoryCache<String, bool>> {
        self.token_blacklist.clone()
    }

    /// 获取 CSRF Token 缓存
    ///
    /// 直接访问底层缓存的场景较少；优先使用 [AppCache::set_csrf_token] /
    /// [AppCache::consume_csrf_token] / [AppCache::clear_old_csrf_token_for_user]
    /// 这些高层 API（封装了 IP 绑定 + 强制轮换逻辑）。
    /// 此方法保留用于测试与内部维护。
    pub fn get_csrf_token_cache(&self) -> Arc<MemoryCache<String, (String, String)>> {
        self.csrf_token_cache.clone()
    }

    /// 获取 CSRF Token 反向索引（user_id → csrf_token）
    ///
    /// 优先使用 [AppCache::clear_old_csrf_token_for_user] 访问；此方法保留供
    /// 测试与内部维护。
    pub fn get_csrf_user_index(&self) -> &DashMap<i32, String> {
        &self.csrf_user_index
    }

    /// 写入 CSRF Token（含 IP 绑定 + 反向索引维护）
    ///
    /// Wave 3 安全漏洞 #7 修复：
    /// - 缓存值 = `(session_id, ip_address)` 元组，IP 用于消费时校验。
    /// - 反向索引 `user_id → csrf_token` 记录最新 token，便于后续登录时
    ///   通过 [AppCache::clear_old_csrf_token_for_user] 清除旧 token。
    /// - 旧 token（若存在）由调用方在写入前先调用 [AppCache::clear_old_csrf_token_for_user] 清除。
    ///
    /// # 参数
    /// - `token`: CSRF Token 字符串（UUID）
    /// - `session_id`: 当前 JWT session_id
    /// - `ip_address`: 客户端 IP（来自 [AuditContext::ip_address]）
    /// - `user_id`: 用户 ID（用于反向索引）
    /// - `ttl`: 过期时长；`None` 时使用 [CSRF_TOKEN_DEFAULT_TTL_SECS]
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
    ///
    /// 行为：
    /// 1. 缓存中找不到 token → `NotFound`。
    /// 2. token 存在但 IP 不匹配 → `IpMismatch`（**不消费**，避免攻击者通过
    ///    IP 探测消耗合法用户的 token）。
    /// 3. token 存在且 IP 匹配 → `Ok`（消费：从缓存移除并清理反向索引）。
    ///
    /// IP 校验失败不消费的设计权衡：若消费则攻击者可以通过重复请求消耗掉
    /// 合法用户的 token，触发拒绝服务（DoS）。保留 token 让合法用户仍可使用。
    ///
    /// # 参数
    /// - `token`: 请求头 `X-CSRF-Token` 携带的 token
    /// - `client_ip`: 当前请求的客户端 IP（与登录时记录的 IP 对比）
    pub fn consume_csrf_token(&self, token: &str, client_ip: &str) -> CsrfConsumeResult {
        // 使用 take 实现"一次性消费"语义：成功匹配后从缓存移除
        match self.csrf_token_cache.take(&token.to_string()) {
            Some((session_id, bound_ip)) => {
                if bound_ip != client_ip {
                    // IP 不匹配：把 token 放回缓存（保留合法用户的可用性）
                    // 重新设置时不再更新反向索引（索引仍指向该 token）。
                    self.csrf_token_cache
                        .set(token.to_string(), (session_id, bound_ip), None);
                    return CsrfConsumeResult::IpMismatch;
                }
                // IP 匹配：清理反向索引（找到 user_id 并移除）。
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

    /// 清除指定用户的旧 CSRF Token（强制轮换）
    ///
    /// Wave 3 安全漏洞 #7 修复：用户重新登录时调用此方法，使该用户的历史 CSRF
    /// Token 立即失效（即便 TTL 未到），防止多设备/多标签登录时旧 token 长期残留。
    ///
    /// # 返回
    /// - `true`: 清除了至少一个旧 token
    /// - `false`: 该用户无活跃 CSRF Token（首次登录场景）
    pub fn clear_old_csrf_token_for_user(&self, user_id: i32) -> bool {
        if let Some((_, old_token)) = self.csrf_user_index.remove(&user_id) {
            // 同时清除 csrf_token_cache 中的旧 token 主体
            self.csrf_token_cache.storage.remove(&old_token);
            return true;
        }
        false
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

/// 写入 CSRF Token 单元测试（不经过 AppState，AppCache::new() 即用）
// 批次 343 v11 复审 P3 修复：移除 #[allow(unused_imports)]，use super::* 已在测试中使用
#[cfg(test)]
mod csrf_token_tests {
    use super::*;

    /// 单元测试：set_csrf_token 写入 + consume_csrf_token 匹配 IP 成功
    #[test]
    fn test_set_csrf_token_then_consume_with_matching_ip() {
        let cache = AppCache::new();
        let token = "test-csrf-token-001".to_string();
        cache.set_csrf_token(
            token.clone(),
            "session-A".to_string(),
            "203.0.113.10".to_string(),
            42,
            None,
        );
        let result = cache.consume_csrf_token(&token, "203.0.113.10");
        assert_eq!(
            result,
            CsrfConsumeResult::Ok,
            "IP 匹配应返回 Ok，实际: {:?}",
            result
        );
    }

    /// 单元测试：consume_csrf_token IP 不匹配时返回 IpMismatch，且 token 仍保留
    #[test]
    fn test_consume_csrf_token_with_mismatched_ip_returns_ip_mismatch_and_keeps_token() {
        let cache = AppCache::new();
        let token = "test-csrf-token-002".to_string();
        cache.set_csrf_token(
            token.clone(),
            "session-B".to_string(),
            "203.0.113.20".to_string(),
            43,
            None,
        );

        // 第一次消费：IP 不匹配 → IpMismatch
        let r1 = cache.consume_csrf_token(&token, "198.51.100.99");
        assert_eq!(
            r1,
            CsrfConsumeResult::IpMismatch,
            "IP 不匹配应返回 IpMismatch，实际: {:?}",
            r1
        );

        // 第二次消费：使用正确 IP → 仍能消费成功（IP 不匹配不消费 token）
        let r2 = cache.consume_csrf_token(&token, "203.0.113.20");
        assert_eq!(
            r2,
            CsrfConsumeResult::Ok,
            "IP 不匹配不应消耗 token，原 IP 仍可消费，实际: {:?}",
            r2
        );
    }

    /// 单元测试：clear_old_csrf_token_for_user 清除用户旧 token
    #[test]
    fn test_clear_old_csrf_token_for_user_invalidates_old_token() {
        let cache = AppCache::new();
        let old_token = "old-csrf-token-003".to_string();
        cache.set_csrf_token(
            old_token.clone(),
            "session-C".to_string(),
            "203.0.113.30".to_string(),
            44,
            None,
        );

        // 强制轮换
        let cleared = cache.clear_old_csrf_token_for_user(44);
        assert!(cleared, "应返回 true（存在旧 token）");

        // 旧 token 已失效
        let r = cache.consume_csrf_token(&old_token, "203.0.113.30");
        assert_eq!(
            r,
            CsrfConsumeResult::NotFound,
            "清除后旧 token 应返回 NotFound，实际: {:?}",
            r
        );

        // 清除不存在的用户 → false
        let cleared_none = cache.clear_old_csrf_token_for_user(999);
        assert!(!cleared_none, "无活跃 token 的用户应返回 false");
    }

    /// 单元测试：IP 匹配消费后，反向索引同步清理（不再泄漏 user_id → token）
    #[test]
    fn test_consume_cleans_up_user_index() {
        let cache = AppCache::new();
        let token = "test-csrf-token-004".to_string();
        cache.set_csrf_token(
            token.clone(),
            "session-D".to_string(),
            "203.0.113.40".to_string(),
            45,
            None,
        );
        assert!(
            cache.csrf_user_index.contains_key(&45),
            "set 后反向索引应包含 user_id=45"
        );
        let _ = cache.consume_csrf_token(&token, "203.0.113.40");
        assert!(
            !cache.csrf_user_index.contains_key(&45),
            "consume 后反向索引应移除 user_id=45"
        );
    }
}
