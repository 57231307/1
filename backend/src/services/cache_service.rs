//! 性能优化缓存服务（P4-1 性能优化）
//!
//! 提供进程内 LRU + TTL 缓存，专为以下场景设计：
//! - Dashboard 热点数据（订单数 / 库存 / 应收应付汇总）
//! - 配置类数据（菜单 / 权限 / 字典）
//! - 报表聚合查询
//!
//! ## 关键设计
//!
//! - **多租户隔离**：key 必须以 `tenant:{id}:` 开头，避免跨租户数据串味
//! - **TTL 失效**：通过 `moka::future::Cache` 的 expire_after 实现
//! - **容量上限**：默认 10000 条，超过 LRU 淘汰
//! - **指标埋点**：`hit` / `miss` 计数
//!
//! ## 关闭缓存
//!
//! 缓存仅作为加速层；查询逻辑必须保证绕过缓存也能返回正确数据。
//! 关闭方式：把 `CACHE_ENABLED=false` 写入环境变量即可。

use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
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
}

impl CacheService {
    /// 创建默认配置缓存（容量 10000、TTL 60s）
    #[allow(dead_code)] // TODO(tech-debt): 业务模块接入后移除
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
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        if !self.enabled {
            return None;
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
    pub async fn set(&self, key: String, value: Vec<u8>) {
        if !self.enabled {
            return;
        }
        self.inner.insert(key, value).await;
    }

    /// 带自定义 TTL 的写入
    #[allow(dead_code)] // TODO(tech-debt): 业务模块接入后移除
    pub async fn set_with_ttl(&self, key: String, value: Vec<u8>, ttl: Duration) {
        if !self.enabled {
            return;
        }
        // moka 顶层只支持统一 TTL；自定义 TTL 通过 key 后缀模拟
        // 此处简化：忽略 ttl，统一使用 default_ttl
        let _ = ttl;
        self.inner.insert(key, value).await;
    }

    /// 失效指定 key
    #[allow(dead_code)] // TODO(tech-debt): 业务模块接入后移除
    pub async fn invalidate(&self, key: &str) {
        self.inner.invalidate(key).await;
    }

    /// 按前缀失效（多租户场景：失效某租户全部缓存）
    pub async fn invalidate_prefix(&self, prefix: &str) {
        // moka 不支持原生前缀失效，采用全量失效的近似
        // 业务侧建议使用 `tenant:{id}:module:` 命名空间，失效时使用整租户清理
        let _ = prefix;
        self.inner.invalidate_all();
    }

    /// 当前统计
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// 默认 TTL
    #[allow(dead_code)] // TODO(tech-debt): 业务模块接入后移除
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }
}

#[allow(dead_code)] // TODO(tech-debt): 业务模块接入后移除
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
}
