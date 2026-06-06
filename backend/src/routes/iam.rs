//! IAM（身份与访问管理）域路由
//!
//! 处理用户、角色、权限、部门等身份与访问管理相关接口。
//! 提供多个子路由供主 mod.rs 在不同路径下 nest。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    department_handler, field_permission_handler, init_handler, role_handler, user_handler,
};

/// 用户管理路由（path 前缀 /users，由 routes() 中 merge 装配到 /api/v1/erp 根下）
pub fn users() -> Router<AppState> {
    Router::new()
        .route("/users", get(user_handler::list_users))
        .route("/users", post(user_handler::create_user))
        .route("/users/:id", get(user_handler::get_user))
        .route("/users/:id", put(user_handler::update_user))
        .route("/users/:id", delete(user_handler::delete_user))
        .route("/users/change-password", post(user_handler::change_password))
        .route("/users/reset-password", post(init_handler::reset_admin_password))
}

/// 角色管理路由（path 前缀 /roles）
pub fn roles() -> Router<AppState> {
    Router::new()
        .route("/roles", get(role_handler::list_roles))
        .route("/roles", post(role_handler::create_role))
        .route("/roles/:id", get(role_handler::get_role))
        .route("/roles/:id", put(role_handler::update_role))
        .route("/roles/:id", delete(role_handler::delete_role))
        .route("/roles/:id/permissions", get(role_handler::get_role_permissions))
        .route("/roles/:id/permissions", post(role_handler::assign_permission))
        .route(
            "/roles/permissions/:id",
            delete(role_handler::remove_permission),
        )
        .route("/roles/permissions", get(role_handler::list_permissions))
}

/// 部门管理路由（path 前缀 /departments）
pub fn departments() -> Router<AppState> {
    Router::new()
        .route("/departments", get(department_handler::list))
        .route("/departments", post(department_handler::create))
        .route("/departments/:id", get(department_handler::get))
        .route("/departments/:id", put(department_handler::update))
        .route("/departments/:id", delete(department_handler::delete))
        .route("/departments/tree", get(department_handler::get_department_tree))
}

/// 权限管理路由（path 前缀 /permissions）
pub fn permissions() -> Router<AppState> {
    Router::new().route("/permissions", get(role_handler::list_permissions))
}

/// 字段权限路由（path 前缀 /field-permissions）
pub fn field_permissions() -> Router<AppState> {
    Router::new()
        .route(
            "/field-permissions",
            get(field_permission_handler::list_field_permissions)
                .post(field_permission_handler::create_field_permission),
        )
        .route(
            "/field-permissions/:id",
            get(field_permission_handler::get_field_permission)
                .put(field_permission_handler::update_field_permission)
                .delete(field_permission_handler::delete_field_permission),
        )
}

/// IAM 域统一入口（合并所有子路由）
///
/// 注意：axum 0.7 的 `Router::merge` 会检查 path+method 重叠并 panic，
/// 因此每个子 router 内部 path 都已加上各自独立前缀（`/users`、`/roles` 等），
/// 这样 merge 之后 path+method 不会重叠，不再触发
/// `Overlapping method route` 错误。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(users())
        .merge(roles())
        .merge(departments())
        .merge(permissions())
        .merge(field_permissions())
}
