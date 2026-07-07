use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// 批次 98 P2-B 修复（v5 复审）：本地 validate_amount_range 已抽取到 utils::validator 模块，
// 统一追加 round_dp(2) 精度校验。#[validate(custom)] 引用改为 crate::utils::validator::validate_amount_range。

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
/// 批次 31 v7 P1-6 修复：添加 Validate + 字段验证
#[derive(Debug, Deserialize, Validate)]
pub struct CreateArPaymentRequest {
    pub customer_id: i32,
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub amount: rust_decimal::Decimal,
    #[validate(length(min = 1, max = 50, message = "收款方式长度必须在1到50字符之间"))]
    pub payment_method: String,
    pub payment_date: chrono::NaiveDate,
    #[validate(length(max = 50, message = "银行账号长度不能超过50字符"))]
    pub bank_account: Option<String>,
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
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

/// 获取收款列表
/// GET /api/v1/erp/ar/payments
pub async fn list_payments(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArPaymentQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

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
    // v8 P1-C 修复：调用 DTO 验证，激活 Validate 注解
    payload.validate()?;
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

/// 取消收款（批次 158 v11 真实接入）
/// POST /api/v1/erp/ar/payments/:id/cancel
pub async fn cancel_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payment = service
        .cancel_collection(id, auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("取消收款失败: {}", e)))?;

    Ok(Json(ApiResponse::success(payment)))
}
