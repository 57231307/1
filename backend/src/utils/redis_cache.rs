//! Redis 分布式缓存工具（P0-D03/D04 / Batch 488）
//!
//! 提供 Redis 作为分布式缓存后端的薄封装，支持：
//! - JSON 序列化的 get/set/del 操作
//! - 按前缀批量删除
//! - 优雅降级：未配置 `REDIS_URL` 时所有操作返回 None/no-op
//!
//! ## 设计要点
//!
//! - **全局静态懒加载**：通过 `OnceCell` 在首次使用时初始化，避免改动 service 构造函数
//! - **ConnectionManager**：redis 0.27 的连接管理器，自动重连，适合多 tokio task 共享
//! - **双缓存策略**：本模块作为 L2（分布式），配合 `cache_service.rs` 的 moka L1（进程内）
//! - **参考模板**：`services/auth_service.rs` 的 JTI 黑名单 Redis 实现（同模式）
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::utils::redis_cache::redis_cache_get_json;
//!
//! // 读缓存
//! if let Some(cached) = redis_cache_get_json::<UserModel>("user:42").await {
//!     return Ok(cached);
//! }
//! // 缓存未命中 → 查询 DB → 写入缓存
//! let user = UserEntity::find_by_id(42).one(&db).await?;
//! redis_cache_set_json("user:42", &user, 300).await; // 5 分钟 TTL
//! ```

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::sync::OnceCell;

/// 全局 Redis 连接管理器（懒加载）
///
/// 首次调用 `get_redis_conn()` 时初始化：
/// - 读取 `REDIS_URL`（或回退 `JTI_REDIS_URL`）
/// - 创建 `redis::Client` + `ConnectionManager`
/// - 未配置 URL → 返回 None，所有缓存操作降级为 no-op
static REDIS_CONN: OnceCell<Option<Arc<tokio::sync::Mutex<ConnectionManager>>>> =
    OnceCell::const_new();

/// 初始化 Redis 连接
async fn init_redis_conn() -> Option<Arc<tokio::sync::Mutex<ConnectionManager>>> {
    let url = std::env::var("REDIS_URL")
        .or_else(|_| std::env::var("JTI_REDIS_URL"))
        .ok()
        .filter(|s| !s.is_empty());

    let url = match url {
        Some(u) => u,
        None => {
            tracing::debug!(
                "REDIS_URL/JTI_REDIS_URL 未配置，分布式缓存降级为 no-op（多实例部署缓存不共享）"
            );
            return None;
        }
    };

    match redis::Client::open(url.as_str()) {
        Ok(client) => match ConnectionManager::new(client).await {
            Ok(conn) => {
                tracing::info!("分布式缓存已启用 Redis 后端");
                Some(Arc::new(tokio::sync::Mutex::new(conn)))
            }
            Err(e) => {
                tracing::warn!("Redis 连接失败 ({:?})，分布式缓存降级为 no-op", e);
                None
            }
        },
        Err(e) => {
            tracing::warn!("Redis URL 解析失败 ({:?})，分布式缓存降级为 no-op", e);
            None
        }
    }
}

/// 获取 Redis 连接（懒初始化）
async fn get_redis_conn() -> Option<Arc<tokio::sync::Mutex<ConnectionManager>>> {
    REDIS_CONN.get_or_init(init_redis_conn).await.clone()
}

/// 从 Redis 读取 JSON 并反序列化
///
/// 未配置 Redis 或 key 不存在时返回 None（优雅降级）
pub async fn redis_cache_get_json<T: DeserializeOwned>(key: &str) -> Option<T> {
    let conn_arc = get_redis_conn().await?;
    let mut conn = conn_arc.lock().await;

    let result: redis::RedisResult<Option<String>> = conn.get(key).await;
    match result {
        Ok(Some(json_str)) => match serde_json::from_str::<T>(&json_str) {
            Ok(value) => Some(value),
            Err(e) => {
                tracing::warn!(key = %key, error = %e, "Redis 缓存反序列化失败，回退到 DB");
                None
            }
        },
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(key = %key, error = %e, "Redis GET 失败，回退到 DB");
            None
        }
    }
}

/// 写入 JSON 到 Redis（带 TTL）
///
/// 未配置 Redis 时为 no-op
pub async fn redis_cache_set_json<T: Serialize>(key: &str, value: &T, ttl_secs: u64) {
    let conn_arc = match get_redis_conn().await {
        Some(c) => c,
        None => return,
    };

    let json_str = match serde_json::to_string(value) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(key = %key, error = %e, "Redis 缓存序列化失败，跳过写入");
            return;
        }
    };

    let mut conn = conn_arc.lock().await;
    // P0-D03 修复：原 .map_err(...).ok() 类型不匹配（Result → Option），
    // 改用 if let Err 直接处理错误，保持优雅降级语义
    if let Err(e) = conn.set_ex(key, json_str, ttl_secs).await {
        tracing::warn!(key = %key, error = %e, "Redis SET 失败");
    }
}

/// 删除指定 key
///
/// 未配置 Redis 时为 no-op
pub async fn redis_cache_del(key: &str) {
    let conn_arc = match get_redis_conn().await {
        Some(c) => c,
        None => return,
    };

    let mut conn = conn_arc.lock().await;
    // P0-D03 修复：原 .map_err(...).ok() 类型不匹配，改用 if let Err 直接处理
    if let Err(e) = conn.del(key).await {
        tracing::warn!(key = %key, error = %e, "Redis DEL 失败");
    }
}

/// 按前缀批量删除（使用 SCAN 避免阻塞 Redis）
///
/// 未配置 Redis 时为 no-op
///
/// 注意：前缀匹配会扫描 keys，大量 key 时可能影响性能。
/// 建议仅在创建/更新/删除时调用，不要在热点读路径使用。
pub async fn redis_cache_del_prefix(prefix: &str) {
    let conn_arc = match get_redis_conn().await {
        Some(c) => c,
        None => return,
    };

    let mut conn = conn_arc.lock().await;
    // 使用 SCAN 模式遍历匹配的 key（生产环境友好，不阻塞 Redis）
    let pattern = format!("{}*", prefix);
    let mut cursor: u64 = 0;
    loop {
        let (next_cursor, keys): (u64, Vec<String>) = match redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut *conn)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!(prefix = %prefix, error = %e, "Redis SCAN 失败");
                return;
            }
        };

        if !keys.is_empty() {
            // P0-D03 修复：原 .map_err(...).ok() 类型不匹配，改用 if let Err 直接处理
            if let Err(e) = conn.del(&keys).await {
                tracing::warn!(prefix = %prefix, error = %e, "Redis 批量 DEL 失败");
            }
        }

        if next_cursor == 0 {
            break;
        }
        cursor = next_cursor;
    }
}

/// 默认缓存 TTL（5 分钟）
pub const DEFAULT_CACHE_TTL_SECS: u64 = 300;

/// 生成标准化的缓存 key
///
/// 格式：`{service}:{entity}:{id}`
/// 例如：`user:42`、`product:100`、`customer:555`
pub fn cache_key(service: &str, id: impl std::fmt::Display) -> String {
    format!("{}:{}", service, id)
}
