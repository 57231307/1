//! 缓存模块（P12 批 1 性能优化）
//!
//! 包含 Redis 客户端封装、业务级缓存门面、统计与 graceful degradation 支持。
//! 业务接入示例见 `redis_client::CacheService::build_key`。

pub mod redis_client;

pub use redis_client::{CacheBackend, CacheService, CacheStats, RedisBackend, DEFAULT_TTL_SECS};
