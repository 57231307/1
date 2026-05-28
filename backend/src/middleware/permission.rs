#![allow(dead_code)]

use crate::middleware::public_routes::is_public_path;
use crate::middleware::auth_context::AuthContext;
use crate::models::role_permission;
use crate::models::role;
use crate::utils::app_state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, Method},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Utc, Duration};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tracing::warn;

pub async fn permission_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method();
    let uri = request.uri();
    let path = uri.path();

    if is_public_path(path) {
        return Ok(next.run(request).await);
    }

    let auth = request.extensions().get::<AuthContext>().cloned();
    let auth = match auth {
        Some(auth) => auth,
        None => {
            warn!("缺少认证上下文");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 权限检查日志（仅记录事件，不包含敏感详细信息）
    tracing::debug!("权限检查: user_id={}, path={}", auth.user_id, path);

    let role_id = match auth.role_id {
        Some(id) => id,
        None => {
            warn!("用户 {} 没有关联角色，拒绝访问", auth.user_id);
            return Err(StatusCode::FORBIDDEN);
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

    tracing::info!("DEBUG_PERM: user={}, role={}, path={}, has_perm={}", auth.user_id, role_id, path, has_permission);

    if has_permission {
        Ok(next.run(request).await)
    } else {
        warn!("用户 {} 没有权限访问 {} {}", auth.user_id, method, path);
        Err(StatusCode::FORBIDDEN)
    }
}

fn extract_resource_info(path: &str) -> (String, Option<i32>) {
    // 解析API路径，提取资源类型和ID
    let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    
    if path_parts.len() >= 4 && path_parts[0] == "api" && path_parts[1] == "v1" && path_parts[2] == "erp" {
        // 处理嵌套路径，如 /api/v1/erp/sales/orders/:id/approve
        // 资源类型由第4段决定，如果第4段是资源类型（如users, products），直接使用
        // 如果第4段是模块名（如sales, purchases），则使用第5段作为资源类型
        let resource_type = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
            path_parts[4].to_string()
        } else {
            path_parts[3].to_string()
        };
        
        // 尝试提取资源ID（跳过模块前缀）
        let start_idx = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) { 5 } else { 4 };
        for i in start_idx..path_parts.len() {
            if let Ok(id) = path_parts[i].parse::<i32>() {
                return (resource_type, Some(id));
            }
        }
        
        (resource_type, None)
    } else {
        ("unknown".to_string(), None)
    }
}

/// 判断是否为模块前缀（如 sales, purchases, finance 等）
fn is_module_prefix(part: &str) -> bool {
    matches!(part,
        "sales" | "purchases" | "finance" | "inventory" | "gl" | "ap" | "ar" |
        "bpm" | "crm" | "ai" | "reports" | "tenants" | "webhooks" | "api-keys" |
        "supplier-evaluation" | "customer-credits" | "financial-analysis" |
        "fund-management" | "quality-inspection" | "cost-collections" |
        "sales-analysis" | "sales-prices" | "purchase-prices" |
        "sales-returns" | "ar-reconciliations" | "exchange-rates"
    )
}

fn method_to_action(method: &Method) -> String {
    match *method {
        Method::GET => "read",
        Method::POST => "create",
        Method::PUT => "update",
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

// Cache: role_id -> CacheEntry<Vec<role_permission::Model>>
static PERMISSION_CACHE: Lazy<DashMap<i32, CacheEntry<Vec<role_permission::Model>>>> =
    Lazy::new(|| DashMap::new());

/// 权限缓存TTL（5分钟）
const PERMISSION_CACHE_TTL: i64 = 5;

pub fn clear_permission_cache(role_id: Option<i32>) {
    if let Some(id) = role_id {
        PERMISSION_CACHE.remove(&id);
    } else {
        PERMISSION_CACHE.clear();
    }
}

/// 检查角色是否是管理员角色
async fn is_admin_role(db: &sea_orm::DatabaseConnection, role_id: i32) -> bool {
    // 从数据库查询角色，检查code是否为"admin"
    match role::Entity::find_by_id(role_id)
        .one(db)
        .await
    {
        Ok(Some(role)) => role.code == "admin",
        Ok(None) => false,
        Err(e) => {
            warn!("查询角色失败: {}", e);
            false
        }
    }
}

async fn check_permission(
    db: &sea_orm::DatabaseConnection,
    role_id: i32,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> bool {
    // 检查是否是管理员角色（从数据库查询，而非硬编码）
    if is_admin_role(db, role_id).await {
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
                .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::RoleId.eq(role_id))
                .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::Allowed.eq(true))
                .all(db)
                .await
                .unwrap_or_default();
            
            // 插入缓存，设置TTL
            let ttl = Duration::minutes(PERMISSION_CACHE_TTL);
            PERMISSION_CACHE.insert(role_id, CacheEntry::new(db_perms.clone(), ttl));
            db_perms
        }
    };

    // 检查是否有匹配的权限
    permissions.into_iter().any(|p| {
        p.resource_type == resource_type &&
        p.action == action &&
        (p.resource_id == resource_id || p.resource_id.is_none())
    })
}
