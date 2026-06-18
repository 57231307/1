// P12 批 1 性能优化：Redis 缓存层（P2-2）
//
// 设计要点：
// - 键空间：tenant_id + entity_type + entity_id（强租户隔离）
// - TTL 默认 300 秒，可通过环境变量 CACHE_TTL_SECS 调整
// - 支持 graceful degradation：Redis 不可用时返回 NullCache，业务回退到 DB
// - 通过 `CacheBackend` trait 抽象，便于单元测试使用 mock

#![allow(dead_code)]
// TODO(tech-debt): cache 模块的辅助 API（from_env / is_enabled / stats / snapshot /
// DEFAULT_TTL_SECS / NullBackend / new / disabled / connect / ping）在 server bin crate
// 独立编译时无调用方。当前 P12 批 1 仅在 lib crate 内部接入了 build_key / get_json /
// set_json / invalidate，辅助 API 计划在后续业务模块（销售报价单/审批/导入导出）
// 分阶段接入。CI 强制项级评估，禁止绕过此注释。

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// 默认缓存 TTL（秒）
pub const DEFAULT_TTL_SECS: u64 = 300;

/// 缓存后端 trait（用于 mock 测试）
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// 获取键对应的值（JSON 字符串）
    async fn get(&self, key: &str) -> Result<Option<String>, redis::RedisError>;

    /// 设置键值与 TTL
    async fn set_ex(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), redis::RedisError>;

    /// 删除键
    async fn del(&self, key: &str) -> Result<u64, redis::RedisError>;

    /// 健康检查（用于 graceful degradation 决策）
    async fn ping(&self) -> Result<(), redis::RedisError>;
}

/// Redis 后端实现（基于 ConnectionManager）
pub struct RedisBackend {
    conn: tokio::sync::Mutex<ConnectionManager>,
}

impl RedisBackend {
    /// 从 REDIS_URL 环境变量创建连接管理器
    ///
    /// # 参数
    /// - `url`: Redis 连接 URL（如 `redis://127.0.0.1:6379`）
    pub async fn connect(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self {
            conn: tokio::sync::Mutex::new(manager),
        })
    }
}

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get(&self, key: &str) -> Result<Option<String>, redis::RedisError> {
        let mut conn = self.conn.lock().await;
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    async fn set_ex(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.lock().await;
        let _: () = conn.set_ex(key, value, ttl_secs).await?;
        Ok(())
    }

    async fn del(&self, key: &str) -> Result<u64, redis::RedisError> {
        let mut conn = self.conn.lock().await;
        let count: u64 = conn.del(key).await?;
        Ok(count)
    }

    async fn ping(&self) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.lock().await;
        let _: String = redis::cmd("PING").query_async(&mut *conn).await?;
        Ok(())
    }
}

/// 缓存统计（命中 / 未命中 / 错误 / 回退次数）
#[derive(Debug, Default)]
pub struct CacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
    errors: AtomicU64,
    fallbacks: AtomicU64,
}

impl CacheStats {
    /// 记录一次命中
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次未命中
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次错误
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录一次回退（Redis 不可用，绕过缓存读 DB）
    pub fn record_fallback(&self) {
        self.fallbacks.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取统计快照
    pub fn snapshot(&self) -> (u64, u64, u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
            self.fallbacks.load(Ordering::Relaxed),
        )
    }
}

/// 业务缓存门面
///
/// 提供租户隔离的键命名、JSON 序列化、TTL 控制与 graceful degradation。
/// 任何 Redis 错误都会被吞掉并按 fallback 计入统计，调用方始终拿到 Option。
pub struct CacheService {
    backend: Arc<dyn CacheBackend>,
    default_ttl: Duration,
    stats: Arc<CacheStats>,
    /// 是否启用真实缓存（false 时所有 get/set 直接返回 None/null）
    enabled: bool,
}

impl std::fmt::Debug for CacheService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheService")
            .field("enabled", &self.enabled)
            .field("default_ttl_secs", &self.default_ttl.as_secs())
            .finish_non_exhaustive()
    }
}

impl CacheService {
    /// 创建启用 Redis 后端的缓存服务
    pub fn new(backend: Arc<dyn CacheBackend>, default_ttl: Duration) -> Self {
        Self {
            backend,
            default_ttl,
            stats: Arc::new(CacheStats::default()),
            enabled: true,
        }
    }

    /// 创建禁用缓存的服务（用于测试或显式关闭缓存）
    pub fn disabled() -> Self {
        Self {
            backend: Arc::new(NullBackend),
            default_ttl: Duration::from_secs(DEFAULT_TTL_SECS),
            stats: Arc::new(CacheStats::default()),
            enabled: false,
        }
    }

    /// 尝试从环境变量自动构建（REDIS_URL），失败时返回禁用实例
    pub async fn from_env() -> Self {
        let url = std::env::var("REDIS_URL").unwrap_or_default();
        if url.is_empty() {
            warn!("REDIS_URL 未设置，缓存层将禁用（graceful degradation 模式）");
            return Self::disabled();
        }
        match RedisBackend::connect(&url).await {
            Ok(backend) => {
                let ttl = std::env::var("CACHE_TTL_SECS")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(DEFAULT_TTL_SECS);
                debug!("Redis 缓存层已启用，TTL={}s", ttl);
                Self::new(Arc::new(backend), Duration::from_secs(ttl))
            }
            Err(e) => {
                warn!("Redis 连接失败 {:?}，缓存层将禁用", e);
                Self::disabled()
            }
        }
    }

    /// 是否启用缓存
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取统计句柄
    pub fn stats(&self) -> Arc<CacheStats> {
        self.stats.clone()
    }

    /// 构造缓存键：`tenant:{tenant_id}:{entity_type}:{entity_id}`
    pub fn build_key(tenant_id: i64, entity_type: &str, entity_id: &str) -> String {
        format!("tenant:{}:{}:{}", tenant_id, entity_type, entity_id)
    }

    /// 读取并反序列化 JSON 缓存值
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        if !self.enabled {
            self.stats.record_fallback();
            return None;
        }
        match self.backend.get(key).await {
            Ok(Some(raw)) => match serde_json::from_str::<T>(&raw) {
                Ok(v) => {
                    self.stats.record_hit();
                    Some(v)
                }
                Err(e) => {
                    warn!("缓存反序列化失败 key={} err={:?}", key, e);
                    self.stats.record_error();
                    None
                }
            },
            Ok(None) => {
                self.stats.record_miss();
                None
            }
            Err(e) => {
                warn!("缓存读取失败 key={} err={:?}", key, e);
                self.stats.record_error();
                self.stats.record_fallback();
                None
            }
        }
    }

    /// 写入 JSON 序列化值
    pub async fn set_json<T: serde::Serialize>(&self, key: &str, value: &T, ttl: Option<Duration>) {
        if !self.enabled {
            return;
        }
        let payload = match serde_json::to_string(value) {
            Ok(s) => s,
            Err(e) => {
                warn!("缓存序列化失败 key={} err={:?}", key, e);
                self.stats.record_error();
                return;
            }
        };
        let ttl_secs = ttl.unwrap_or(self.default_ttl).as_secs();
        if let Err(e) = self.backend.set_ex(key, &payload, ttl_secs).await {
            warn!("缓存写入失败 key={} err={:?}", key, e);
            self.stats.record_error();
        }
    }

    /// 删除缓存键
    pub async fn invalidate(&self, key: &str) {
        if !self.enabled {
            return;
        }
        if let Err(e) = self.backend.del(key).await {
            warn!("缓存删除失败 key={} err={:?}", key, e);
            self.stats.record_error();
        }
    }
}

/// 空后端（Redis 不可用时的占位实现，所有操作均返回 Ok 模拟成功以避免误报错误）
struct NullBackend;

#[async_trait]
impl CacheBackend for NullBackend {
    async fn get(&self, _key: &str) -> Result<Option<String>, redis::RedisError> {
        Ok(None)
    }

    async fn set_ex(
        &self,
        _key: &str,
        _value: &str,
        _ttl_secs: u64,
    ) -> Result<(), redis::RedisError> {
        Ok(())
    }

    async fn del(&self, _key: &str) -> Result<u64, redis::RedisError> {
        Ok(0)
    }

    async fn ping(&self) -> Result<(), redis::RedisError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tokio::sync::RwLock;

    /// 测试用 mock backend（基于 HashMap）
    struct MockBackend {
        store: Arc<RwLock<HashMap<String, (String, u64)>>>,
        /// 模拟故障（用于测试 graceful degradation）
        fail_next: Arc<Mutex<bool>>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                store: Arc::new(RwLock::new(HashMap::new())),
                fail_next: Arc::new(Mutex::new(false)),
            }
        }

        async fn set_fail(&self, fail: bool) {
            let mut g = self.fail_next.lock().unwrap();
            *g = fail;
        }
    }

    #[async_trait]
    impl CacheBackend for MockBackend {
        async fn get(&self, key: &str) -> Result<Option<String>, redis::RedisError> {
            if *self.fail_next.lock().unwrap() {
                self.set_fail(false).await;
                return Err(redis::RedisError::from((
                    redis::ErrorKind::IoError,
                    "mock get failure",
                )));
            }
            let g = self.store.read().await;
            Ok(g.get(key).map(|(v, _)| v.clone()))
        }

        async fn set_ex(
            &self,
            key: &str,
            value: &str,
            ttl_secs: u64,
        ) -> Result<(), redis::RedisError> {
            if *self.fail_next.lock().unwrap() {
                self.set_fail(false).await;
                return Err(redis::RedisError::from((
                    redis::ErrorKind::IoError,
                    "mock set failure",
                )));
            }
            let mut g = self.store.write().await;
            g.insert(key.to_string(), (value.to_string(), ttl_secs));
            Ok(())
        }

        async fn del(&self, key: &str) -> Result<u64, redis::RedisError> {
            let mut g = self.store.write().await;
            Ok(if g.remove(key).is_some() { 1 } else { 0 })
        }

        async fn ping(&self) -> Result<(), redis::RedisError> {
            Ok(())
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Sample {
        id: i32,
        name: String,
    }

    fn make_service() -> (CacheService, Arc<MockBackend>) {
        let backend = Arc::new(MockBackend::new());
        let svc = CacheService::new(backend.clone(), Duration::from_secs(60));
        (svc, backend)
    }

    #[tokio::test]
    async fn cache_miss_returns_none_and_increments_miss() {
        let (svc, _backend) = make_service();
        let key = CacheService::build_key(1, "user", "42");

        let result: Option<Sample> = svc.get_json(&key).await;
        assert!(result.is_none(), "首次读取应当 miss");

        let (hits, misses, _err, _fb) = svc.stats().snapshot();
        assert_eq!(hits, 0);
        assert_eq!(misses, 1, "misses 应当 +1");
    }

    #[tokio::test]
    async fn cache_hit_returns_value_and_increments_hit() {
        let (svc, _backend) = make_service();
        let key = CacheService::build_key(1, "product", "P-001");
        let payload = Sample {
            id: 7,
            name: "测试面料".to_string(),
        };

        svc.set_json(&key, &payload, None).await;
        let got: Option<Sample> = svc.get_json(&key).await;

        assert_eq!(got, Some(payload), "应当命中缓存");

        let (hits, misses, _err, _fb) = svc.stats().snapshot();
        assert_eq!(hits, 1, "hits 应当 +1");
        assert_eq!(misses, 0);
    }

    #[tokio::test]
    async fn cache_invalidate_removes_entry() {
        let (svc, _backend) = make_service();
        let key = CacheService::build_key(2, "user", "99");
        let payload = Sample {
            id: 99,
            name: "待删除".to_string(),
        };

        svc.set_json(&key, &payload, None).await;
        // 命中校验
        let before: Option<Sample> = svc.get_json(&key).await;
        assert!(before.is_some());

        // 失效
        svc.invalidate(&key).await;
        // 失效后应 miss
        let after: Option<Sample> = svc.get_json(&key).await;
        assert!(after.is_none(), "失效后应当 miss");

        let (hits, misses, _err, _fb) = svc.stats().snapshot();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1, "应当一次命中 + 一次失效后未命中");
    }

    #[tokio::test]
    async fn redis_failure_falls_back_to_none() {
        // 模拟 Redis 故障：所有 get 应 fallback 到 None，业务继续可用
        let backend = Arc::new(MockBackend::new());
        backend.set_fail(true).await;
        let svc = CacheService::new(backend.clone(), Duration::from_secs(60));
        let key = CacheService::build_key(3, "user", "1");

        let result: Option<Sample> = svc.get_json(&key).await;
        assert!(result.is_none(), "故障时必须 fallback 到 None");

        let (hits, misses, _err, fb) = svc.stats().snapshot();
        assert_eq!(hits, 0);
        assert_eq!(misses, 0);
        assert_eq!(fb, 1, "fallback 次数必须 +1");
    }

    #[tokio::test]
    async fn disabled_service_never_touches_backend() {
        // 验证禁用模式下不访问后端（用于 REDIS_URL 未配置场景）
        let svc = CacheService::disabled();
        let key = CacheService::build_key(0, "x", "y");

        let v: Option<Sample> = svc.get_json(&key).await;
        assert!(v.is_none());

        // 写入也无副作用
        svc.set_json(
            &key,
            &Sample {
                id: 1,
                name: "x".to_string(),
            },
            None,
        )
        .await;

        let (hits, misses, err, fb) = svc.stats().snapshot();
        assert_eq!((hits, misses, err, fb), (0, 0, 0, 1));
    }
}
