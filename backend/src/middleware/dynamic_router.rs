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

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 端点状态缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    status: String,
    cached_at: Instant,
}

/// 端点状态缓存（线程安全，支持并发访问）
#[derive(Debug, Clone)]
pub struct EndpointCache {
    /// 缓存存储：key = "METHOD:path", value = CacheEntry
    cache: Arc<DashMap<String, CacheEntry>>,
    /// 缓存 TTL
    ttl: Duration,
}

impl EndpointCache {
    /// 创建新的缓存实例
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// 获取缓存的端点状态
    pub fn get(&self, method: &str, path: &str) -> Option<String> {
        let key = format!("{}:{}", method, path);
        if let Some(entry) = self.cache.get(&key) {
            if entry.cached_at.elapsed() < self.ttl {
                return Some(entry.status.clone());
            } else {
                // 缓存过期，移除
                self.cache.remove(&key);
            }
        }
        None
    }

    /// 设置缓存的端点状态
    pub fn set(&self, method: &str, path: &str, status: String) {
        let key = format!("{}:{}", method, path);
        self.cache.insert(
            key,
            CacheEntry {
                status,
                cached_at: Instant::now(),
            },
        );
    }

    /// 失效指定端点的缓存
    pub fn invalidate(&self, method: &str, path: &str) {
        let key = format!("{}:{}", method, path);
        self.cache.remove(&key);
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        self.cache.clear();
    }
}

/// 白名单路径（不经过动态路由检查）
const WHITELIST_PATHS: &[&str] = &[
    "/health",
    "/metrics",
    "/api/v1/erp/auth/login",
    "/api/v1/erp/auth/refresh",
    "/api/v1/erp/auth/logout",
    "/api/v1/erp/init",
];

/// 检查路径是否在白名单中
fn is_whitelist_path(path: &str) -> bool {
    WHITELIST_PATHS.iter().any(|&p| path.starts_with(p))
}

/// 动态路由中间件
///
/// 根据 api_endpoints 表的状态动态放行/拒绝请求。
/// 缓存未命中时放行请求（fail-open），确保可用性。
pub async fn dynamic_router_middleware(
    State(state): State<crate::container::AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let path = request.uri().path().to_string();
    let method = request.method().to_string();

    // 白名单路径直接放行
    if is_whitelist_path(&path) {
        return Ok(next.run(request).await);
    }

    // 从缓存获取端点状态
    let cache = state.endpoint_cache.clone();
    let status = cache.get(&method, &path);

    match status {
        Some(ref s) if s == "active" => {
            // 端点活跃，放行
            Ok(next.run(request).await)
        }
        Some(ref s) if s == "inactive" => {
            // 端点不活跃，返回 503
            tracing::warn!(
                method = %method,
                path = %path,
                "动态路由：端点已停用"
            );
            Err(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::from("Service Unavailable"))
                .unwrap())
        }
        Some(ref s) if s == "deprecated" => {
            // 端点已废弃，放行但添加 Deprecation 头
            let mut response = next.run(request).await;
            response
                .headers_mut()
                .insert("Deprecation", "true".parse().unwrap());
            Ok(response)
        }
        Some(_) => {
            // 未知状态，放行
            Ok(next.run(request).await)
        }
        None => {
            // 缓存未命中，查询数据库
            match query_endpoint_status(&state, &method, &path).await {
                Some(endpoint_status) => {
                    // 更新缓存
                    cache.set(&method, &path, endpoint_status.clone());

                    match endpoint_status.as_str() {
                        "active" => Ok(next.run(request).await),
                        "inactive" => {
                            tracing::warn!(
                                method = %method,
                                path = %path,
                                "动态路由：端点已停用"
                            );
                            Err(Response::builder()
                                .status(StatusCode::SERVICE_UNAVAILABLE)
                                .body(Body::from("Service Unavailable"))
                                .unwrap())
                        }
                        "deprecated" => {
                            let mut response = next.run(request).await;
                            response
                                .headers_mut()
                                .insert("Deprecation", "true".parse().unwrap());
                            Ok(response)
                        }
                        _ => Ok(next.run(request).await),
                    }
                }
                None => {
                    // 数据库查询失败或端点不存在，fail-open 放行
                    tracing::debug!(
                        method = %method,
                        path = %path,
                        "动态路由：端点未找到或查询失败，放行请求"
                    );
                    Ok(next.run(request).await)
                }
            }
        }
    }
}

/// 查询数据库获取端点状态
async fn query_endpoint_status(
    state: &crate::container::AppState,
    method: &str,
    path: &str,
) -> Option<String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use crate::models::api_endpoint::{self, Entity as ApiEndpointEntity};

    let result = ApiEndpointEntity::find()
        .filter(api_endpoint::Column::Method.eq(method))
        .filter(api_endpoint::Column::Path.eq(path))
        .one(&*state.db)
        .await;

    match result {
        Ok(Some(endpoint)) => Some(endpoint.status),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(
                method = %method,
                path = %path,
                error = %e,
                "动态路由：查询端点状态失败"
            );
            None
        }
    }
}
