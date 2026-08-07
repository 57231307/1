//! 劳动合同域路由

use crate::container::AppState;
use crate::handlers::labor_contract_handler;
use axum::{
    routing::{get, post, put},
    Router,
};

/// 劳动合同路由（path 前缀 /labor-contracts）
pub fn labor_contracts() -> Router<AppState> {
    Router::new()
        .route("/labor-contracts", post(labor_contract_handler::create))
        .route("/labor-contracts", get(labor_contract_handler::list))
        .route(
            "/labor-contracts/scan-expiry-warnings",
            post(labor_contract_handler::scan_expiry_warnings),
        )
        .route(
            "/labor-contracts/active-by-worker/:worker_id",
            get(labor_contract_handler::get_active_by_worker),
        )
        .route(
            "/labor-contracts/:id",
            get(labor_contract_handler::get_by_id),
        )
        .route("/labor-contracts/:id", put(labor_contract_handler::update))
        .route(
            "/labor-contracts/:id/terminate",
            post(labor_contract_handler::terminate),
        )
        // 打印路由
        .route(
            "/labor-contracts/:id/print",
            get(print_handler::labor_contract_print_docx),
        )
}

/// 劳动合同域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(labor_contracts())
}
