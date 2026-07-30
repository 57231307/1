//! 权限委托域路由（path 前缀 /permission-delegations）

use crate::container::AppState;
use crate::handlers::permission_delegation_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 权限委托路由（path 前缀 /permission-delegations）
pub fn permission_delegations() -> Router<AppState> {
    Router::new()
        // 静态路径必须在 :delegation_id 参数路由之前注册，避免 axum matchit 把静态段当 :delegation_id 匹配
        .route(
            "/permission-delegations",
            post(permission_delegation_handler::create_delegation),
        )
        .route(
            "/permission-delegations",
            get(permission_delegation_handler::list_delegations),
        )
        .route(
            "/permission-delegations/active/:delegatee_id",
            get(permission_delegation_handler::get_active_delegated_permissions),
        )
        .route(
            "/permission-delegations/check",
            get(permission_delegation_handler::has_delegated_permission),
        )
        .route(
            "/permission-delegations/expire-overdue",
            post(permission_delegation_handler::expire_overdue_delegations),
        )
        // 参数路由（delegation_id 为 i64）放在静态路径之后
        .route(
            "/permission-delegations/:delegation_id",
            get(permission_delegation_handler::get_delegation),
        )
        .route(
            "/permission-delegations/:delegation_id/revoke",
            post(permission_delegation_handler::revoke_delegation),
        )
}

/// 权限委托域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(permission_delegations())
}
