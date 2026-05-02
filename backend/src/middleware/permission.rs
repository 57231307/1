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

    let public_paths = [
        "/health",
        "/ready",
        "/live",
        "/init",
        "/api/v1/erp/health",
        "/api/v1/erp/ready",
        "/api/v1/erp/live",
        "/api/v1/erp/init",
        "/api/v1/erp/auth/login",
        "/api/v1/erp/auth/refresh",
        "/api/v1/erp/auth/logout",
        "/api/v1/erp/dashboard",
    ];

    if public_paths.iter().any(|p| path.starts_with(p)) {
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

    let (resource_type, resource_id) = extract_resource_info(path);

    let has_permission = check_permission(
        &state.db,
        auth.role_id.unwrap_or(0),
        &resource_type,
        resource_id,
        &method_to_action(method),
    )
    .await;

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
    match method {
        &Method::GET => "read",
        &Method::POST => "create",
        &Method::PUT => "update",
        &Method::DELETE => "delete",
        _ => "read",
    }
    .to_string()
}

async fn check_permission(
    db: &sea_orm::DatabaseConnection,
    role_id: i32,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> bool {
    let permission = role_permission::Entity::find()
        .filter(
            <role_permission::Entity as sea_orm::EntityTrait>::Column::RoleId
                .is_in([role_id])
        )
        .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::ResourceType.eq(resource_type))
        .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::Action.eq(action))
        .filter(
            <role_permission::Entity as sea_orm::EntityTrait>::Column::ResourceId
                .eq(resource_id)
                .or(<role_permission::Entity as sea_orm::EntityTrait>::Column::ResourceId.is_null()),
        )
        .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::Allowed.eq(true))
        .one(db)
        .await
        .unwrap_or(None);

    permission.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resource_info_valid_with_id() {
        let (resource_type, resource_id) = extract_resource_info("/api/v1/erp/users/123");
        assert_eq!(resource_type, "users");
        assert_eq!(resource_id, Some(123));
    }

    #[test]
    fn test_extract_resource_info_valid_without_id() {
        let (resource_type, resource_id) = extract_resource_info("/api/v1/erp/users");
        assert_eq!(resource_type, "users");
        assert_eq!(resource_id, None);
    }

    #[test]
    fn test_extract_resource_info_non_numeric_id() {
        let (resource_type, resource_id) = extract_resource_info("/api/v1/erp/users/abc");
        assert_eq!(resource_type, "users");
        assert_eq!(resource_id, None);
    }

    #[test]
    fn test_extract_resource_info_unknown_path() {
        let (resource_type, resource_id) = extract_resource_info("/v1/erp/users/1");
        assert_eq!(resource_type, "unknown");
        assert_eq!(resource_id, None);
    }

    #[test]
    fn test_method_to_action_mapping() {
        assert_eq!(method_to_action(&Method::GET), "read");
        assert_eq!(method_to_action(&Method::POST), "create");
        assert_eq!(method_to_action(&Method::PUT), "update");
        assert_eq!(method_to_action(&Method::DELETE), "delete");
        assert_eq!(method_to_action(&Method::PATCH), "read");
    }
}
