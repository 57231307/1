//! 付款 Handler
//!
//! 付款 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::services::ap_payment_service::{
    ApPaymentService, CreateApPaymentRequest, UpdateApPaymentRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use validator::Validate;

/// 查询付款列表参数
#[derive(Debug, Deserialize)]
pub struct ApPaymentQueryParams {
    pub supplier_id: Option<i32>,
    pub payment_status: Option<String>,
    pub payment_method: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询付款列表
pub async fn list_payments(
    Query(params): Query<ApPaymentQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 查询付款列表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApPaymentService::new(state.db.clone());
    let (payments, total) = service
        .get_list(
            params.supplier_id,
            params.payment_status,
            params.payment_method,
            params.start_date,
            params.end_date,
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
        )
        .await?;

    info!("用户 {} 查询付款成功，共 {} 条记录", auth.username, total);

    let result = serde_json::json!({
        "items": payments,
        "total": total,
        "page": params.page.unwrap_or(1),
        "page_size": params.page_size.unwrap_or(20),
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取付款详情
pub async fn get_payment(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 查询付款详情 ID: {}", auth.username, id);

    let service = ApPaymentService::new(state.db.clone());
    let payment = service.get_by_id(id).await?;

    info!(
        "用户 {} 查询付款详情成功：{}",
        auth.username, payment.payment_no
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(payment)?)))
}

/// 创建付款
#[axum::debug_handler]
pub async fn create_payment(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateApPaymentRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 创建付款，付款申请 ID: {}",
        auth.username, req.request_id
    );

    req.validate().map_err(|e| {
        warn!("用户 {} 创建付款验证失败：{}", auth.username, e);
        AppError::ValidationError(e.to_string())
    })?;

    let service = ApPaymentService::new(state.db.clone());
    let payment = service.create(req, auth.user_id).await?;

    info!(
        "用户 {} 创建付款成功：{}",
        auth.username, payment.payment_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(payment)?,
        "付款创建成功",
    )))
}

/// 更新付款
#[axum::debug_handler]
pub async fn update_payment(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateApPaymentRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 更新付款 ID: {}", auth.username, id);

    req.validate().map_err(|e| {
        warn!("用户 {} 更新付款验证失败：{}", auth.username, e);
        AppError::ValidationError(e.to_string())
    })?;

    let service = ApPaymentService::new(state.db.clone());
    let payment = service.update(id, req, auth.user_id).await?;

    info!(
        "用户 {} 更新付款成功：{}",
        auth.username, payment.payment_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(payment)?,
        "付款更新成功",
    )))
}

/// 确认付款
pub async fn confirm_payment(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 确认付款 ID: {}", auth.username, id);

    let service = ApPaymentService::new(state.db.clone());
    let payment = service.confirm(id, auth.user_id).await?;

    info!(
        "用户 {} 确认付款成功：{}",
        auth.username, payment.payment_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(payment)?,
        "付款确认成功",
    )))
}

/// 获取付款计划
#[derive(Debug, Deserialize)]
pub struct PaymentScheduleParams {
    pub supplier_id: Option<i32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

pub async fn get_payment_schedule(
    Query(params): Query<PaymentScheduleParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 查询付款计划，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApPaymentService::new(state.db.clone());
    let schedule = service
        .get_payment_schedule(params.supplier_id, params.start_date, params.end_date)
        .await?;

    info!("用户 {} 查询付款计划成功", auth.username);

    Ok(Json(ApiResponse::success(serde_json::to_value(schedule)?)))
}
