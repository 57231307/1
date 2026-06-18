//! 缓存模块（P12 批 1 性能优化）
//!
//! 包含 Redis 客户端封装、业务级缓存门面、统计与 graceful degradation 支持。
//! 业务接入示例见 `redis_client::CacheService::build_key`。

pub mod redis_client;

// P12 批 1：re-export 全部 cache API（CacheBackend / CacheService / CacheStats /
// RedisBackend / DEFAULT_TTL_SECS），便于外部通过 `use crate::cache::xxx` 引用。
// 当前 P12 阶段仅 CacheService 接入业务（user_service / product_service），
// 其他项目在 server bin crate 编译时触发 unused_imports，按 cache 模块 TODO
// 计划在后续业务模块分阶段接入并移除本标注。
#[allow(unused_imports)]
pub use redis_client::{CacheBackend, CacheService, CacheStats, RedisBackend, DEFAULT_TTL_SECS};
