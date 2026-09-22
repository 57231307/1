//! 合同签名域路由

use crate::container::AppState;
use crate::handlers::contract_signature_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 合同签名路由（挂载于 nest 前缀 /api/v1/erp/contract-signatures 之下，路由为相对路径）
pub fn contract_signatures() -> Router<AppState> {
    Router::new()
        .route("/sign", post(contract_signature_handler::sign_contract))
        .route(
            "/{contract_id}/verify",
            get(contract_signature_handler::verify_signature),
        )
        .route(
            "/{contract_id}/revoke",
            post(contract_signature_handler::revoke_signature),
        )
        .route("/", get(contract_signature_handler::list_signed_contracts))
}

/// 合同签名域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(contract_signatures())
}
