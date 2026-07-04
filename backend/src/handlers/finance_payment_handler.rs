//! 通用财务支付网关 Handler
//!
//! 区别于 ap_payment (仅限应付账款)，此模块提供全局统一的财务支付入口
//! 能够接收来自采购、销售、退货、人工调整等所有渠道的支付动作。

use crate::middleware::auth_context::AuthContext;
use crate::services::finance_payment_service::FinancePaymentService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

// 批次 98 P2-B 修复（v5 复审）：本地 validate_amount_range 已抽取到 utils::validator 模块，
// 统一追加 round_dp(2) 精度校验。#[validate(custom)] 引用改为 crate::utils::validator::validate_amount_range。

/// 通用财务支付请求 DTO
/// 批次 31 v7 P1-6 修复：添加 Validate + 字段验证
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePaymentRequest {
    #[validate(length(max = 50, message = "支付单号长度不能超过50字符"))]
    pub payment_no: Option<String>,
    pub invoice_id: Option<i32>,
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub amount: Decimal,
    pub payment_date: Option<DateTime<Utc>>,
    #[validate(length(max = 50, message = "支付方式长度不能超过50字符"))]
    pub payment_method: Option<String>,
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    pub id: i32,
    pub payment_no: String,
    pub amount: Decimal,
    pub status: String,
    pub payment_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PaymentListResponse {
    pub payments: Vec<PaymentResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

pub async fn get_payment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<PaymentResponse>>, AppError> {
    let service = FinancePaymentService::new(state.db.clone());

    let payment = service
        .find_by_id(id)
        .await
        .map_err(|e| AppError::not_found(e.to_string()))?;

    Ok(Json(ApiResponse::success(PaymentResponse {
        id: payment.id,
        payment_no: payment.payment_no,
        amount: payment.amount,
        status: payment.status,
        payment_date: payment.payment_date,
        created_at: payment.created_at,
    })))
}

pub async fn create_payment(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<CreatePaymentRequest>,
) -> Result<Json<ApiResponse<PaymentResponse>>, AppError> {
    // v8 P1-C 修复：调用 DTO 验证，激活 Validate 注解
    payload.validate()?;
    let service = FinancePaymentService::new(state.db.clone());

    // 自动生成付款单号
    let payment_no = payload.payment_no.unwrap_or_else(|| {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_4_digit();
        format!("PAY-{}-{:04}", timestamp, random)
    });

    let payment = service
        .create_payment(
            payment_no,
            payload.invoice_id,
            payload.amount,
            payload.payment_date.unwrap_or_else(chrono::Utc::now),
            payload.payment_method,
            payload.notes,
            Some(auth.user_id),
        )
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(PaymentResponse {
        id: payment.id,
        payment_no: payment.payment_no,
        amount: payment.amount,
        status: payment.status,
        payment_date: payment.payment_date,
        created_at: payment.created_at,
    })))
}

pub async fn list_payments(
    State(state): State<AppState>,
    Query(params): Query<ListPaymentsParams>,
) -> Result<Json<ApiResponse<PaymentListResponse>>, AppError> {
    let service = FinancePaymentService::new(state.db.clone());

    let (payments, total) = service
        .list_payments(
            params.page.unwrap_or(1).clamp(1, 1000),
            params.page_size.unwrap_or(20).clamp(1, 100),
            params.status,
        )
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let payment_responses: Vec<PaymentResponse> = payments
        .into_iter()
        .map(|payment| PaymentResponse {
            id: payment.id,
            payment_no: payment.payment_no,
            amount: payment.amount,
            status: payment.status,
            payment_date: payment.payment_date,
            created_at: payment.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(PaymentListResponse {
        payments: payment_responses,
        total,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(20).clamp(1, 100),
    })))
}

#[derive(Debug, Deserialize)]
pub struct ListPaymentsParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
}
