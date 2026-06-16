//! 主备隔离路由注册
//!
//! 注册 3 个端点：
//! - `GET /api/v1/erp/admin/failover/status`
//! - `GET /api/v1/erp/admin/failover/metrics`
//! - `POST /api/v1/erp/admin/failover/test/switch`
//! - `GET /api/v1/erp/admin/failover/health`

use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::failover_handler::{
    get_failover_metrics, get_failover_status, health_check, post_test_switch,
};
use crate::utils::app_state::AppState;

/// 主备隔离路由
pub fn failover_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/erp/admin/failover/status", get(get_failover_status))
        .route(
            "/api/v1/erp/admin/failover/metrics",
            get(get_failover_metrics),
        )
        .route(
            "/api/v1/erp/admin/failover/test/switch",
            post(post_test_switch),
        )
        .route(
            "/api/v1/erp/admin/failover/health",
            get(health_check),
        )
}
