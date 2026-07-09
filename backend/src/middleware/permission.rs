
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
use std::sync::LazyLock;

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
static PERMISSION_CACHE: LazyLock<DashMap<i32, CacheEntry<Arc<Vec<role_permission::Model>>>>> =
    LazyLock::new(DashMap::new);

/// 权限缓存TTL（5分钟）
const PERMISSION_CACHE_TTL: i64 = 5;



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
    // M-6 修复：resource_id 精确匹配，action 支持 "*" 通配符
    permissions.iter().any(|p| {
        matches_permission(p, resource_type, resource_id, action)
    })
}

/// 权限匹配纯函数（v14 P0-4 修复：提取为纯函数便于单元测试）
///
/// 匹配规则：
/// - resource_type 必须完全匹配
/// - action 支持精确匹配或 "*" 通配符
/// - resource_id 精确匹配（None 匹配 None，Some(id) 匹配 Some(id)），防止垂直越权
fn matches_permission(
    p: &role_permission::Model,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> bool {
    p.resource_type == resource_type
        && (p.action == action || p.action == "*")
        && match (p.resource_id, resource_id) {
            (None, None) => true,
            (Some(pid), Some(rid)) => pid == rid,
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用权限模型
    fn make_permission(
        resource_type: &str,
        resource_id: Option<i32>,
        action: &str,
    ) -> role_permission::Model {
        role_permission::Model {
            id: 1,
            role_id: 1,
            resource_type: resource_type.to_string(),
            resource_id,
            action: action.to_string(),
            allowed: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ===== extract_resource_info 测试 =====

    #[test]
    fn test_extract_resource_info_标准路径无ID() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/users");
        assert_eq!(rt, "users");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_标准路径带ID() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/users/123");
        assert_eq!(rt, "users");
        assert_eq!(rid, Some(123));
    }

    #[test]
    fn test_extract_resource_info_模块前缀路径无ID() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders");
        assert_eq!(rt, "orders");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_模块前缀路径带ID() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders/456");
        assert_eq!(rt, "orders");
        assert_eq!(rid, Some(456));
    }

    #[test]
    fn test_extract_resource_info_嵌套路径带ID和动作() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders/123/approve");
        assert_eq!(rt, "orders");
        assert_eq!(rid, Some(123));
    }

    #[test]
    fn test_extract_resource_info_非API路径() {
        let (rt, rid) = extract_resource_info("/health");
        assert_eq!(rt, "unknown");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_短路径() {
        let (rt, rid) = extract_resource_info("/api/v1");
        assert_eq!(rt, "unknown");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_空路径() {
        let (rt, rid) = extract_resource_info("/");
        assert_eq!(rt, "unknown");
        assert_eq!(rid, None);
    }

    // ===== method_to_action 测试 =====

    #[test]
    fn test_method_to_action_GET映射read() {
        assert_eq!(method_to_action(&Method::GET), "read");
    }

    #[test]
    fn test_method_to_action_POST映射create() {
        assert_eq!(method_to_action(&Method::POST), "create");
    }

    #[test]
    fn test_method_to_action_PUT映射update() {
        assert_eq!(method_to_action(&Method::PUT), "update");
    }

    #[test]
    fn test_method_to_action_PATCH映射update() {
        assert_eq!(method_to_action(&Method::PATCH), "update");
    }

    #[test]
    fn test_method_to_action_DELETE映射delete() {
        assert_eq!(method_to_action(&Method::DELETE), "delete");
    }

    #[test]
    fn test_method_to_action_未知方法映射read() {
        // OPTIONS 等未明确映射的方法默认为 read
        assert_eq!(method_to_action(&Method::OPTIONS), "read");
    }

    // ===== CacheEntry 测试 =====

    #[test]
    fn test_cache_entry_新建未过期() {
        let entry = CacheEntry::new(true, Duration::minutes(5));
        assert!(!entry.is_expired());
        assert!(entry.data);
    }

    #[test]
    fn test_cache_entry_已过期() {
        // 构造一个已过期的缓存项（过期时间为当前时间减 1 分钟）
        let entry = CacheEntry {
            data: false,
            expires_at: Utc::now() - Duration::minutes(1),
        };
        assert!(entry.is_expired());
    }

    // ===== matches_permission 测试（安全核心）=====

    #[test]
    fn test_matches_permission_类型不匹配返回false() {
        let p = make_permission("users", None, "read");
        assert!(!matches_permission(&p, "orders", None, "read"));
    }

    #[test]
    fn test_matches_permission_全部匹配无ID() {
        let p = make_permission("users", None, "read");
        assert!(matches_permission(&p, "users", None, "read"));
    }

    #[test]
    fn test_matches_permission_action通配符匹配() {
        let p = make_permission("users", None, "*");
        assert!(matches_permission(&p, "users", None, "read"));
        assert!(matches_permission(&p, "users", None, "create"));
        assert!(matches_permission(&p, "users", None, "delete"));
    }

    #[test]
    fn test_matches_permission_ID精确匹配相等() {
        let p = make_permission("users", Some(100), "read");
        assert!(matches_permission(&p, "users", Some(100), "read"));
    }

    #[test]
    fn test_matches_permission_ID精确匹配不等返回false() {
        // 垂直越权防护：权限 ID=100 不能访问 ID=200
        let p = make_permission("users", Some(100), "read");
        assert!(!matches_permission(&p, "users", Some(200), "read"));
    }

    #[test]
    fn test_matches_permission_权限无ID请求有ID返回false() {
        // M-6 修复点：权限 resource_id=None 不能匹配请求 resource_id=Some
        // 防止拥有全局权限的用户操作特定资源（应通过 action="*" 明确授予）
        let p = make_permission("users", None, "read");
        assert!(!matches_permission(&p, "users", Some(100), "read"));
    }

    #[test]
    fn test_matches_permission_权限有ID请求无ID返回false() {
        let p = make_permission("users", Some(100), "read");
        assert!(!matches_permission(&p, "users", None, "read"));
    }

    #[test]
    fn test_matches_permission_action不匹配且非通配符返回false() {
        let p = make_permission("users", None, "read");
        assert!(!matches_permission(&p, "users", None, "delete"));
    }

    #[test]
    fn test_matches_permission_通配符加ID精确匹配() {
        // action="*" + resource_id 精确匹配的组合
        let p = make_permission("users", Some(100), "*");
        assert!(matches_permission(&p, "users", Some(100), "update"));
        assert!(!matches_permission(&p, "users", Some(200), "update"));
    }
}
