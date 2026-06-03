//! IAM（身份与访问管理）域路由
//!
//! 处理用户、角色、权限、部门等身份与访问管理相关接口。
//! 提供多个子路由供主 mod.rs 在不同路径下 nest。

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    department_handler, field_permission_handler, init_handler, role_handler, user_handler,
};

/// 用户管理路由（nest 到 /api/v1/erp/users）
pub fn users() -> Router {
    Router::new()
        .route("/", get(user_handler::list_users))
        .route("/", post(user_handler::create_user))
        .route("/:id", get(user_handler::get_user))
        .route("/:id", put(user_handler::update_user))
        .route("/:id", delete(user_handler::delete_user))
        .route("/change-password", post(user_handler::change_password))
        .route("/reset-password", post(init_handler::reset_admin_password))
}

/// 角色管理路由（nest 到 /api/v1/erp/roles）
pub fn roles() -> Router {
    Router::new()
        .route("/", get(role_handler::list_roles))
        .route("/", post(role_handler::create_role))
        .route("/:id", get(role_handler::get_role))
        .route("/:id", put(role_handler::update_role))
        .route("/:id", delete(role_handler::delete_role))
        .route("/:id/permissions", get(role_handler::get_role_permissions))
        .route("/:id/permissions", post(role_handler::assign_permission))
        .route("/permissions/:id", delete(role_handler::remove_permission))
        .route("/permissions", get(role_handler::list_permissions))
}

/// 部门管理路由（nest 到 /api/v1/erp/departments）
pub fn departments() -> Router {
    Router::new()
        .route("/", get(department_handler::list))
        .route("/", post(department_handler::create))
        .route("/:id", get(department_handler::get))
        .route("/:id", put(department_handler::update))
        .route("/:id", delete(department_handler::delete))
        .route("/tree", get(department_handler::get_department_tree))
}

/// 权限管理路由（nest 到 /api/v1/erp/permissions）
pub fn permissions() -> Router {
    Router::new().route("/", get(role_handler::list_permissions))
}

/// 字段权限路由（nest 到 /api/v1/erp/permissions/fields）
pub fn field_permissions() -> Router {
    Router::new()
        .route(
            "/",
            get(field_permission_handler::list_field_permissions)
                .post(field_permission_handler::create_field_permission),
        )
        .route(
            "/:id",
            get(field_permission_handler::get_field_permission)
                .put(field_permission_handler::update_field_permission)
                .delete(field_permission_handler::delete_field_permission),
        )
}

/// IAM 域统一入口（合并所有子路由）
pub fn routes() -> Router {
    Router::new()
        .merge(users())
        .merge(roles())
        .merge(departments())
        .merge(permissions())
        .merge(field_permissions())
}
