//! 排污许可证管理域路由（path 前缀 /pollution-permits）

use crate::container::AppState;
use crate::handlers::pollution_permit_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 排污许可证管理路由（path 前缀 /pollution-permits）
pub fn pollution_permits() -> Router<AppState> {
    Router::new()
        .route(
            "/pollution-permits",
            post(pollution_permit_handler::create),
        )
        .route(
            "/pollution-permits",
            get(pollution_permit_handler::list),
        )
        // 静态路径必须在 :id 参数路由之前注册，避免 axum matchit 把静态段当 :id 匹配
        .route(
            "/pollution-permits/expiry-warnings",
            get(pollution_permit_handler::scan_expiry_warnings),
        )
        // 参数路由放在静态路径之后
        .route(
            "/pollution-permits/:id",
            get(pollution_permit_handler::get_by_id),
        )
        .route(
            "/pollution-permits/:id/revoke",
            post(pollution_permit_handler::revoke),
        )
}

/// 排污许可证管理域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(pollution_permits())
}
