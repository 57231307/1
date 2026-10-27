// dead_code 检查已开启；当前所有 pub API 均已被业务引用（AppCache/MemoryCache/Cache trait/CacheStats）。
// 私有项 CachedValue<T> 内部使用。

use crate::middleware::audit_context::CLIENT_IP_UNRESOLVED;
use dashmap::DashMap;
use std::collections::HashSet;
use std::net::IpAddr;
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

/// IP 漂移时旧 Token 的宽限窗口（秒）。
/// 取值依据：轮换（rotation）与 IP 重绑发生在同一个请求上，此后 60 秒内仍可能带着旧 Token 的
/// 是在途并发请求（浏览器一次提交可能并行发出多个写请求，移动端切网时队列里还压着未发出的请求）。
/// 立即作旧会让这批请求全部落入"已消费"分支，靠恢复重放才能回来，代价是把一次正常的换网
/// 放大成一轮 403 风暴。60 秒远小于 1800 秒的 Token 生命周期，既覆盖在途窗口，
/// 又不给被窃 Token 额外拉长可用期（OWASP 关于 Token 轮换建议保留一个有界的重叠窗口，而不是无限期双活）。
pub const CSRF_IP_DRIFT_GRACE_SECS: u64 = 60;

/// Token 未绑定任何 JWT 会话时的占位值。
/// 只有"签发方拿不到会话标识"这一种情况会写入该值，此时会话维度不参与核验（用户维度仍然核验）。
pub const CSRF_SESSION_UNBOUND: &str = "unbound";

/// CSRF Token 的绑定记录。
/// 拒绝依据是"用户 + 会话"两项：Token 记录的身份必须与当前认证上下文一致。
/// `ip_address` 只是漂移信号（移动与 NAT 客户端必然变址），不再参与拒绝判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrfTokenBinding {
    /// 签发时的 JWT 会话标识（登录与刷新链路来自 AppClaims.session_id）
    pub session_id: String,
    /// 签发时观测到的客户端 IP（仅供漂移比较与审计留痕）
    pub ip_address: String,
    /// 签发时的用户 ID（消费时与 AuthContext.user_id 强比对）
    pub user_id: i32,
}

/// CSRF Token 绑定核验失败的分类。三类都必须拒绝请求，且都不得消费 Token
/// （否则攻击者可以用错配的 Token 把受害者的有效 Token 消耗掉，构成 DoS）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsrfBindingMismatch {
    /// 请求没有认证上下文，无法核验 Token 的用户与会话归属
    Unauthenticated,
    /// Token 绑定的用户与当前登录用户不一致（跨用户盗用面）
    UserMismatch,
    /// Token 绑定的会话与当前 JWT 会话不一致（同用户跨会话盗用面）
    SessionMismatch,
}

/// CSRF Token 消费结果
/// 拒绝依据只有绑定（用户 / 会话）；IP 差异降级为"漂移信号"，由调用方原子重发 Token 并留痕后放行
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsrfConsumeResult {
    /// 消费成功（token 有效 + 绑定核验通过，已从缓存移除）
    Ok,
    /// 消费成功但请求 IP 与绑定 IP 不同：调用方必须重发绑定到新 IP 的 Token 并写审计留痕。
    /// 旧 Token 未被立即作废，已在 [CSRF_IP_DRIFT_GRACE_SECS] 窗口内重新挂到缓存供在途请求使用。
    OkIpDrift {
        /// 漂移前的绑定 IP（供审计留痕）
        previous_ip: String,
    },
    /// Token 不存在或已过期
    NotFound,
    /// Token 存在但绑定核验不通过（疑似盗用），Token 原样保留
    BindingMismatch(CsrfBindingMismatch),
}

/// 判断一条 IP 值是否属于不可用（占位串或空串）。
/// 不可用的值不构成客户端身份证据，既不能用来判"换址"，也不能用来判"同一地址"。
fn is_unresolved_ip(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case(CLIENT_IP_UNRESOLVED)
}

/// 把一个 IP 字符串折算成可比较的 IPv6 段组（IPv4 走 v4-mapped 形态）。
/// 目的：`1.2.3.4` 与 `::ffff:1.2.3.4` 是同一个地址，必须判为相等；
/// 直接按 IpAddr 枚举比较会因 V4 与 V6 是两个 variant 而误判成换址。
fn ip_comparable(value: &str) -> Option<[u16; 8]> {
    match value.trim().parse::<IpAddr>().ok()? {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            Some([
                0,
                0,
                0,
                0,
                0,
                0xffff,
                ((o[0] as u16) << 8) | o[1] as u16,
                ((o[2] as u16) << 8) | o[3] as u16,
            ])
        }
        IpAddr::V6(v6) => Some(v6.segments()),
    }
}

/// IP 漂移判据：绑定 IP 与请求 IP 是否代表"不同的客户端地址"。公开供活体锁直接点名判据本身。
/// 规则（全部服务于同一个目标：只有真换址才算漂移，格式差异与占位值都不算）：
/// - 任一侧是占位或空值, 不算漂移。直连与测试链路里 CLIENT_IP_UNRESOLVED 与真实 IP 的组合
///   属于"观测能力差异"，不是客户端换了网络。
/// - 两侧都能解析, 按规范化段组比较。IPv4 与 v4-mapped IPv6 写法、前导零、大小写差异
///   都不会被误判成换址。
/// - 解析不了的两侧, 按去空白小写后的字面比较（非 IP 形态的值走这条）。
pub fn csrf_ip_drifted(bound_ip: &str, request_ip: &str) -> bool {
    if is_unresolved_ip(bound_ip) || is_unresolved_ip(request_ip) {
        return false;
    }
    match (ip_comparable(bound_ip), ip_comparable(request_ip)) {
        (Some(a), Some(b)) => a != b,
        _ => bound_ip.trim().to_ascii_lowercase() != request_ip.trim().to_ascii_lowercase(),
    }
}

/// 绑定核验判据：返回不一致的分类，全部一致返回 None。公开供活体锁直接点名核验次序。
/// 用户维度必核（拿不到认证上下文即视为核验不通过）；
/// 会话维度在"token 已绑定真实会话"且"当前请求能解析出会话"时必核，
/// token 侧是 CSRF_SESSION_UNBOUND 占位时不参与比对（此时用户维度仍然必核）。
pub fn csrf_binding_mismatch(
    binding: &CsrfTokenBinding,
    expected_user_id: Option<i32>,
    expected_session_id: Option<&str>,
) -> Option<CsrfBindingMismatch> {
    let Some(user_id) = expected_user_id else {
        return Some(CsrfBindingMismatch::Unauthenticated);
    };
    if binding.user_id != user_id {
        return Some(CsrfBindingMismatch::UserMismatch);
    }
    match expected_session_id {
        Some(session_id)
            if binding.session_id != CSRF_SESSION_UNBOUND && binding.session_id != session_id =>
        {
            Some(CsrfBindingMismatch::SessionMismatch)
        }
        _ => None,
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
    /// 缺陷 3.1 修复：BI 多维分析聚合结果缓存（5 分钟 TTL）
    pub bi_cache: Arc<MemoryCache<String, serde_json::Value>>,
    pub token_blacklist: Arc<MemoryCache<String, bool>>,
    /// CSRF Token 缓存：key=csrf_token，value=CsrfTokenBinding（会话 + 用户 + 观测 IP）。
    /// 拒绝依据是会话与用户绑定；观测 IP 只用于漂移判定与审计留痕。
    pub csrf_token_cache: Arc<MemoryCache<String, CsrfTokenBinding>>,
    /// CSRF Token 用户索引（key=user_id, value=该用户全部存活 token 集合；
    /// 登录/刷新强制轮换时按用户整集清除，实现"重新登录使历史 token 立即失效"，
    /// 防多设备旧 token 残留，Wave 3 安全漏洞 #7）
    pub csrf_user_index: DashMap<i32, HashSet<String>>,
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
    /// 优先使用 set_csrf_token / consume_csrf_token / clear_old_csrf_token_for_user 高层 API（封装绑定核验 + 强制轮换）
    pub fn get_csrf_token_cache(&self) -> Arc<MemoryCache<String, CsrfTokenBinding>> {
        self.csrf_token_cache.clone()
    }

    /// 写入 CSRF Token（会话与用户绑定 + 观测 IP 留痕 + 用户索引维护）
    /// 缓存值为 CsrfTokenBinding：会话与用户是消费时的拒绝依据，IP 只作漂移信号。
    /// 参数：token/session_id/ip_address/user_id/ttl（None 用 CSRF_TOKEN_DEFAULT_TTL_SECS）
    pub fn set_csrf_token(
        &self,
        token: String,
        session_id: String,
        ip_address: String,
        user_id: i32,
        ttl: Option<Duration>,
    ) {
        let effective_ttl = ttl.unwrap_or(Duration::from_secs(CSRF_TOKEN_DEFAULT_TTL_SECS));
        self.install_csrf_token(token, session_id, ip_address, user_id, effective_ttl);
        // 用户索引不显式 TTL：其生命周期由 clear_old_csrf_token_for_user（登录/刷新轮换时
        // 整集清除）与 consume_csrf_token（消费后从集合移除）驱动；残留的已过期 token 集合项
        // 在该用户下次轮换时随整集清理，不会无限增长
    }

    /// 把一条 CSRF Token 绑定写入缓存并登记用户索引。
    /// set_csrf_token 的正常签发与 IP 漂移的宽限副本共用这一条路径，保证两者都必然进用户索引，
    /// 从而都能被登录/刷新的强制轮换整集清除。
    fn install_csrf_token(
        &self,
        token: String,
        session_id: String,
        ip_address: String,
        user_id: i32,
        ttl: Duration,
    ) {
        self.csrf_token_cache.set(
            token.clone(),
            CsrfTokenBinding {
                session_id,
                ip_address,
                user_id,
            },
            Some(ttl),
        );
        self.csrf_user_index
            .entry(user_id)
            .or_default()
            .insert(token);
    }

    /// 从用户索引里摘掉一个已消费的 token；集合空时移除用户条目，避免空集合堆积。
    fn detach_csrf_token_from_user_index(&self, token: &str, user_id: i32) {
        let mut should_remove_user = false;
        if let Some(mut set) = self.csrf_user_index.get_mut(&user_id) {
            set.remove(token);
            should_remove_user = set.is_empty();
        }
        if should_remove_user {
            self.csrf_user_index.remove(&user_id);
        }
    }

    /// 校验并一次性消费 CSRF Token（默认宽限窗口，业务链路用这个）
    /// 参数：token(X-CSRF-Token 头)/client_ip/expected_user_id/expected_session_id
    pub fn consume_csrf_token(
        &self,
        token: &str,
        client_ip: &str,
        expected_user_id: Option<i32>,
        expected_session_id: Option<&str>,
    ) -> CsrfConsumeResult {
        self.consume_csrf_token_with_grace(
            token,
            client_ip,
            expected_user_id,
            expected_session_id,
            Duration::from_secs(CSRF_IP_DRIFT_GRACE_SECS),
        )
    }

    /// 消费判定的真正实现，宽限窗口作为入参暴露给测试（线上调用方一律走 consume_csrf_token 的常量窗口）
    /// 判定次序：
    /// 1. 取不到 token 或已过期，返回 NotFound。
    /// 2. 绑定核验（用户必核，会话在两侧都已知时必核）。任一不符直接返回 BindingMismatch，
    ///    token 原样保留：核验失败的一方不得消耗掉受害方的有效 token。
    /// 3. 核验通过后 take 完成一次性消费；take 落空说明被并发请求抢先，返回 NotFound。
    /// 4. 消费成功但 IP 相对绑定值漂移时，按 grace_ttl 把旧 token 以"新 IP + 同会话同用户"
    ///    重新挂回缓存并登记索引（在途并发请求仍可消费它），同时返回 OkIpDrift 让调用方
    ///    重发新 token 并写审计留痕。IP 漂移本身不构成拒绝，也不放宽任何绑定核验。
    pub fn consume_csrf_token_with_grace(
        &self,
        token: &str,
        client_ip: &str,
        expected_user_id: Option<i32>,
        expected_session_id: Option<&str>,
        grace_ttl: Duration,
    ) -> CsrfConsumeResult {
        let Some(binding) = self.csrf_token_cache.get(&token.to_string()) else {
            return CsrfConsumeResult::NotFound;
        };
        let mismatch = csrf_binding_mismatch(&binding, expected_user_id, expected_session_id);
        if let Some(kind) = mismatch {
            tracing::warn!(
                client_ip = %client_ip,
                bound_user_id = binding.user_id,
                expected_user_id = ?expected_user_id,
                bound_session = %binding.session_id,
                expected_session = ?expected_session_id,
                mismatch = ?kind,
                "CSRF Token 绑定核验不通过，拒绝且不消费 Token"
            );
            return CsrfConsumeResult::BindingMismatch(kind);
        }

        // 二次 take 完成一次性消费语义；None 说明被并发请求抢先消费
        let Some(consumed) = self.csrf_token_cache.take(&token.to_string()) else {
            return CsrfConsumeResult::NotFound;
        };
        self.detach_csrf_token_from_user_index(token, consumed.user_id);

        if !csrf_ip_drifted(&consumed.ip_address, client_ip) {
            return CsrfConsumeResult::Ok;
        }

        // 宽限副本：旧 token 不立刻作废，改绑到本次请求观测到的 IP 后按短窗口保留。
        // 会话与用户绑定原样继承，因此副本只能在同一会话同一用户下使用，跨会话跨用户仍被拒。
        self.install_csrf_token(
            token.to_string(),
            consumed.session_id.clone(),
            client_ip.to_string(),
            consumed.user_id,
            grace_ttl,
        );
        tracing::warn!(
            bound_ip = %consumed.ip_address,
            request_ip = %client_ip,
            user_id = consumed.user_id,
            session_id = %consumed.session_id,
            "CSRF Token 观测 IP 发生漂移，放行并重发 Token（旧 Token 进入宽限窗口）"
        );
        CsrfConsumeResult::OkIpDrift {
            previous_ip: consumed.ip_address.clone(),
        }
    }

    /// 强制轮换：清除指定用户的全部活跃 CSRF Token（Wave 3 安全漏洞 #7 修复）
    /// 重新登录/刷新时调用使历史 token 立即失效防多设备残留；返回 true=清除了至少一个活跃 token，false=无活跃 token（首次登录）
    ///
    /// 多会话语义说明：重新登录使该用户全部旧 token 失效是本函数的安全本意
    /// （旧 token 一旦被踢，持有方下一次请求即收到 CSRF_TOKEN_INVALID，重新登录即可恢复）。
    /// E2E CI 各分片使用独立分片账号（e2e_admin_s{N}，见 e2e/global-setup.ts），
    /// 按用户清除不会跨分片踢 token；同账号多设备场景下，后登录方踢掉先登录方的
    /// token 属预期安全行为（后者随下次请求自动恢复会话）。
    pub fn clear_old_csrf_token_for_user(&self, user_id: i32) -> bool {
        // 取走该用户的全部存活 token 并从 csrf_token_cache 逐个移除，
        // 实现"重新登录使历史 token 立即失效"
        match self.csrf_user_index.remove(&user_id) {
            Some((_, tokens)) => {
                let mut removed_any = false;
                for t in tokens {
                    if self.csrf_token_cache.take(&t).is_some() {
                        removed_any = true;
                    }
                }
                removed_any
            }
            None => false,
        }
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
