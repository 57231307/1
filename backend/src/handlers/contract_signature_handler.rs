//! 合同签名 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::contract_signature_service::{ContractSignatureService, SignContractRequest};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    Json,
};

/// 签署合同
pub async fn sign_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<SignContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _ = auth.user_id; // 审计用
    let service = ContractSignatureService::new(state.db.clone());
    let model = service.sign_contract(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 验证签名
pub async fn verify_signature(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(contract_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ContractSignatureService::new(state.db.clone());
    let result = service.verify_signature(contract_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// 撤销签名
pub async fn revoke_signature(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(contract_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ContractSignatureService::new(state.db.clone());
    let model = service.revoke_signature(contract_id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 列出已签署合同
pub async fn list_signed_contracts(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ContractSignatureService::new(state.db.clone());
    let list = service.list_signed_contracts().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}
