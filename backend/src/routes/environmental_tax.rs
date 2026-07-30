//! 环保税域路由

use crate::container::AppState;
use crate::handlers::environmental_tax_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 环保税路由（path 前缀 /environmental-tax）
pub fn environmental_tax() -> Router<AppState> {
    Router::new()
        .route(
            "/environmental-tax/discharge-records",
            post(environmental_tax_handler::create_discharge_record),
        )
        .route(
            "/environmental-tax/discharge-records",
            get(environmental_tax_handler::list_discharge_records),
        )
        .route(
            "/environmental-tax/tax-declarations",
            get(environmental_tax_handler::generate_tax_declaration),
        )
}

/// 环保税域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(environmental_tax())
}
