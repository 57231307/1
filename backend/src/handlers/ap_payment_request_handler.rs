//! 付款申请 Handler
//!
//! 付款申请 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::services::ap_payment_request_service::{
    ApPaymentRequestService, CreateApPaymentRequest, UpdateApPaymentRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use validator::Validate;

/// 查询付款申请列表参数
#[derive(Debug, Deserialize)]
pub struct ApPaymentRequestQueryParams {
    pub supplier_id: Option<i32>,
    pub approval_status: Option<String>,
    pub payment_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询付款申请列表
pub async fn list_requests(
    Query(params): Query<ApPaymentRequestQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询付款申请列表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApPaymentRequestService::new(state.db.clone());
    let (requests, total) = service
        .get_list(
            params.supplier_id,
            params.approval_status,
            params.payment_type,
            params.start_date,
            params.end_date,
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
        )
        .await?;

    info!(
        "用户 {} 查询付款申请成功，共 {} 条记录",
        auth.username, total
    );

    let result = crate::utils::response::build_paginated_response(requests, total, params.page.unwrap_or(1), params.page_size.unwrap_or(20));

    Ok(Json(ApiResponse::success(result)))
}

/// 获取付款申请详情
pub async fn get_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询付款申请详情 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.get_by_id(id).await?;

    info!(
        "用户 {} 查询付款申请详情成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(request)?)))
}

/// 创建付款申请
#[axum::debug_handler]
pub async fn create_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateApPaymentRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 创建付款申请，供应商 ID: {}",
        auth.username, req.supplier_id
    );

    req.validate().map_err(|e| {
        warn!("用户 {} 创建付款申请验证失败：{}", auth.username, e);
        AppError::ValidationError(e.to_string())
    })?;

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.create(req, auth.user_id).await?;

    info!(
        "用户 {} 创建付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请创建成功",
    )))
}

/// 更新付款申请
#[axum::debug_handler]
pub async fn update_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateApPaymentRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 更新付款申请 ID: {}", auth.username, id);

    req.validate().map_err(|e| {
        warn!("用户 {} 更新付款申请验证失败：{}", auth.username, e);
        AppError::ValidationError(e.to_string())
    })?;

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.update(id, req, auth.user_id).await?;

    info!(
        "用户 {} 更新付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请更新成功",
    )))
}

/// 删除付款申请
pub async fn delete_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    service.delete(id).await?;

    info!("用户 {} 删除付款申请成功", auth.username);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "付款申请删除成功",
    )))
}

/// 提交付款申请
pub async fn submit_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 提交付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.submit(id, auth.user_id).await?;

    info!(
        "用户 {} 提交付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请提交成功",
    )))
}

/// 审批付款申请
pub async fn approve_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 审批付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.approve(id, auth.user_id).await?;

    info!(
        "用户 {} 审批付款申请通过：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请审批通过",
    )))
}

/// 拒绝付款申请
#[derive(Debug, Deserialize, Serialize)]
pub struct RejectRequest {
    pub reason: String,
}

pub async fn reject_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RejectRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 拒绝付款申请 ID: {}, 原因：{}",
        auth.username, id, req.reason
    );

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.reject(id, req.reason, auth.user_id).await?;

    info!(
        "用户 {} 拒绝付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请已拒绝",
    )))
}
