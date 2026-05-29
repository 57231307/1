//! 应收对账增强 Handler
//!
//! 提供自动对账、账龄分析、对账明细、客户确认/争议处理等 HTTP 接口

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::ar_reconciliation_service::{
    ArReconciliationService, AutoMatchRequest, GenerateReconciliationRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use serde_json::Value as JsonValue;
use tracing::info;

/// 自动对账请求参数
#[derive(Debug, Deserialize)]
pub struct AutoMatchQueryParams {
    pub customer_id: Option<i32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub match_strategy: Option<String>,
}

/// 账龄分析查询参数
#[derive(Debug, Deserialize)]
pub struct AgingReportQueryParams {
    pub customer_id: Option<i32>,
}

/// 确认对账单请求
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    pub notes: Option<String>,
}

/// 争议处理请求
#[derive(Debug, Deserialize)]
pub struct DisputeRequest {
    pub reason: String,
}

/// POST /api/v1/erp/ar-reconciliations/auto-match
///
/// 自动对账：按客户匹配发票和收款，生成对账单及明细
pub async fn auto_match(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<AutoMatchQueryParams>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 执行自动对账，客户ID: {:?}, 期间: {} ~ {}",
        auth.username, req.customer_id, req.start_date, req.end_date
    );

    let service = ArReconciliationService::new(state.db.clone());
    let match_req = AutoMatchRequest {
        customer_id: req.customer_id,
        start_date: req.start_date,
        end_date: req.end_date,
        match_strategy: req.match_strategy,
    };

    let results = service.auto_match(match_req, auth.user_id).await?;

    let success_count = results.len();
    info!(
        "用户 {} 自动对账完成，处理 {} 个客户",
        auth.username, success_count
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(&results)?,
        &format!("自动对账完成，共处理 {} 个客户", success_count),
    )))
}

/// GET /api/v1/erp/ar-reconciliations/aging-report
///
/// 账龄分析：按 0-30天/31-60天/61-90天/90天以上 分桶统计
pub async fn aging_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<AgingReportQueryParams>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 查询账龄分析，客户ID: {:?}",
        auth.username, params.customer_id
    );

    let service = ArReconciliationService::new(state.db.clone());
    let report = service.get_aging_report(params.customer_id).await?;

    info!(
        "用户 {} 账龄分析完成，应收总额: {}",
        auth.username, report.total_receivable
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(report)?)))
}

/// GET /api/v1/erp/ar-reconciliations/:id/details
///
/// 获取对账单明细：返回对账单及其所有明细行
pub async fn get_reconciliation_details(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 查询对账单明细 ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());
    let details = service.get_with_details(id).await?;

    info!(
        "用户 {} 查询对账单明细成功，共 {} 条明细",
        auth.username,
        details.details.len()
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(details)?)))
}

/// POST /api/v1/erp/ar-reconciliations/:id/confirm
///
/// 客户确认对账单
pub async fn confirm_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(_req): Json<ConfirmRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 确认对账单 ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());
    let reconciliation = service.customer_confirm(id, auth.user_id).await?;

    info!(
        "用户 {} 确认对账单成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "对账单确认成功",
    )))
}

/// POST /api/v1/erp/ar-reconciliations/:id/dispute
///
/// 客户对对账单提出争议
pub async fn dispute_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<DisputeRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 对账单 ID: {} 提出争议，原因：{}",
        auth.username, id, req.reason
    );

    let service = ArReconciliationService::new(state.db.clone());
    let reconciliation = service
        .customer_dispute(id, req.reason, auth.user_id)
        .await?;

    info!(
        "用户 {} 对账单争议提交成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "争议提交成功",
    )))
}

/// POST /api/v1/erp/ar-reconciliations/generate
///
/// 为指定客户自动生成对账单（从发票/收款汇总）
pub async fn generate_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<GenerateReconciliationApiRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 生成对账单，客户 ID: {}",
        auth.username, req.customer_id
    );

    let service = ArReconciliationService::new(state.db.clone());
    let gen_req = GenerateReconciliationRequest {
        customer_id: req.customer_id,
        start_date: req.start_date,
        end_date: req.end_date,
        notes: req.notes,
    };

    let reconciliation = service
        .generate_reconciliation(gen_req, auth.user_id)
        .await?;

    info!(
        "用户 {} 生成对账单成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "对账单生成成功",
    )))
}

/// 生成对账单 API 请求体
#[derive(Debug, Deserialize)]
pub struct GenerateReconciliationApiRequest {
    pub customer_id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub notes: Option<String>,
}

/// GET /api/v1/erp/ar-reconciliations/:id/pdf - 导出对账单PDF
pub async fn export_reconciliation_pdf(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 导出对账单PDF，ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());
    let pdf_content = service.export_pdf(id).await?;

    // 返回base64编码的PDF内容
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&pdf_content);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({
            "id": id,
            "content_type": "application/pdf",
            "size": pdf_content.len(),
            "base64_content": encoded,
        }),
        "PDF导出成功",
    )))
}
