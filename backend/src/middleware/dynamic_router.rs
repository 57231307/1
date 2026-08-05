//! V15 P2 20.6-A：API 网关动态路由中间件
//!
//! 根据 `api_endpoints` 表的 status 字段动态放行/拒绝请求。
//!
//! ## 工作原理
//!
//! 1. 请求进入时，从 `AppState` 中的 endpoint cache 查询 path+method 对应的端点状态
//! 2. 若端点不存在或 status=inactive，返回 404/503
//! 3. 若端点 active，放行请求
//!
//! ## 设计要点
//!
//! - **缓存**：endpoint 状态缓存在内存中（TTL 60s），避免每次请求查库
//! - **降级**：缓存未命中时放行请求（fail-open），确保可用性
//! - **白名单**：健康检查/指标/文档等系统路径不经过动态路由检查

use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// 端点状态缓存条目
#[derive(Debug, Clone)]
struct EndpointCacheEntry {
    /// 是否 active
    is_active: bool,
    /// 缓存写入时间
    cached_at: Instant,
}

/// 端点状态缓存（path+method → status）
///
/// 使用 RwLock<HashMap> 实现简单的内存缓存，TTL 60s。
/// 生产环境可替换为 Redis 缓存。
#[derive(Clone)]
pub struct EndpointCache {
    cache: Arc<RwLock<HashMap<String, EndpointCacheEntry>>>,
    ttl: Duration,
}

impl EndpointCache {
    /// 创建新的缓存实例
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// 获取端点状态
    ///
    /// 返回 Some(true) 表示 active，Some(false) 表示 inactive，None 表示未注册
    pub fn get(&self, method: &str, path: &str) -> Option<bool> {
        let key = format!("{}:{}", method.to_uppercase(), path);
        let cache = self.cache.read().ok()?;
        let entry = cache.get(&key)?;
        if entry.cached_at.elapsed() > self.ttl {
            return None; // 缓存过期
        }
        Some(entry.is_active)
    }

    /// 更新端点状态
    pub fn set(&self, method: &str, path: &str, is_active: bool) {
        let key = format!("{}:{}", method.to_uppercase(), path);
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(
                key,
                EndpointCacheEntry {
                    is_active,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// 清除过期缓存条目
    pub fn cleanup_expired(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.retain(|_, entry| entry.cached_at.elapsed() <= self.ttl);
        }
    }
}

impl Default for EndpointCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

/// 不经过动态路由检查的系统路径前缀
const SYSTEM_PATH_PREFIXES: &[&str] = &[
    "/health",
    "/metrics",
    "/swagger-ui",
    "/api-docs",
    "/static",
    "/init/",
    "/ws/",
];

/// 动态路由中间件
///
/// 检查请求的 path+method 是否在 api_endpoints 表中注册且 active。
/// 系统路径（健康检查/指标/文档等）不经过检查。
pub async fn dynamic_router_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    let method = request.method().as_str();

    // 系统路径直接放行
    if SYSTEM_PATH_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return Ok(next.run(request).await);
    }

    // 从 extensions 中获取 EndpointCache（由 AppState 注入）
    let cache = request
        .extensions()
        .get::<EndpointCache>()
        .cloned();

    if let Some(cache) = cache {
        match cache.get(method, path) {
            Some(true) => {
                // active，放行
                Ok(next.run(request).await)
            }
            Some(false) => {
                // inactive，返回 503
                let body = serde_json::json!({
                    "code": 503,
                    "message": "该 API 端点已停用",
                    "data": null
                });
                Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap_or_default()))
                    .unwrap_or_else(|_| Response::new(Body::empty())))
            }
            None => {
                // 未注册或缓存未命中，放行（fail-open）
                Ok(next.run(request).await)
            }
        }
    } else {
        // 缓存未注入，放行（降级处理）
        Ok(next.run(request).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_cache_set_get() {
        let cache = EndpointCache::new(Duration::from_secs(60));
        assert!(cache.get("GET", "/api/test").is_none());

        cache.set("GET", "/api/test", true);
        assert_eq!(cache.get("GET", "/api/test"), Some(true));

        cache.set("POST", "/api/test", false);
        assert_eq!(cache.get("POST", "/api/test"), Some(false));
    }

    #[test]
    fn test_endpoint_cache_case_insensitive_method() {
        let cache = EndpointCache::new(Duration::from_secs(60));
        cache.set("GET", "/api/test", true);
        assert_eq!(cache.get("get", "/api/test"), Some(true));
        assert_eq!(cache.get("Get", "/api/test"), Some(true));
    }

    #[test]
    fn test_endpoint_cache_cleanup() {
        let cache = EndpointCache::new(Duration::from_millis(1));
        cache.set("GET", "/api/test", true);
        // 等待缓存过期
        std::thread::sleep(Duration::from_millis(10));
        cache.cleanup_expired();
        assert!(cache.get("GET", "/api/test").is_none());
    }

    #[test]
    fn test_system_path_prefixes() {
        // 确保系统路径前缀列表不为空
        assert!(!SYSTEM_PATH_PREFIXES.is_empty());
        assert!(SYSTEM_PATH_PREFIXES.contains(&"/health"));
        assert!(SYSTEM_PATH_PREFIXES.contains(&"/metrics"));
    }
}
