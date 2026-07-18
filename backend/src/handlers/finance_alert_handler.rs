//! 财务预警 Handler（V15 P0-B04 Batch 481 创建）
//!
//! 实现 7 个 HTTP 端点：
//!   - POST   /trigger-scan            触发扫描生成预警
//!   - POST   /                         手动创建预警
//!   - GET    /                         预警列表
//!   - GET    /:id                      预警详情
//!   - POST   /:id/acknowledge         确认预警（active → acknowledged）
//!   - POST   /:id/resolve             解决预警（acknowledged → resolved）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::finance_alert;
use crate::models::finance_alert_dto::{
    AcknowledgeAlertRequest, CreateAlertRequest, ListAlertQuery, ResolveAlertRequest,
    TriggerScanRequest,
};
use crate::services::finance_alert_service::{FinanceAlertError, FinanceAlertService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ==================== 响应 DTO ====================

/// 财务预警响应
#[derive(Debug, Serialize, Clone)]
pub struct AlertInfo {
    pub id: i64,
    pub alert_no: String,
    pub alert_type: String,
    pub alert_level: String,
    pub title: String,
    pub content: String,
    pub target_module: Option<String>,
    pub target_id: Option<i64>,
    pub threshold_value: Option<Decimal>,
    pub actual_value: Option<Decimal>,
    pub value_unit: Option<String>,
    pub triggered_at: DateTime<Utc>,
    pub triggered_by: Option<i32>,
    pub status: String,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<i32>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<i32>,
    pub resolve_note: Option<String>,
    pub expired_at: Option<DateTime<Utc>>,
    pub notification_id: Option<i32>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<finance_alert::Model> for AlertInfo {
    fn from(m: finance_alert::Model) -> Self {
        Self {
            id: m.id,
            alert_no: m.alert_no,
            alert_type: m.alert_type,
            alert_level: m.alert_level,
            title: m.title,
            content: m.content,
            target_module: m.target_module,
            target_id: m.target_id,
            threshold_value: m.threshold_value,
            actual_value: m.actual_value,
            value_unit: m.value_unit,
            triggered_at: m.triggered_at,
            triggered_by: m.triggered_by,
            status: m.status,
            acknowledged_at: m.acknowledged_at,
            acknowledged_by: m.acknowledged_by,
            resolved_at: m.resolved_at,
            resolved_by: m.resolved_by,
            resolve_note: m.resolve_note,
            expired_at: m.expired_at,
            notification_id: m.notification_id,
            remark: m.remark,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// 分页响应
#[derive(Debug, Serialize, Clone)]
pub struct PagedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 批量扫描响应
#[derive(Debug, Serialize, Clone)]
pub struct TriggerScanResponse {
    pub created: Vec<AlertInfo>,
    pub created_count: usize,
}

/// FinanceAlertError → AppError
pub fn finance_alert_err(e: FinanceAlertError) -> AppError {
    match e {
        FinanceAlertError::NotFound => AppError::not_found("财务预警不存在"),
        FinanceAlertError::InvalidState { current, expected } => AppError::business(format!(
            "当前状态 {} 不允许此操作（期望 {}）",
            current, expected
        )),
        FinanceAlertError::Validation(msg) => AppError::validation(msg),
        FinanceAlertError::Database(e) => AppError::database(e.to_string()),
        // paginate_with_total 返回的 AppError 直接透传
        FinanceAlertError::App(e) => e,
    }
}

// ==================== Handler 端点 ====================

/// POST /api/v1/erp/finance-alerts/trigger-scan - 触发扫描生成预警
pub async fn trigger_scan(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<TriggerScanRequest>,
) -> Result<Json<ApiResponse<TriggerScanResponse>>, AppError> {
    let service = FinanceAlertService::from_state(&state);
    let created = service
        .trigger_scan(req, Some(auth.user_id))
        .await
        .map_err(finance_alert_err)?;
    let count = created.len();
    let infos: Vec<AlertInfo> = created.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(TriggerScanResponse {
        created: infos,
        created_count: count,
    })))
}

/// POST /api/v1/erp/finance-alerts - 手动创建预警
pub async fn create_alert(
    _auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<CreateAlertRequest>,
) -> Result<Json<ApiResponse<AlertInfo>>, AppError> {
    let service = FinanceAlertService::from_state(&state);
    let record = service
        .create_alert(req)
        .await
        .map_err(finance_alert_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// GET /api/v1/erp/finance-alerts - 预警列表
pub async fn list_alerts(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListAlertQuery>,
) -> Result<Json<ApiResponse<PagedResponse<AlertInfo>>>, AppError> {
    let service = FinanceAlertService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let (items, total) = service
        .list_alerts(query)
        .await
        .map_err(finance_alert_err)?;
    let infos: Vec<AlertInfo> = items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/finance-alerts/:id - 预警详情
pub async fn get_alert(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<AlertInfo>>, AppError> {
    let service = FinanceAlertService::from_state(&state);
    let record = service
        .get_alert(id)
        .await
        .map_err(finance_alert_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/finance-alerts/:id/acknowledge - 确认预警（active → acknowledged）
pub async fn acknowledge(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<AcknowledgeAlertRequest>,
) -> Result<Json<ApiResponse<AlertInfo>>, AppError> {
    let service = FinanceAlertService::from_state(&state);
    let record = service
        .acknowledge(id, auth.user_id, req)
        .await
        .map_err(finance_alert_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/finance-alerts/:id/resolve - 解决预警（acknowledged → resolved）
pub async fn resolve(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ResolveAlertRequest>,
) -> Result<Json<ApiResponse<AlertInfo>>, AppError> {
    let service = FinanceAlertService::from_state(&state);
    let record = service
        .resolve(id, auth.user_id, req)
        .await
        .map_err(finance_alert_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}
