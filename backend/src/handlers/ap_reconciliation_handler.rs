//! 供应商对账 Handler
//!
//! 供应商对账 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::services::ap_reconciliation_service::{
    ApReconciliationService, GenerateReconciliationRequest,
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
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::{info, warn};
use validator::Validate;

/// 查询对账单列表参数
#[derive(Debug, Deserialize)]
pub struct ApReconciliationQueryParams {
    pub supplier_id: Option<i32>,
    pub reconciliation_status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询对账单列表
pub async fn list_reconciliations(
    Query(params): Query<ApReconciliationQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 查询对账单列表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApReconciliationService::new(state.db.clone());
    let (reconciliations, total) = service
        .get_list(
            params.supplier_id,
            params.reconciliation_status,
            params.start_date,
            params.end_date,
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
        )
        .await?;

    info!("用户 {} 查询对账单成功，共 {} 条记录", auth.username, total);

    let result = serde_json::json!({
        "items": reconciliations,
        "total": total,
        "page": params.page.unwrap_or(1),
        "page_size": params.page_size.unwrap_or(20),
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取对账单详情
pub async fn get_reconciliation(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 查询对账单详情 ID: {}", auth.username, id);

    let service = ApReconciliationService::new(state.db.clone());
    let reconciliation = service.get_by_id(id).await?;

    info!(
        "用户 {} 查询对账单详情成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(
        reconciliation,
    )?)))
}

/// 生成对账单
#[axum::debug_handler]
pub async fn generate_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<GenerateReconciliationRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 生成对账单，供应商 ID: {}",
        auth.username, req.supplier_id
    );

    req.validate().map_err(|e| {
        warn!("用户 {} 生成对账单验证失败：{}", auth.username, e);
        AppError::ValidationError(e.to_string())
    })?;

    let service = ApReconciliationService::new(state.db.clone());
    let reconciliation = service.generate_reconciliation(req, auth.user_id).await?;

    info!(
        "用户 {} 生成对账单成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "对账单生成成功",
    )))
}

/// 确认对账单
pub async fn confirm_reconciliation(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 确认对账单 ID: {}", auth.username, id);

    let service = ApReconciliationService::new(state.db.clone());
    let reconciliation = service.confirm_reconciliation(id, auth.user_id).await?;

    info!(
        "用户 {} 确认对账单成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "对账单确认成功",
    )))
}

/// 提出争议
#[derive(Debug, Deserialize, Serialize)]
pub struct DisputeRequest {
    pub reason: String,
}

pub async fn dispute_reconciliation(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<DisputeRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 提出对账单争议 ID: {}, 原因：{}",
        auth.username, id, req.reason
    );

    let service = ApReconciliationService::new(state.db.clone());
    let reconciliation = service.dispute(id, req.reason, auth.user_id).await?;

    info!(
        "用户 {} 提出对账单争议成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "已提出争议",
    )))
}

/// 获取供应商应付汇总
pub async fn get_supplier_summary(
    Query(params): Query<ApReconciliationQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 查询供应商应付汇总，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApReconciliationService::new(state.db.clone());
    let summary = service.get_supplier_summary(params.supplier_id).await?;

    info!("用户 {} 查询供应商应付汇总成功", auth.username);

    Ok(Json(ApiResponse::success(serde_json::to_value(summary)?)))
}
