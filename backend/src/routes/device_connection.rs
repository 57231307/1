//! 设备连接管理路由（V15 P2 B05-P2-7）
//!
//! 7 端点：
//! - POST   /register                  设备注册（首次或重新上线）
//! - POST   /:device_id/heartbeat      心跳上报
//! - POST   /:device_id/disconnect     主动下线
//! - GET    /                          设备列表（分页 + 过滤）
//! - GET    /:device_id                设备详情
//! - GET    /online/count              在线设备数
//! - POST   /cleanup-timeout           手动触发超时清理
//!
//! 路由注册顺序：静态路径（/register、/online/count、/cleanup-timeout）必须在 /:id 之前，
//! 避免 axum matchit 把静态段当 :device_id 匹配。

use axum::{
    routing::{get, post},
    Router,
};

use crate::container::AppState;
use crate::handlers::device_connection_handler;

/// 设备连接管理路由（nest 到 /api/v1/erp/device-connections）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 静态路径必须在 /:device_id 之前注册，避免 axum matchit 把静态段当 :device_id 匹配
        .route("/register", post(device_connection_handler::register_device))
        .route("/online/count", get(device_connection_handler::count_online))
        .route(
            "/cleanup-timeout",
            post(device_connection_handler::cleanup_timeout),
        )
        .route("/", get(device_connection_handler::list_devices))
        .route(
            "/:device_id",
            get(device_connection_handler::get_device),
        )
        .route(
            "/:device_id/heartbeat",
            post(device_connection_handler::heartbeat),
        )
        .route(
            "/:device_id/disconnect",
            post(device_connection_handler::disconnect),
        )
}
