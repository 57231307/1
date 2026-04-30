//! 应付核销 Handler
//!
//! 应付核销 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::services::ap_verification_service::{ApVerificationService, ManualVerifyRequest};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use validator::Validate;

/// 查询核销列表参数
#[derive(Debug, Deserialize)]
pub struct ApVerificationQueryParams {
    pub supplier_id: Option<i32>,
    pub verification_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询核销列表
pub async fn list_verifications(
    Query(params): Query<ApVerificationQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 查询核销列表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApVerificationService::new(state.db.clone());
    let (verifications, total) = service
        .get_list(
            params.supplier_id,
            params.verification_type,
            params.start_date,
            params.end_date,
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
        )
        .await?;

    info!("用户 {} 查询核销成功，共 {} 条记录", auth.username, total);

    let result = serde_json::json!({
        "items": verifications,
        "total": total,
        "page": params.page.unwrap_or(1),
        "page_size": params.page_size.unwrap_or(20),
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取核销详情
pub async fn get_verification(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 查询核销详情 ID: {}", auth.username, id);

    let service = ApVerificationService::new(state.db.clone());
    let verification = service.get_by_id(id).await?;

    info!(
        "用户 {} 查询核销详情成功：{}",
        auth.username, verification.verification_no
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(
        verification,
    )?)))
}

/// 自动核销
#[derive(Debug, Deserialize, Serialize)]
pub struct AutoVerifyRequest {
    pub supplier_id: i32,
}

pub async fn auto_verify(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<AutoVerifyRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 执行自动核销，供应商 ID: {}",
        auth.username, req.supplier_id
    );

    let service = ApVerificationService::new(state.db.clone());
    let verification = service.auto_verify(req.supplier_id, auth.user_id).await?;

    info!(
        "用户 {} 自动核销成功：{}",
        auth.username, verification.verification_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(verification)?,
        "自动核销成功",
    )))
}

/// 手工核销
#[axum::debug_handler]
pub async fn manual_verify(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ManualVerifyRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 执行手工核销，供应商 ID: {}",
        auth.username, req.supplier_id
    );

    req.validate().map_err(|e| {
        warn!("用户 {} 手工核销验证失败：{}", auth.username, e);
        AppError::ValidationError(e.to_string())
    })?;

    let service = ApVerificationService::new(state.db.clone());
    let verification = service.manual_verify(req, auth.user_id).await?;

    info!(
        "用户 {} 手工核销成功：{}",
        auth.username, verification.verification_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(verification)?,
        "手工核销成功",
    )))
}

/// 取消核销
#[derive(Debug, Deserialize, Serialize)]
pub struct CancelVerificationRequest {
    pub reason: String,
}

pub async fn cancel_verification(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelVerificationRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 取消核销 ID: {}, 原因：{}",
        auth.username, id, req.reason
    );

    let service = ApVerificationService::new(state.db.clone());
    let verification = service.cancel(id, req.reason, auth.user_id).await?;

    info!(
        "用户 {} 取消核销成功：{}",
        auth.username, verification.verification_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(verification)?,
        "核销已取消",
    )))
}

/// 获取未核销应付单列表
pub async fn get_unverified_invoices(
    Query(params): Query<ApVerificationQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let supplier_id = params.supplier_id.ok_or_else(|| {
        warn!("用户 {} 查询未核销应付单列表未提供供应商 ID", auth.username);
        AppError::BadRequest("必须提供供应商 ID".to_string())
    })?;

    info!(
        "用户 {} 查询未核销应付单列表，供应商 ID: {}",
        auth.username, supplier_id
    );

    let service = ApVerificationService::new(state.db.clone());
    let invoices = service.get_unverified_invoices(supplier_id).await?;

    info!(
        "用户 {} 查询未核销应付单成功，共 {} 条",
        auth.username,
        invoices.len()
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(invoices)?)))
}

/// 获取未核销付款单列表
pub async fn get_unverified_payments(
    Query(params): Query<ApVerificationQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let supplier_id = params.supplier_id.ok_or_else(|| {
        warn!("用户 {} 查询未核销付款单列表未提供供应商 ID", auth.username);
        AppError::BadRequest("必须提供供应商 ID".to_string())
    })?;

    info!(
        "用户 {} 查询未核销付款单列表，供应商 ID: {}",
        auth.username, supplier_id
    );

    let service = ApVerificationService::new(state.db.clone());
    let payments = service.get_unverified_payments(supplier_id).await?;

    info!(
        "用户 {} 查询未核销付款单成功，共 {} 条",
        auth.username,
        payments.len()
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(payments)?)))
}
