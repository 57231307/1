//! Incoterms 贸易术语域路由

use crate::container::AppState;
use crate::handlers::incoterms_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// Incoterms 路由（path 前缀 /incoterms）
pub fn incoterms() -> Router<AppState> {
    Router::new()
        .route(
            "/incoterms/quotations/:quotation_id/price-composition",
            get(incoterms_handler::get_price_composition),
        )
        .route(
            "/incoterms/cost-calculation",
            post(incoterms_handler::calculate_costs),
        )
        .route(
            "/incoterms/usage-report",
            get(incoterms_handler::monthly_usage_report),
        )
}

/// Incoterms 域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(incoterms())
}
