use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 核销查询参数
#[derive(Debug, Deserialize)]
pub struct ArVerificationQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub invoice_id: Option<i32>,
    pub payment_id: Option<i32>,
    pub status: Option<String>,
}

/// 手动核销请求
#[derive(Debug, Deserialize)]
pub struct ManualVerifyRequest {
    pub invoice_id: i32,
    pub payment_id: i32,
    pub amount: rust_decimal::Decimal,
    pub remark: Option<String>,
}

/// 获取核销列表
/// GET /api/v1/erp/ar/verifications
pub async fn list_verifications(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArVerificationQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let page = query.page.unwrap_or(1);
    // v12 批次 39 修复：page_size clamp(1,100) 防 DoS（即便 service 当前为空实现，前置防护避免未来埋雷）
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let (verifications, total) = service
        .list_verifications(
            page,
            page_size,
            query.invoice_id,
            query.payment_id,
            query.status,
        )
        .await
        .map_err(|e| AppError::internal(format!("获取核销列表失败: {}", e)))?;

    let result = serde_json::json!({
        "list": verifications,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取核销详情
/// GET /api/v1/erp/ar/verifications/:id
pub async fn get_verification(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let verification = service
        .get_verification(id)
        .await
        .map_err(|e| AppError::internal(format!("获取核销详情失败: {}", e)))?;

    Ok(Json(ApiResponse::success(verification)))
}

/// 自动核销
/// POST /api/v1/erp/ar/verifications/auto
pub async fn auto_verify(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let result = service
        .auto_verify(auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("自动核销失败: {}", e)))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 手动核销
/// POST /api/v1/erp/ar/verifications/manual
pub async fn manual_verify(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<ManualVerifyRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let verification = service
        .manual_verify(
            payload.invoice_id,
            payload.payment_id,
            payload.amount,
            payload.remark,
            auth.user_id,
        )
        .await
        .map_err(|e| AppError::internal(format!("手动核销失败: {}", e)))?;

    Ok(Json(ApiResponse::success(verification)))
}

/// 取消核销
/// POST /api/v1/erp/ar/verifications/:id/cancel
pub async fn cancel_verification(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let result = service
        .cancel_verification(id, auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("取消核销失败: {}", e)))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取未核销发票
/// GET /api/v1/erp/ar/verifications/unverified/invoices
pub async fn get_unverified_invoices(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let invoices = service
        .get_unverified_invoices(query)
        .await
        .map_err(|e| AppError::internal(format!("获取未核销发票失败: {}", e)))?;

    Ok(Json(ApiResponse::success(invoices)))
}

/// 获取未核销收款
/// GET /api/v1/erp/ar/verifications/unverified/payments
pub async fn get_unverified_payments(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payments = service
        .get_unverified_payments(query)
        .await
        .map_err(|e| AppError::internal(format!("获取未核销收款失败: {}", e)))?;

    Ok(Json(ApiResponse::success(payments)))
}
