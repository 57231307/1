//! 主备隔离路由注册
//!
//! 本文件只写**相对路径**，前缀由 `routes/mod.rs` 一次性组合：
//! `.nest("/api/v1/erp/admin/failover", failover::failover_routes())`。
//! 注册 5 个端点：
//! - `GET  /status`        → `/api/v1/erp/admin/failover/status`
//! - `GET  /metrics`       → `/api/v1/erp/admin/failover/metrics`
//! - `POST /test/switch`   → `/api/v1/erp/admin/failover/test/switch`
//! - `GET  /health`        → `/api/v1/erp/admin/failover/health`
//! - `POST /failback`      → `/api/v1/erp/admin/failover/failback`

use axum::{
    routing::{get, post},
    Router,
};

use crate::container::AppState;
use crate::handlers::failover_handler::{
    get_failover_metrics, get_failover_status, health_check, post_failback, post_test_switch,
};

/// 主备隔离路由（相对路径；最终 URL 前缀见模块注释与 `route-snapshot.txt` 基线）
pub fn failover_routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_failover_status))
        .route("/metrics", get(get_failover_metrics))
        .route("/test/switch", post(post_test_switch))
        .route("/health", get(health_check))
        // V15 P2 20.4-D：故障回切端点（人工确认后切换回主库）
        .route("/failback", post(post_failback))
}
