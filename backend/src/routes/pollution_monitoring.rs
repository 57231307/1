//! 环境监测与固废处置域路由（path 前缀 /pollution-monitoring）

use crate::container::AppState;
use crate::handlers::pollution_monitoring_handler;
use axum::{
    routing::{get, post, put},
    Router,
};

/// 环境监测与固废处置路由（path 前缀 /pollution-monitoring）
pub fn pollution_monitoring() -> Router<AppState> {
    Router::new()
        .route(
            "/pollution-monitoring/records",
            post(pollution_monitoring_handler::create_monitoring_record),
        )
        .route(
            "/pollution-monitoring/records",
            get(pollution_monitoring_handler::list_monitoring_records),
        )
        .route(
            "/pollution-monitoring/solid-waste-disposals",
            post(pollution_monitoring_handler::create_solid_waste_disposal),
        )
        .route(
            "/pollution-monitoring/solid-waste-disposals/:id/status",
            put(pollution_monitoring_handler::update_waste_status),
        )
        .route(
            "/pollution-monitoring/exceedance-alerts",
            get(pollution_monitoring_handler::scan_exceedance_alerts),
        )
        // 打印路由
        .route(
            "/pollution-monitoring/solid-waste-disposals/:id/print",
            get(crate::handlers::print_handler::solid_waste_disposal_print_docx),
        )
}

/// 环境监测与固废处置域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(pollution_monitoring())
}
