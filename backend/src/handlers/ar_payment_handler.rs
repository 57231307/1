use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 收款查询参数
#[derive(Debug, Deserialize)]
pub struct ArPaymentQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub payment_no: Option<String>,
}

/// 创建收款请求
#[derive(Debug, Deserialize)]
pub struct CreateArPaymentRequest {
    pub customer_id: i32,
    pub amount: rust_decimal::Decimal,
    pub payment_method: String,
    pub payment_date: chrono::NaiveDate,
    pub bank_account: Option<String>,
    pub remark: Option<String>,
    pub invoice_ids: Option<Vec<i32>>,
}

/// 更新收款请求
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateArPaymentRequest {
    pub amount: Option<rust_decimal::Decimal>,
    pub payment_method: Option<String>,
    pub payment_date: Option<chrono::NaiveDate>,
    pub bank_account: Option<String>,
    pub remark: Option<String>,
}

/// 收款响应
#[derive(Debug, Serialize)]
pub struct ArPaymentResponse {
    pub id: i32,
    pub payment_no: String,
    pub customer_id: i32,
    pub amount: rust_decimal::Decimal,
    pub payment_method: String,
    pub payment_date: chrono::NaiveDate,
    pub status: String,
    pub bank_account: Option<String>,
    pub remark: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 获取收款列表
/// GET /api/v1/erp/ar/payments
pub async fn list_payments(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArPaymentQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let (payments, total) = service
        .list_payments(
            page,
            page_size,
            query.status,
            query.customer_id,
            query.payment_no,
        )
        .await
        .map_err(|e| AppError::internal(format!("获取收款列表失败: {}", e)))?;

    let result = serde_json::json!({
        "list": payments,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取收款详情
/// GET /api/v1/erp/ar/payments/:id
pub async fn get_payment(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payment = service
        .get_payment(id)
        .await
        .map_err(|e| AppError::internal(format!("获取收款详情失败: {}", e)))?;

    Ok(Json(ApiResponse::success(payment)))
}

/// 创建收款
/// POST /api/v1/erp/ar/payments
pub async fn create_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<CreateArPaymentRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payment = service
        .create_payment(
            payload.customer_id,
            payload.amount,
            payload.payment_method,
            payload.payment_date,
            payload.bank_account,
            payload.remark,
            payload.invoice_ids,
            auth.user_id,
        )
        .await
        .map_err(|e| AppError::internal(format!("创建收款失败: {}", e)))?;

    Ok(Json(ApiResponse::success(payment)))
}

/// 更新收款
/// PUT /api/v1/erp/ar/payments/:id
pub async fn update_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateArPaymentRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payload_json = serde_json::to_value(payload)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;

    let payment = service
        .update_payment(id, payload_json, auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("更新收款失败: {}", e)))?;

    Ok(Json(ApiResponse::success(payment)))
}

/// 确认收款
/// POST /api/v1/erp/ar/payments/:id/confirm
pub async fn confirm_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payment = service
        .confirm_payment(id, auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("确认收款失败: {}", e)))?;

    Ok(Json(ApiResponse::success(payment)))
}
