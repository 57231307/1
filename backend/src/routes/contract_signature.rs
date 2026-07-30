//! 合同签名域路由

use crate::container::AppState;
use crate::handlers::contract_signature_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 合同签名路由（path 前缀 /contract-signatures）
pub fn contract_signatures() -> Router<AppState> {
    Router::new()
        .route(
            "/contract-signatures/sign",
            post(contract_signature_handler::sign_contract),
        )
        .route(
            "/contract-signatures/:contract_id/verify",
            get(contract_signature_handler::verify_signature),
        )
        .route(
            "/contract-signatures/:contract_id/revoke",
            post(contract_signature_handler::revoke_signature),
        )
        .route(
            "/contract-signatures",
            get(contract_signature_handler::list_signed_contracts),
        )
}

/// 合同签名域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(contract_signatures())
}
