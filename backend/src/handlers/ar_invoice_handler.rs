//! 应收单 Handler
//!
//! HTTP 接口层

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, warn};

use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::ar_invoice;
use crate::services::ar_invoice_service::{ArInvoiceService, CreateArInvoiceRequest};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::xlsx_export::{build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable};
use rust_decimal::Decimal;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct CancelReason {
    pub reason: String,
}

/// 查询参数
// V15 P0-S12 修复（Batch 475e）：派生 Clone，export_ar_invoices 需要 clone 后覆盖分页参数用于全量导出
#[derive(Debug, Clone, Deserialize)]
pub struct ArInvoiceQuery {
    pub customer_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建请求
#[derive(Debug, Deserialize)]

pub struct CreateArInvoiceRequestDto {
    pub invoice_date: Option<String>,
    pub due_date: Option<String>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub source_type: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub invoice_amount: Option<Decimal>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub sales_order_no: Option<String>,
}

/// 查询应收单列表
pub async fn list_ar_invoices(
    Query(params): Query<ArInvoiceQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ar_invoice::Model>>>, AppError> {
    info!("用户 {} 查询应收单列表", auth.username);

    let service = ArInvoiceService::new(state.db.clone());
    let (invoices, total) = service
        .get_list(
            params.customer_id,
            params.status,
            params.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
            params.page_size.unwrap_or(20).clamp(1, 100),
        )
        .await?;

    info!("用户 {} 查询应收单成功，共 {} 条", auth.username, total);

    Ok(Json(ApiResponse::success(invoices)))
}

/// 创建应收单
#[axum::debug_handler]
pub async fn create_ar_invoice(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateArInvoiceRequestDto>,
) -> Result<Json<ApiResponse<ar_invoice::Model>>, AppError> {
    info!(
        "用户 {} 创建应收单，客户 ID: {:?}",
        auth.username, req.customer_id
    );

    let invoice_date = req
        .invoice_date
        .map(|d| {
            d.parse().map_err(|e| {
                warn!("用户 {} 应收单日期格式错误：{}", auth.username, e);
                AppError::validation("应收单日期格式错误")
            })
        })
        .transpose()?;

    let due_date = req
        .due_date
        .map(|d| {
            d.parse().map_err(|e| {
                warn!("用户 {} 到期日格式错误：{}", auth.username, e);
                AppError::validation("到期日格式错误")
            })
        })
        .transpose()?;

    let create_req = CreateArInvoiceRequest {
        invoice_date,
        due_date,
        customer_id: req.customer_id,
        customer_name: req.customer_name,
        source_type: req.source_type,
        source_bill_id: req.source_bill_id,
        source_bill_no: req.source_bill_no,
        invoice_amount: req.invoice_amount,
        batch_no: req.batch_no,
        color_no: req.color_no,
        sales_order_no: req.sales_order_no,
    };

    let service = ArInvoiceService::new(state.db.clone());
    let invoice = service.create(create_req, auth.user_id).await?;
    info!(
        "用户 {} 创建应收单成功：{}",
        auth.username, invoice.invoice_no
    );

    Ok(Json(ApiResponse::success_with_message(
        invoice,
        "应收单创建成功",
    )))
}

use crate::services::ar_invoice_service::UpdateArInvoiceRequest;
use axum::extract::Path;
use serde_json::Value as JsonValue;

/// 获取应收发票详情
pub async fn get_ar_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let invoice = service.get_by_id(id, Some(&data_scope_ctx)).await?;
    Ok(Json(ApiResponse::success(
        serde_json::to_value(invoice).map_err(|_| AppError::internal("序列化失败"))?,
    )))
}

/// 更新应收发票
pub async fn update_ar_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateArInvoiceRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_by_id + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    let invoice = service.update(id, req, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice).map_err(|_| AppError::internal("序列化失败"))?,
        "应收发票更新成功",
    )))
}

/// 删除应收发票
pub async fn delete_ar_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_by_id + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    service.delete(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "应收发票删除成功",
    )))
}

/// 审批应收发票
pub async fn approve_ar_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    let invoice = service.approve(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice).map_err(|_| AppError::internal("序列化失败"))?,
        "应收发票审批成功",
    )))
}

/// 取消应收发票
pub async fn cancel_ar_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelReason>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    let invoice = service.cancel(id, req.reason, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice).map_err(|_| AppError::internal("序列化失败"))?,
        "应收发票取消成功",
    )))
}

/// GET /api/v1/erp/ar/invoices/export - 导出应收发票列表（带水印 + 异步审计日志）
///
/// V15 P0-S12 修复（Batch 475e）：导出接入后端
/// - 注入水印（operator/exported_at/extra 含条数）
/// - 异步审计日志（OperationType::Export）
/// - 直接调 service.get_list 取全量数据（page=1/page_size=10000）
pub async fn export_ar_invoices(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ArInvoiceQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = ArInvoiceService::new(state.db.clone());

    let (invoices, _total) = service
        .get_list(query.customer_id, query.status, 1, 10000)
        .await?;
    let row_count = invoices.len();

    let invoices_json: Vec<serde_json::Value> = invoices
        .into_iter()
        .map(|i| serde_json::to_value(i).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    let headers: Vec<String> = vec![
        "ID".to_string(),
        "发票编号".to_string(),
        "客户ID".to_string(),
        "客户名称".to_string(),
        "发票日期".to_string(),
        "到期日".to_string(),
        "发票金额".to_string(),
        "税额".to_string(),
        "源单类型".to_string(),
        "源单编号".to_string(),
        "状态".to_string(),
        "创建时间".to_string(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(invoices_json.len());
    for i in invoices_json {
        let obj = i.as_object().ok_or_else(|| {
            AppError::internal("应收发票序列化失败：期望 JSON 对象")
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
            get_str("customer_id"),
            get_str("customer_name"),
            get_str("invoice_date"),
            get_str("due_date"),
            get_str("invoice_amount"),
            get_str("tax_amount"),
            get_str("source_type"),
            get_str("source_bill_no"),
            get_str("status"),
            get_str("created_at"),
        ]);
    }

    let table = XlsxTable {
        sheet_name: "应收发票".to_string(),
        headers,
        rows,
    };

    let filename = format!(
        "ar_invoices_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("ar_invoice".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出应收发票列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/ar/invoices/export".to_string()),
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
        extra: Some(format!("应收发票导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
