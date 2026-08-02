//! 色卡发放报表 / 统计 / 库存预警 / 成本核算 Handler
//!
//! V15 P2 类九：10.3-3（5 类报表）、10.3-4（成本核算）、10.5-2（库存预警）、10.5-3（发放统计）
//! 端点全部接入 ColorCardIssueReportService / ColorCardCostAccountingService /
//! ColorCardInventoryWarningService / ColorCardIssueStatisticsService 真实实现。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::color_card_cost_accounting_service::ColorCardCostAccountingService;
use crate::services::color_card_inventory_warning_service::{
    ColorCardInventoryWarningService, WarningLevel,
};
use crate::services::color_card_issue_report_service::{ColorCardIssueReportService, ReportParams};
use crate::services::color_card_issue_statistics_service::{
    ColorCardIssueStatisticsService, DailyStats,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};

use super::issue::require_issue_permission;

// ==================== DTO 定义 ====================

/// 报表查询参数（Query string）
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReportQuery {
    pub customer_id: Option<i32>,
    pub color_card_id: Option<i32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

impl From<ReportQuery> for ReportParams {
    fn from(q: ReportQuery) -> Self {
        Self {
            customer_id: q.customer_id,
            color_card_id: q.color_card_id,
            start_date: q.start_date,
            end_date: q.end_date,
            page: q.page,
            page_size: q.page_size,
        }
    }
}

/// 日统计查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct DailyStatsQuery {
    pub date: NaiveDate,
}

/// 成本核算结果 DTO
#[derive(Debug, Serialize)]
pub struct CostAmountResult {
    pub amount: rust_decimal::Decimal,
    pub currency: String,
}

impl CostAmountResult {
    fn of(amount: rust_decimal::Decimal) -> Self {
        Self {
            amount,
            currency: "CNY".to_string(),
        }
    }
}

// ==================== 报表端点（10.3-3） ====================

/// GET /api/v1/erp/color-cards/reports/issue-detail - 发放明细报表
pub async fn issue_detail_report(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ReportQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardIssueReportService::new(state.db.clone());
    let rows = svc.issue_detail_report(query.into()).await?;
    Ok(Json(ApiResponse::success(rows)))
}

/// GET /api/v1/erp/color-cards/reports/issue-summary - 发放汇总报表
pub async fn issue_summary_report(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ReportQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardIssueReportService::new(state.db.clone());
    let rows = svc.issue_summary_report(query.into()).await?;
    Ok(Json(ApiResponse::success(rows)))
}

/// GET /api/v1/erp/color-cards/reports/customer-ledger/:customer_id - 客户色卡台账
pub async fn customer_color_card_ledger(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(customer_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardIssueReportService::new(state.db.clone());
    let rows = svc.customer_color_card_ledger(customer_id).await?;
    Ok(Json(ApiResponse::success(rows)))
}

/// GET /api/v1/erp/color-cards/reports/expired-unused - 过期未使用色卡报表
pub async fn expired_unused_report(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardIssueReportService::new(state.db.clone());
    let rows = svc.expired_unused_report().await?;
    Ok(Json(ApiResponse::success(rows)))
}

/// GET /api/v1/erp/color-cards/reports/order-related/:sales_order_id - 订单关联报表
pub async fn order_related_report(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(sales_order_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardIssueReportService::new(state.db.clone());
    let rows = svc.order_related_report(sales_order_id).await?;
    Ok(Json(ApiResponse::success(rows)))
}

/// GET /api/v1/erp/color-cards/reports/issue-detail/export - 发放明细报表导出（xlsx）
pub async fn export_issue_detail_report(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ReportQuery>,
) -> Result<axum::response::Response, AppError> {
    require_issue_permission(&state, &auth, "export").await?;
    let svc = ColorCardIssueReportService::new(state.db.clone());
    let rows = svc.issue_detail_report(query.into()).await?;

    let headers = vec![
        "issue_id",
        "color_card_id",
        "card_no",
        "card_name",
        "customer_id",
        "customer_name",
        "issue_qty",
        "issued_at",
        "expected_return_date",
        "actual_return_date",
        "status",
        "purpose",
        "remark",
        "compensation_amount",
        "dye_lot_no",
        "sales_order_id",
    ];
    let rows_str: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            headers
                .iter()
                .map(|h| {
                    r.get(*h)
                        .map(|x| {
                            if x.is_null() {
                                String::new()
                            } else if let Some(s) = x.as_str() {
                                s.to_string()
                            } else {
                                x.to_string()
                            }
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .collect();

    let table = XlsxTable {
        sheet_name: "发放明细".to_string(),
        headers: headers.into_iter().map(|s| s.to_string()).collect(),
        rows: rows_str,
    };
    let filename = "color-card-issue-detail-report";

    // 导出审计日志（best-effort）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("color_card_issue_report".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出色卡发放明细报表（共 {} 条）",
            auth.username,
            rows.len()
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/color-cards/reports/issue-detail/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({ "format": "xlsx", "total": rows.len() })),
    };
    let svc_audit = Arc::new(AuditLogService::new(state.db.clone()));
    svc_audit.record_async(event, None);

    build_xlsx_response(&table, filename)
}

// ==================== 库存预警端点（10.5-2） ====================

/// GET /api/v1/erp/color-cards/warnings - 检查全部色卡库存预警
pub async fn check_all_warnings(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::color_card_inventory_warning_service::WarningItem>>>,
    AppError,
> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardInventoryWarningService::new(state.db.clone());
    let warnings = svc.check_all_warnings().await?;
    Ok(Json(ApiResponse::success(warnings)))
}

/// GET /api/v1/erp/color-cards/warnings/:color_card_id - 检查单个色卡库存预警
pub async fn check_single_warning(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(color_card_id): Path<i32>,
) -> Result<Json<ApiResponse<WarningLevel>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardInventoryWarningService::new(state.db.clone());
    let level = svc.check_single_warning(color_card_id).await?;
    Ok(Json(ApiResponse::success(level)))
}

// ==================== 成本核算端点（10.3-4） ====================

/// GET /api/v1/erp/color-cards/cost/production/:color_card_id - 制作成本归集
pub async fn collect_production_cost(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(color_card_id): Path<i32>,
) -> Result<Json<ApiResponse<CostAmountResult>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardCostAccountingService::new(state.db.clone());
    let amount = svc.collect_production_cost(color_card_id).await?;
    Ok(Json(ApiResponse::success(CostAmountResult::of(amount))))
}

/// GET /api/v1/erp/color-cards/cost/issue/:record_id/transfer - 发放成本结转
pub async fn transfer_issue_cost(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i32>,
) -> Result<Json<ApiResponse<CostAmountResult>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardCostAccountingService::new(state.db.clone());
    let amount = svc.transfer_issue_cost(record_id).await?;
    Ok(Json(ApiResponse::success(CostAmountResult::of(amount))))
}

/// POST /api/v1/erp/color-cards/cost/issue/:record_id/restore - 取消发放恢复成本
pub async fn restore_cost_on_cancel(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardCostAccountingService::new(state.db.clone());
    svc.restore_cost_on_cancel(record_id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "issue_id": record_id,
        "restored": true,
    }))))
}

/// GET /api/v1/erp/color-cards/cost/issue/:record_id/expiry-loss - 过期损失核算
pub async fn calculate_expiry_loss(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i32>,
) -> Result<Json<ApiResponse<CostAmountResult>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardCostAccountingService::new(state.db.clone());
    let amount = svc.calculate_expiry_loss(record_id).await?;
    Ok(Json(ApiResponse::success(CostAmountResult::of(amount))))
}

// ==================== 发放统计端点（10.5-3） ====================

/// GET /api/v1/erp/color-cards/statistics/daily?date=YYYY-MM-DD - 生成日统计
pub async fn generate_daily_stats(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<DailyStatsQuery>,
) -> Result<Json<ApiResponse<DailyStats>>, AppError> {
    require_issue_permission(&state, &auth, "read").await?;
    let svc = ColorCardIssueStatisticsService::new(state.db.clone());
    let stats = svc.generate_daily_stats(query.date).await?;
    Ok(Json(ApiResponse::success(stats)))
}
