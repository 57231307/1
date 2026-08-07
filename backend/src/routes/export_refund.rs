//! 出口退税域路由

use crate::container::AppState;
use crate::handlers::export_refund_handler;
use crate::handlers::print_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 出口退税路由（path 前缀 /export-refunds）
pub fn export_refunds() -> Router<AppState> {
    Router::new()
        .route(
            "/export-refunds/customs-declarations",
            post(export_refund_handler::create_customs_declaration),
        )
        .route(
            "/export-refunds/sales-orders/:sales_order_id/documents-verification",
            get(export_refund_handler::verify_documents_completeness),
        )
        .route(
            "/export-refunds/refund-calculation",
            post(export_refund_handler::calculate_refund),
        )
        .route(
            "/export-refunds/refund-declarations",
            post(export_refund_handler::generate_refund_declaration),
        )
        .route(
            "/export-refunds/refund-declarations",
            get(export_refund_handler::list_refund_declarations),
        )
        // 打印路由
        .route(
            "/export-refunds/:id/print",
            get(print_handler::export_refund_declaration_print_docx),
        )
}

/// 出口退税域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(export_refunds())
}
