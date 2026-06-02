#![allow(dead_code)]

use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
use crate::models::role_permission;
use crate::utils::admin_checker;
use crate::utils::app_state::AppState;
use crate::utils::path_utils::is_module_prefix;
use crate::utils::request_ext::PublicPathCache;
use crate::utils::response::{forbidden_response, unauthorized_response};
use axum::{
    body::Body,
    extract::State,
    http::{Method, Request},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;
use tracing::warn;

pub async fn permission_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method();
    let uri = request.uri();
    let path = uri.path();

    // 使用缓存的公共路径检查结果，避免重复计算
    let is_public = request
        .extensions()
        .get::<PublicPathCache>()
        .map(|cache| cache.is_public)
        .unwrap_or_else(|| is_public_path(path));

    if is_public {
        return Ok(next.run(request).await);
    }

    let auth = request.extensions().get::<AuthContext>().cloned();
    let auth = match auth {
        Some(auth) => auth,
        None => {
            warn!("缺少认证上下文");
            return Err(unauthorized_response("缺少认证上下文"));
        }
    };

    // 权限检查日志（仅记录事件，不包含敏感详细信息）
    tracing::debug!("权限检查: user_id={}, path={}", auth.user_id, path);

    let role_id = match auth.role_id {
        Some(id) => id,
        None => {
            warn!("用户没有关联角色，拒绝访问");
            return Err(forbidden_response("没有关联角色，无法访问"));
        }
    };

    let (resource_type, resource_id) = extract_resource_info(path);

    let has_permission = check_permission(
        &state.db,
        role_id,
        &resource_type,
        resource_id,
        &method_to_action(method),
    )
    .await;

    tracing::debug!("权限检查结果: path={}, has_perm={}", path, has_permission);

    if has_permission {
        Ok(next.run(request).await)
    } else {
        warn!("权限不足: path={} {}", method, path);
        Err(forbidden_response("权限不足，无法访问该资源"))
    }
}

fn extract_resource_info(path: &str) -> (String, Option<i32>) {
    // 解析API路径，提取资源类型和ID
    let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

    if path_parts.len() >= 4
        && path_parts[0] == "api"
        && path_parts[1] == "v1"
        && path_parts[2] == "erp"
    {
        // 处理嵌套路径，如 /api/v1/erp/sales/orders/:id/approve
        // 资源类型由第4段决定，如果第4段是资源类型（如users, products），直接使用
        // 如果第4段是模块名（如sales, purchases），则使用第5段作为资源类型
        let resource_type = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
            path_parts[4].to_string()
        } else {
            path_parts[3].to_string()
        };

        // 尝试提取资源ID（跳过模块前缀）
        let start_idx = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
            5
        } else {
            4
        };
        for part in path_parts.iter().skip(start_idx) {
            if let Ok(id) = part.parse::<i32>() {
                return (resource_type, Some(id));
            }
        }

        (resource_type, None)
    } else {
        ("unknown".to_string(), None)
    }
}

fn method_to_action(method: &Method) -> String {
    match *method {
        Method::GET => "read",
        Method::POST => "create",
        Method::PUT => "update",
        Method::PATCH => "update",
        Method::DELETE => "delete",
        _ => "read",
    }
    .to_string()
}

use dashmap::DashMap;
use once_cell::sync::Lazy;

/// 缓存项，包含数据和过期时间
#[derive(Clone)]
struct CacheEntry<T: Clone> {
    data: T,
    expires_at: DateTime<Utc>,
}

impl<T: Clone> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Utc::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

// Cache: role_id -> CacheEntry<Arc<Vec<role_permission::Model>>>
// 使用 Arc 包装，克隆时只增加引用计数，不复制数据
static PERMISSION_CACHE: Lazy<DashMap<i32, CacheEntry<Arc<Vec<role_permission::Model>>>>> =
    Lazy::new(DashMap::new);

/// 权限缓存TTL（5分钟）
const PERMISSION_CACHE_TTL: i64 = 5;

pub fn clear_permission_cache(role_id: Option<i32>) {
    if let Some(id) = role_id {
        PERMISSION_CACHE.remove(&id);
    } else {
        PERMISSION_CACHE.clear();
    }
}

/// 清理所有过期的权限缓存条目
/// 建议定期调用此函数以避免内存泄漏
pub fn cleanup_expired_permission_cache() {
    PERMISSION_CACHE.retain(|_, entry| !entry.is_expired());
    // 同时清理管理员角色缓存
    admin_checker::cleanup_expired_admin_cache();
}

async fn check_permission(
    db: &sea_orm::DatabaseConnection,
    role_id: i32,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> bool {
    // 检查是否是管理员角色（带缓存）
    if admin_checker::is_admin_role(db, role_id).await {
        return true;
    }

    // 尝试从缓存加载，检查是否过期
    let permissions = if let Some(cached) = PERMISSION_CACHE.get(&role_id) {
        if cached.is_expired() {
            // 缓存已过期，移除并重新加载
            PERMISSION_CACHE.remove(&role_id);
            None
        } else {
            Some(cached.data.clone())
        }
    } else {
        None
    };

    let permissions = match permissions {
        Some(perms) => perms,
        None => {
            // 从数据库加载
            let db_perms = role_permission::Entity::find()
                .filter(
                    <role_permission::Entity as sea_orm::EntityTrait>::Column::RoleId.eq(role_id),
                )
                .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::Allowed.eq(true))
                .all(db)
                .await
                .unwrap_or_default();

            // 使用 Arc 包装，插入缓存，设置TTL
            let arc_perms = Arc::new(db_perms);
            let ttl = Duration::minutes(PERMISSION_CACHE_TTL);
            PERMISSION_CACHE.insert(role_id, CacheEntry::new(arc_perms.clone(), ttl));
            arc_perms
        }
    };

    // 检查是否有匹配的权限
    permissions.iter().any(|p| {
        p.resource_type == resource_type
            && p.action == action
            && (p.resource_id == resource_id || p.resource_id.is_none())
    })
}
