//! 财务预警路由（V15 P0-B04 Batch 481）
//!
//! 6 端点：
//!   - POST   /trigger-scan            触发扫描生成预警
//!   - POST   /                         手动创建预警
//!   - GET    /                         预警列表
//!   - GET    /:id                      预警详情
//!   - POST   /:id/acknowledge         确认预警（active → acknowledged）
//!   - POST   /:id/resolve             解决预警（acknowledged → resolved）
//!
//! 路由注册顺序：静态路径（/ 和 /trigger-scan）必须在 /:id 之前，
//! 避免 axum matchit 把 "trigger-scan" 当 :id 匹配。

use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::finance_alert_handler;
use crate::utils::app_state::AppState;

/// 财务预警路由（nest 到 /api/v1/erp/finance-alerts）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 静态路径必须在 /:id 之前注册，避免 axum matchit 把 "trigger-scan" 当 :id 匹配
        .route(
            "/trigger-scan",
            post(finance_alert_handler::trigger_scan),
        )
        .route(
            "/",
            get(finance_alert_handler::list_alerts).post(finance_alert_handler::create_alert),
        )
        .route("/:id", get(finance_alert_handler::get_alert))
        .route(
            "/:id/acknowledge",
            post(finance_alert_handler::acknowledge),
        )
        .route("/:id/resolve", post(finance_alert_handler::resolve))
}
