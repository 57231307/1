import re

# 1. Update backend/src/handlers/auth_handler.rs
with open("/home/root0/桌面/121/1/backend/src/handlers/auth_handler.rs", "r") as f:
    content = f.read()

# Add UserPermissionDto
dto = """
#[derive(Debug, Serialize, ToSchema)]
pub struct UserPermissionDto {
    pub resource: String,
    pub action: String,
    pub resource_id: Option<i32>,
}
"""
if "pub struct UserPermissionDto" not in content:
    content = content.replace("pub struct LoginResponse {", dto + "\n#[derive(Debug, Serialize, ToSchema)]\npub struct LoginResponse {")

# Add permissions field
if "pub permissions: Vec<UserPermissionDto>," not in content:
    content = content.replace("    pub user: UserInfo,\n}", "    pub user: UserInfo,\n    pub permissions: Vec<UserPermissionDto>,\n}")

# Fix login response creation
# find `Ok(Json(ApiResponse::success_with_message(` and replace
login_resp_creation = """
    // Fetch permissions
    let mut permissions = vec![];
    if let Some(role_id) = user.role_id {
        let role_perms = crate::models::role_permission::Entity::find()
            .filter(crate::models::role_permission::Column::RoleId.eq(role_id))
            .filter(crate::models::role_permission::Column::Allowed.eq(true))
            .all(state.db.as_ref())
            .await
            .unwrap_or_default();
            
        permissions = role_perms.into_iter().map(|p| UserPermissionDto {
            resource: p.resource_type,
            action: p.action,
            resource_id: p.resource_id,
        }).collect();
    }

    Ok(Json(ApiResponse::success_with_message(
        LoginResponse {
            token,
            user: UserInfo {
                id: user.id,
                username: user.username,
                email: user.email,
                role_id: user.role_id,
            },
            permissions,
        },"""

if "let mut permissions = vec![];" not in content:
    content = content.replace("""    Ok(Json(ApiResponse::success_with_message(
        LoginResponse {
            token,
            user: UserInfo {
                id: user.id,
                username: user.username,
                email: user.email,
                role_id: user.role_id,
            },
        },""", login_resp_creation)
        
# also need to import `sea_orm::*` in auth_handler.rs
if "use sea_orm::" not in content:
    content = content.replace("use axum::{", "use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};\nuse axum::{")

with open("/home/root0/桌面/121/1/backend/src/handlers/auth_handler.rs", "w") as f:
    f.write(content)

