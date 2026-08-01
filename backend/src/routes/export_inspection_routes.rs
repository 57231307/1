//! 出口商检路由
//! V15 P2 B08-12

use axum::{routing::get, Router};
use crate::container::AppState;
use crate::handlers::{export_inspection_handler, certificate_of_origin_handler};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(export_inspection_handler::list_inspections))
        .route("/:id", get(export_inspection_handler::get_inspection))
        .route("/:id/certificates", get(certificate_of_origin_handler::list_certificates))
        .route("/certificates/:id", get(certificate_of_origin_handler::get_certificate))
}