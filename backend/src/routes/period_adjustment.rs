//! 期末调整路由（V15 P2 B05-P2-10）
//!
//! 6 端点：
//! - POST   /                  创建期末调整（暂估/摊销/预提）
//! - POST   /:id/confirm       确认（生成调整凭证）
//! - POST   /:id/reverse       红字冲销（生成红字凭证）
//! - POST   /:id/cancel        取消（draft → cancelled）
//! - GET    /:id               详情
//! - GET    /                  列表（按类型/期间/状态过滤分页）

use axum::{
    routing::{get, post},
    Router,
};

use crate::container::AppState;
use crate::handlers::period_adjustment_handler;

/// 期末调整路由（nest 到 /api/v1/erp/period-adjustments）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(period_adjustment_handler::create_adjustment))
        .route("/", get(period_adjustment_handler::list_adjustments))
        .route(
            "/:id/confirm",
            post(period_adjustment_handler::confirm_adjustment),
        )
        .route(
            "/:id/reverse",
            post(period_adjustment_handler::reverse_adjustment),
        )
        .route(
            "/:id/cancel",
            post(period_adjustment_handler::cancel_adjustment),
        )
        .route("/:id", get(period_adjustment_handler::get_adjustment))
}
