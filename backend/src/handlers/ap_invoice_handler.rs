//! 应付单 Handler
//!
//! 应付单 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::services::ap_invoice_service::{
    ApInvoiceService, CreateApInvoiceRequest, UpdateApInvoiceRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use sea_orm::DatabaseConnection;
use crate::utils::app_state::AppState;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, warn};
use validator::Validate;

/// 查询应付单列表参数
#[derive(Debug, Deserialize)]
pub struct ApInvoiceQueryParams {
    pub supplier_id: Option<i32>,
    pub invoice_status: Option<String>,
    pub invoice_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询应付单列表
pub async fn list_invoices(
    Query(params): Query<ApInvoiceQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询应付单列表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApInvoiceService::new(db);
    let (invoices, total) = service
        .get_list(
            params.supplier_id,
            params.invoice_status,
            params.invoice_type,
            params.start_date,
            params.end_date,
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
        )
        .await?;

    info!("查询成功，共 {} 条记录", total);

    let result = serde_json::json!({
        "items": invoices,
        "total": total,
        "page": params.page.unwrap_or(1),
        "page_size": params.page_size.unwrap_or(20),
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取应付单详情
pub async fn get_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询应付单详情 ID: {}", auth.username, id);

    let service = ApInvoiceService::new(db);
    let invoice = service.get_by_id(id).await?;

    info!("查询成功：{}", invoice.invoice_no);

    Ok(Json(ApiResponse::success(serde_json::to_value(invoice)?)))
}

/// 创建应付单
#[axum::debug_handler]
pub async fn create_invoice(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateApInvoiceRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 创建应付单，供应商 ID: {}",
        auth.username, req.supplier_id
    );

    // 验证请求
    req.validate().map_err(|e| {
        warn!("用户 {} 创建应付单验证失败：{}", auth.username, e);
        AppError::ValidationError(e.to_string())
    })?;

    let service = ApInvoiceService::new(db);
    let invoice = service.create_manual(req, auth.user_id).await?;

    info!(
        "用户 {} 创建应付单成功：{}",
        auth.username, invoice.invoice_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice)?,
        "应付单创建成功",
    )))
}

/// 更新应付单
#[axum::debug_handler]
pub async fn update_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateApInvoiceRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新应付单 ID: {}", auth.username, id);

    let service = ApInvoiceService::new(db);
    let invoice = service.update(id, req, auth.user_id).await?;

    info!(
        "用户 {} 更新应付单成功：{}",
        auth.username, invoice.invoice_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice)?,
        "应付单更新成功",
    )))
}

/// 删除应付单
pub async fn delete_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除应付单 ID: {}", auth.username, id);

    let service = ApInvoiceService::new(db);
    service.delete(id).await?;

    info!("用户 {} 删除应付单成功", auth.username);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "应付单删除成功",
    )))
}

/// 审核应付单
pub async fn approve_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 审核应付单 ID: {}", auth.username, id);

    let service = ApInvoiceService::new(db);
    let invoice = service.approve(id, auth.user_id).await?;

    info!(
        "用户 {} 审核应付单成功：{}",
        auth.username, invoice.invoice_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice)?,
        "应付单审核成功",
    )))
}

/// 取消应付单
#[derive(Debug, Deserialize)]
pub struct CancelInvoiceRequest {
    pub reason: String,
}

pub async fn cancel_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelInvoiceRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 取消应付单 ID: {}, 原因：{}",
        auth.username, id, req.reason
    );

    let service = ApInvoiceService::new(db);
    let invoice = service.cancel(id, req.reason.clone(), auth.user_id).await?;

    info!(
        "用户 {} 取消应付单成功：{}",
        auth.username, invoice.invoice_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice)?,
        "应付单取消成功",
    )))
}

/// 自动生成应付单（从采购入库单）
#[derive(Debug, Deserialize)]
pub struct AutoGenerateRequest {
    pub receipt_id: i32,
}

pub async fn auto_generate(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<AutoGenerateRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 自动生成应付单，入库单 ID: {}",
        auth.username, req.receipt_id
    );

    let service = ApInvoiceService::new(db);
    let invoice = service
        .auto_generate_from_receipt(req.receipt_id, auth.user_id)
        .await?;

    info!(
        "用户 {} 自动生成应付单成功：{}",
        auth.username, invoice.invoice_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice)?,
        "应付单自动生成成功",
    )))
}

/// 获取账龄分析
pub async fn get_aging_analysis(
    Query(params): Query<ApInvoiceQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询账龄分析，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApInvoiceService::new(db);
    let aging_data = service.get_aging_analysis(params.supplier_id).await?;

    info!("查询账龄分析成功");

    Ok(Json(ApiResponse::success(serde_json::to_value(
        aging_data,
    )?)))
}

/// 获取应付余额表
pub async fn get_balance_summary(
    Query(params): Query<ApInvoiceQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询应付余额表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApInvoiceService::new(db);
    let summary = service.get_balance_summary(params.supplier_id).await?;

    info!("查询应付余额表成功");

    Ok(Json(ApiResponse::success(serde_json::to_value(summary)?)))
}

/// 获取应付统计报表
pub async fn get_statistics(
    Query(_params): Query<ApInvoiceQueryParams>,
    State(_state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询应付统计报表", auth.username);

    // TODO: 实现统计报表
    let result = serde_json::json!({
        "message": "统计报表功能开发中"
    });

    Ok(Json(ApiResponse::success(result)))
}
