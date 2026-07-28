//! V15 批次 07 P1-7 修复：CacheBackend trait + Mock 实现
//!
//! 抽取缓存后端抽象，使单元测试不依赖真实 Redis/moka。
//!
//! ## 设计
//!
//! - `CacheBackend` trait：定义缓存后端契约（get/set/invalidate/invalidate_prefix）
//! - `MemoryCacheBackend`：基于 `DashMap` 的内存实现，单测/开发环境用
//! - `MockCacheBackend`：测试用，可预设返回值，记录调用次数
//! - 生产环境的 `CacheService`（moka LRU + TTL）保持不变，本 trait 为新增能力
//!
//! ## 使用方式
//!
//! ```rust,ignore
//! use crate::services::cache_backend::{CacheBackend, MockCacheBackend};
//!
//! // 测试中注入 Mock
//! let mut mock = MockCacheBackend::new();
//! mock.set_value("user:1".to_string(), b"alice".to_vec());
//! assert_eq!(mock.get("user:1").await, Some(b"alice".to_vec()));
//! ```

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 缓存后端抽象（生产 CacheService/moka，测试 MockCacheBackend，备用 MemoryCacheBackend/DashMap）
#[async_trait::async_trait]
pub trait CacheBackend: Send + Sync {
    /// 获取缓存值，未命中返回 None
    async fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// 写入缓存
    async fn set(&self, key: String, value: Vec<u8>);

    /// 带自定义 TTL 的写入
    async fn set_with_ttl(&self, key: String, value: Vec<u8>, ttl: std::time::Duration);

    /// 失效指定 key
    async fn invalidate(&self, key: &str);

    /// 按前缀批量失效
    async fn invalidate_prefix(&self, prefix: &str);
}

/// 内存缓存后端（DashMap 实现，开发环境用，不支持 LRU 淘汰与 TTL 过期）
#[derive(Default)]
pub struct MemoryCacheBackend {
    inner: Arc<DashMap<String, Vec<u8>>>,
}

impl MemoryCacheBackend {
    /// 创建空的内存缓存
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl CacheBackend for MemoryCacheBackend {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.inner.get(key).map(|v| v.clone())
    }

    async fn set(&self, key: String, value: Vec<u8>) {
        self.inner.insert(key, value);
    }

    async fn set_with_ttl(&self, key: String, value: Vec<u8>, _ttl: std::time::Duration) {
        // 内存实现忽略 TTL，仅写入数据
        self.inner.insert(key, value);
    }

    async fn invalidate(&self, key: &str) {
        self.inner.remove(key);
    }

    async fn invalidate_prefix(&self, prefix: &str) {
        // 收集匹配的 key 后逐个删除（避免持有锁的同时修改）
        let keys_to_remove: Vec<String> = self
            .inner
            .iter()
            .filter(|k| k.key().starts_with(prefix))
            .map(|k| k.key().clone())
            .collect();
        for key in keys_to_remove {
            self.inner.remove(&key);
        }
    }
}

/// Mock 缓存后端（测试用，支持预设返回值、记录调用次数便于断言）
pub struct MockCacheBackend {
    inner: Arc<DashMap<String, Vec<u8>>>,
    /// get 调用次数
    get_calls: AtomicU64,
    /// set 调用次数
    set_calls: AtomicU64,
    /// invalidate 调用次数
    invalidate_calls: AtomicU64,
    /// invalidate_prefix 调用次数
    invalidate_prefix_calls: AtomicU64,
}

impl MockCacheBackend {
    /// 创建空的 Mock 缓存
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            get_calls: AtomicU64::new(0),
            set_calls: AtomicU64::new(0),
            invalidate_calls: AtomicU64::new(0),
            invalidate_prefix_calls: AtomicU64::new(0),
        }
    }

    /// 预设缓存值（不增加 set 调用计数，仅供测试初始化）
    pub fn set_value(&self, key: String, value: Vec<u8>) {
        self.inner.insert(key, value);
    }

    /// 预设多个缓存值
    pub fn set_values(&self, entries: Vec<(String, Vec<u8>)>) {
        for (k, v) in entries {
            self.inner.insert(k, v);
        }
    }

    /// get 调用次数
    pub fn get_call_count(&self) -> u64 {
        self.get_calls.load(Ordering::SeqCst)
    }

    /// set 调用次数
    pub fn set_call_count(&self) -> u64 {
        self.set_calls.load(Ordering::SeqCst)
    }

    /// invalidate 调用次数
    pub fn invalidate_call_count(&self) -> u64 {
        self.invalidate_calls.load(Ordering::SeqCst)
    }

    /// invalidate_prefix 调用次数
    pub fn invalidate_prefix_call_count(&self) -> u64 {
        self.invalidate_prefix_calls.load(Ordering::SeqCst)
    }

    /// 当前存储的 key 数量
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for MockCacheBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CacheBackend for MockCacheBackend {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.get_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.get(key).map(|v| v.clone())
    }

    async fn set(&self, key: String, value: Vec<u8>) {
        self.set_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.insert(key, value);
    }

    async fn set_with_ttl(&self, key: String, value: Vec<u8>, _ttl: std::time::Duration) {
        self.set_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.insert(key, value);
    }

    async fn invalidate(&self, key: &str) {
        self.invalidate_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.remove(key);
    }

    async fn invalidate_prefix(&self, prefix: &str) {
        self.invalidate_prefix_calls.fetch_add(1, Ordering::SeqCst);
        let keys_to_remove: Vec<String> = self
            .inner
            .iter()
            .filter(|k| k.key().starts_with(prefix))
            .map(|k| k.key().clone())
            .collect();
        for key in keys_to_remove {
            self.inner.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn 测试_memory_backend_set_get() {
        let backend = MemoryCacheBackend::new();
        backend.set("k1".to_string(), b"v1".to_vec()).await;
        assert_eq!(backend.get("k1").await, Some(b"v1".to_vec()));
        assert_eq!(backend.get("missing").await, None);
    }

    #[tokio::test]
    async fn 测试_memory_backend_invalidate() {
        let backend = MemoryCacheBackend::new();
        backend.set("k1".to_string(), b"v1".to_vec()).await;
        backend.invalidate("k1").await;
        assert_eq!(backend.get("k1").await, None);
    }

    #[tokio::test]
    async fn 测试_memory_backend_invalidate_prefix() {
        let backend = MemoryCacheBackend::new();
        backend.set("inventory:1".to_string(), b"v1".to_vec()).await;
        backend.set("inventory:2".to_string(), b"v2".to_vec()).await;
        backend.set("sales:1".to_string(), b"v3".to_vec()).await;

        backend.invalidate_prefix("inventory:").await;
        assert_eq!(backend.get("inventory:1").await, None);
        assert_eq!(backend.get("inventory:2").await, None);
        assert_eq!(backend.get("sales:1").await, Some(b"v3".to_vec()));
    }

    #[tokio::test]
    async fn 测试_mock_backend_预设值不增加计数() {
        let mock = MockCacheBackend::new();
        mock.set_value("k1".to_string(), b"v1".to_vec());
        assert_eq!(mock.set_call_count(), 0, "set_value 不应增加 set 计数");
        assert_eq!(mock.get("k1").await, Some(b"v1".to_vec()));
        assert_eq!(mock.get_call_count(), 1, "get 应增加 get 计数");
    }

    #[tokio::test]
    async fn 测试_mock_backend_set_增加计数() {
        let mock = MockCacheBackend::new();
        mock.set("k1".to_string(), b"v1".to_vec()).await;
        mock.set("k2".to_string(), b"v2".to_vec()).await;
        assert_eq!(mock.set_call_count(), 2, "set 应被计数");
        assert_eq!(mock.len(), 2);
    }

    #[tokio::test]
    async fn 测试_mock_backend_invalidate_计数() {
        let mock = MockCacheBackend::new();
        mock.set_value("k1".to_string(), b"v1".to_vec());
        mock.invalidate("k1").await;
        assert_eq!(mock.invalidate_call_count(), 1);
        assert_eq!(mock.get("k1").await, None);
    }

    #[tokio::test]
    async fn 测试_mock_backend_invalidate_prefix_计数() {
        let mock = MockCacheBackend::new();
        mock.set_value("inv:1".to_string(), b"v1".to_vec());
        mock.set_value("inv:2".to_string(), b"v2".to_vec());
        mock.set_value("sales:1".to_string(), b"v3".to_vec());

        mock.invalidate_prefix("inv:").await;
        assert_eq!(mock.invalidate_prefix_call_count(), 1);
        assert_eq!(mock.get("inv:1").await, None);
        assert_eq!(mock.get("inv:2").await, None);
        assert_eq!(mock.get("sales:1").await, Some(b"v3".to_vec()));
    }

    #[tokio::test]
    async fn 测试_mock_backend_动态注入_trait() {
        // V15 批次 07 P1-7 修复核心目标：通过 trait 对象动态注入，单测不依赖真实 moka
        async fn exercise_backend(backend: &dyn CacheBackend) {
            backend.set("k1".to_string(), b"v1".to_vec()).await;
            let _ = backend.get("k1").await;
            backend.invalidate("k1").await;
        }

        let mock: Box<dyn CacheBackend> = Box::new(MockCacheBackend::new());
        exercise_backend(mock.as_ref()).await;
        // 不 panic 即通过
    }
}
