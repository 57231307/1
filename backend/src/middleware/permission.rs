use crate::middleware::public_routes::is_public_path;
use crate::middleware::auth_context::AuthContext;
use crate::models::role_permission;
use crate::utils::app_state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, Method},
    middleware::Next,
    response::Response,
};
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

    // DEBUG: 输出用户信息
    tracing::error!("DEBUG_PERMISSION: user_id={}, username={}, role_id={:?}", auth.user_id, auth.username, auth.role_id);

    // TEMPORARY: 临时允许所有请求以调试问题
    // TODO: 修复后移除这行
    return Ok(next.run(request).await);

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
        let resource_type = path_parts[3].to_string();
        
        // 尝试提取资源ID
        if path_parts.len() >= 5 {
            if let Ok(id) = path_parts[4].parse::<i32>() {
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
        Method::DELETE => "delete",
        _ => "read",
    }
    .to_string()
}

use dashmap::DashMap;
use once_cell::sync::Lazy;

// Cache: role_id -> Vec<role_permission::Model>
static PERMISSION_CACHE: Lazy<DashMap<i32, Vec<role_permission::Model>>> =
    Lazy::new(|| DashMap::new());

pub fn clear_permission_cache(role_id: Option<i32>) {
    if let Some(id) = role_id {
        PERMISSION_CACHE.remove(&id);
    } else {
        PERMISSION_CACHE.clear();
    }
}

async fn check_permission(
    db: &sea_orm::DatabaseConnection,
    role_id: i32,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> bool {
    // Admin role bypasses all permission checks
    if role_id == 1 {
        return true;
    }

    // Attempt to load from cache
    let permissions = if let Some(cached) = PERMISSION_CACHE.get(&role_id) {
        cached.clone()
    } else {
        // Load from DB
        let db_perms = role_permission::Entity::find()
            .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::RoleId.eq(role_id))
            .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::Allowed.eq(true))
            .all(db)
            .await
            .unwrap_or_default();
            
        PERMISSION_CACHE.insert(role_id, db_perms.clone());
        db_perms
    };

    // Check if any permission matches
    permissions.into_iter().any(|p| {
        p.resource_type == resource_type &&
        p.action == action &&
        (p.resource_id == resource_id || p.resource_id.is_none())
    })
}
