//! 物流跟踪域路由（基于 LogisticsService）

use crate::container::AppState;
use crate::handlers::logistics_tracking_handler;
use crate::handlers::print_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 物流跟踪路由（path 前缀 /logistics-tracking）
pub fn logistics_tracking() -> Router<AppState> {
    Router::new()
        .route(
            "/logistics-tracking/waybills/:waybill_id/link-purchase-order",
            post(logistics_tracking_handler::link_purchase_order),
        )
        .route(
            "/logistics-tracking/waybills/:waybill_id/tracking-events",
            post(logistics_tracking_handler::record_tracking_event),
        )
        .route(
            "/logistics-tracking/waybills/:waybill_id/tracking-events",
            get(logistics_tracking_handler::list_tracking_events),
        )
        .route(
            "/logistics-tracking/waybills/:waybill_id/calculate-freight",
            post(logistics_tracking_handler::calculate_freight),
        )
        // 打印路由
        .route(
            "/logistics-tracking/waybills/:id/print",
            get(print_handler::logistics_waybill_print_docx),
        )
}

/// 物流跟踪域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(logistics_tracking())
}
