//! 应付单 Handler
//!
//! 应付单 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::ap_invoice_service::{
    ApInvoiceListQuery, ApInvoiceService, CreateApInvoiceRequest, UpdateApInvoiceRequest,
};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use crate::utils::xlsx_export::{build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, warn};
use validator::Validate;

/// 查询应付单列表参数
// V15 P0-S12 修复（Batch 475e）：派生 Clone，export_ap_invoices 需要 clone 后覆盖分页参数用于全量导出
#[derive(Debug, Clone, Deserialize)]
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
pub async fn list_ap_invoices(
    Query(params): Query<ApInvoiceQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询应付单列表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApInvoiceService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let (invoices, total) = service
        .get_list(ApInvoiceListQuery {
            supplier_id: params.supplier_id,
            invoice_status: params.invoice_status,
            invoice_type: params.invoice_type,
            start_date: params.start_date,
            end_date: params.end_date,
            page,
            page_size,
        })
        .await?;

    info!("查询成功，共 {} 条记录", total);

    let result = serde_json::to_value(PaginatedResponse::new(
        invoices,
        total,
        page,
        page_size,
    ))
    .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取应付单详情
pub async fn get_ap_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询应付单详情 ID: {}", auth.username, id);

    let service = ApInvoiceService::new(state.db.clone());
    let invoice = service.get_by_id(id).await?;

    info!("查询成功：{}", invoice.invoice_no);

    Ok(Json(ApiResponse::success(serde_json::to_value(invoice)?)))
}

/// 创建应付单
#[axum::debug_handler]
pub async fn create_ap_invoice(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateApInvoiceRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 创建应付单，供应商 ID: {:?}",
        auth.username, req.supplier_id
    );

    // 验证请求
    req.validate().map_err(|e| {
        warn!("用户 {} 创建应付单验证失败：{}", auth.username, e);
        AppError::validation(e.to_string())
    })?;

    let service = ApInvoiceService::new(state.db.clone());
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
pub async fn update_ap_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateApInvoiceRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新应付单 ID: {}", auth.username, id);

    // TS-S-5 安全加固（2026-06-26）：补齐 validate 调用
    req.validate().map_err(|e| AppError::validation(e.to_string()))?;

    let service = ApInvoiceService::new(state.db.clone());
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
pub async fn delete_ap_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除应付单 ID: {}", auth.username, id);

    let service = ApInvoiceService::new(state.db.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service.delete(id, auth.user_id).await?;

    info!("用户 {} 删除应付单成功", auth.username);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "应付单删除成功",
    )))
}

/// 审核应付单
pub async fn approve_ap_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 审核应付单 ID: {}", auth.username, id);

    let service = ApInvoiceService::new(state.db.clone());
    let invoice = service.approve(id, auth.user_id).await?;

    // 记录增强资金操作日志
    tracing::warn!(
        target: "financial_audit",
        "[资金操作] 操作: APPROVE | 类型: AP_INVOICE | 单号: {} | ID: {} | 操作人: {}({}) | IP: {}",
        invoice.invoice_no,
        id,
        auth.username,
        auth.user_id,
        "N/A"
    );

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

pub async fn cancel_ap_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelInvoiceRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 取消应付单 ID: {}, 原因：{}",
        auth.username, id, req.reason
    );

    let service = ApInvoiceService::new(state.db.clone());
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

    let service = ApInvoiceService::new(state.db.clone());
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

    let service = ApInvoiceService::new(state.db.clone());
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

    let service = ApInvoiceService::new(state.db.clone());
    let summary = service.get_balance_summary(params.supplier_id).await?;

    info!("查询应付余额表成功");

    Ok(Json(ApiResponse::success(serde_json::to_value(summary)?)))
}

/// 获取应付统计报表
///
/// 批次 133 v9 复审 P1：原返回 "统计报表功能开发中" 占位，
/// 现综合调用 get_balance_summary + get_aging_analysis + 按状态分组统计，
/// 返回完整统计报表（余额汇总 + 账龄分析 + 状态分布）。
pub async fn get_statistics(
    Query(params): Query<ApInvoiceQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询应付统计报表", auth.username);

    let service = ApInvoiceService::new(state.db.clone());
    let statistics = service.get_statistics(params.supplier_id).await?;

    info!("查询应付统计报表成功");

    Ok(Json(ApiResponse::success(serde_json::to_value(
        statistics,
    )?)))
}

/// GET /api/v1/erp/ap/invoices/export - 导出应付发票列表（带水印 + 异步审计日志）
///
/// V15 P0-S12 修复（Batch 475e）：导出接入后端
/// - 注入水印（operator/exported_at/extra 含条数）
/// - 异步审计日志（OperationType::Export）
/// - 直接调 service.get_list 取全量数据（page=1/page_size=10000）
pub async fn export_ap_invoices(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ApInvoiceQueryParams>,
) -> Result<axum::response::Response, AppError> {
    let service = ApInvoiceService::new(state.db.clone());

    let (invoices, _total) = service
        .get_list(ApInvoiceListQuery {
            supplier_id: query.supplier_id,
            invoice_status: query.invoice_status,
            invoice_type: query.invoice_type,
            start_date: query.start_date,
            end_date: query.end_date,
            page: 1,
            page_size: 10000,
        })
        .await?;
    let row_count = invoices.len();

    let invoices_json: Vec<serde_json::Value> = invoices
        .into_iter()
        .map(|i| serde_json::to_value(i).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    let headers: Vec<String> = vec![
        "ID".to_string(),
        "发票编号".to_string(),
        "供应商ID".to_string(),
        "供应商名称".to_string(),
        "发票日期".to_string(),
        "到期日".to_string(),
        "发票金额".to_string(),
        "税额".to_string(),
        "发票类型".to_string(),
        "状态".to_string(),
        "创建时间".to_string(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(invoices_json.len());
    for i in invoices_json {
        let obj = i.as_object().ok_or_else(|| {
            AppError::internal("应付发票序列化失败：期望 JSON 对象")
        })?;
        let get_str = |key: &str| -> String {
            obj.get(key)
                .map(|v| {
                    if v.is_null() {
                        String::new()
                    } else if v.is_string() {
                        v.as_str().unwrap_or("").to_string()
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_default()
        };
        rows.push(vec![
            get_str("id"),
            get_str("invoice_no"),
            get_str("supplier_id"),
            get_str("supplier_name"),
            get_str("invoice_date"),
            get_str("due_date"),
            get_str("invoice_amount"),
            get_str("tax_amount"),
            get_str("invoice_type"),
            get_str("invoice_status"),
            get_str("created_at"),
        ]);
    }

    let table = XlsxTable {
        sheet_name: "应付发票".to_string(),
        headers,
        rows,
    };

    let filename = format!(
        "ap_invoices_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("ap_invoice".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出应付发票列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/ap/invoices/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("应付发票导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
