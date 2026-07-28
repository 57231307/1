//! 色卡发放管理 Handler（V15 P0-F04 创建）
//!
//! 替代旧 borrow.rs（已废弃）
//! 实现 7 个 HTTP 端点：发放/归还/遗失/损坏/取消/列表/详情

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::color_card_issue;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::color_card_issue_service::{
    ColorCardIssueService, IssueError, IssueParams, ListIssuesQuery,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};

// ==================== DTO 定义 ====================

/// 发放色卡请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IssueColorCardDto {
    pub color_card_id: i64,
    pub customer_id: i64,
    pub issue_qty: Option<i32>,
    pub expected_return_date: Option<NaiveDate>,
    pub purpose: Option<String>,
    pub remark: Option<String>,
    pub dye_lot_no: Option<String>,
}

/// 归还色卡请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ReturnColorCardDto {
    pub actual_return_date: Option<NaiveDate>,
    pub remark: Option<String>,
}

/// 登记遗失请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MarkLostDto {
    pub compensation_amount: Decimal,
    pub remark: Option<String>,
}

/// 标记损坏请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MarkDamagedDto {
    pub compensation_amount: Option<Decimal>,
    pub remark: Option<String>,
}

/// 取消发放请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CancelIssueDto {
    pub remark: Option<String>,
}

/// 发放记录响应 DTO
#[derive(Debug, Serialize, Clone)]
pub struct IssueRecordInfo {
    pub id: i64,
    pub color_card_id: i64,
    pub customer_id: i64,
    pub issue_qty: i32,
    pub issued_by: i64,
    pub issued_at: DateTime<Utc>,
    pub expected_return_date: Option<NaiveDate>,
    pub actual_return_date: Option<NaiveDate>,
    pub status: String,
    pub purpose: Option<String>,
    pub remark: Option<String>,
    pub compensation_amount: Option<Decimal>,
    pub returned_by: Option<i64>,
    pub dye_lot_no: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<color_card_issue::Model> for IssueRecordInfo {
    fn from(m: color_card_issue::Model) -> Self {
        Self {
            id: m.id,
            color_card_id: m.color_card_id,
            customer_id: m.customer_id,
            issue_qty: m.issue_qty,
            issued_by: m.issued_by,
            issued_at: m.issued_at,
            expected_return_date: m.expected_return_date,
            actual_return_date: m.actual_return_date,
            status: m.status,
            purpose: m.purpose,
            remark: m.remark,
            compensation_amount: m.compensation_amount,
            returned_by: m.returned_by,
            dye_lot_no: m.dye_lot_no,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// 通用分页响应
#[derive(Debug, Serialize, Clone)]
pub struct IssuePagedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

// ==================== 错误转换 ====================

/// IssueError → AppError
pub fn issue_err(e: IssueError) -> AppError {
    match e {
        IssueError::ColorCardNotFound => AppError::not_found("色卡不存在"),
        IssueError::CustomerNotFound => AppError::not_found("客户不存在"),
        IssueError::RecordNotFound => AppError::not_found("发放记录不存在"),
        IssueError::InvalidState(msg) => AppError::business(msg),
        IssueError::Validation(msg) => AppError::validation(msg),
        IssueError::GateCheckFailed(msg) => AppError::business(msg),
        IssueError::Database(e) => AppError::database(e.to_string()),
    }
}

// ==================== Handler 端点 ====================

/// POST /api/v1/erp/color-cards/issues - 发放色卡
pub async fn issue_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<IssueColorCardDto>,
) -> Result<Json<ApiResponse<IssueRecordInfo>>, AppError> {
    let user_id = auth.user_id as i64;
    let service = ColorCardIssueService::from_state(&state);

    let params = IssueParams {
        color_card_id: dto.color_card_id,
        customer_id: dto.customer_id,
        issued_by: user_id,
        issue_qty: dto.issue_qty.unwrap_or(1),
        expected_return_date: dto.expected_return_date,
        purpose: dto.purpose,
        remark: dto.remark,
        dye_lot_no: dto.dye_lot_no,
        sales_order_id: None,
    };

    let record = service.issue(params).await.map_err(issue_err)?;

    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/color-cards/issues/:record_id/return - 归还色卡
pub async fn return_issue(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<ReturnColorCardDto>,
) -> Result<Json<ApiResponse<IssueRecordInfo>>, AppError> {
    let user_id = auth.user_id as i64;
    let service = ColorCardIssueService::from_state(&state);

    let record = service
        .return_card(record_id, user_id, dto.actual_return_date, dto.remark)
        .await
        .map_err(issue_err)?;

    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/color-cards/issues/:record_id/lost - 登记遗失
pub async fn mark_issue_lost(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<MarkLostDto>,
) -> Result<Json<ApiResponse<IssueRecordInfo>>, AppError> {
    let service = ColorCardIssueService::from_state(&state);

    let record = service
        .mark_lost(record_id, dto.compensation_amount, dto.remark)
        .await
        .map_err(issue_err)?;

    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/color-cards/issues/:record_id/damaged - 标记损坏
pub async fn mark_issue_damaged(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<MarkDamagedDto>,
) -> Result<Json<ApiResponse<IssueRecordInfo>>, AppError> {
    let service = ColorCardIssueService::from_state(&state);

    let record = service
        .mark_damaged(record_id, dto.compensation_amount, dto.remark)
        .await
        .map_err(issue_err)?;

    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/color-cards/issues/:record_id/cancel - 取消发放
pub async fn cancel_issue(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<CancelIssueDto>,
) -> Result<Json<ApiResponse<IssueRecordInfo>>, AppError> {
    let service = ColorCardIssueService::from_state(&state);

    let record = service
        .cancel_issue(record_id, dto.remark)
        .await
        .map_err(issue_err)?;

    Ok(Json(ApiResponse::success(record.into())))
}

/// GET /api/v1/erp/color-cards/issues - 发放记录列表
pub async fn list_issues(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<Json<ApiResponse<IssuePagedResponse<IssueRecordInfo>>>, AppError> {
    let service = ColorCardIssueService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let (items, total) = service.list_records(query).await.map_err(issue_err)?;

    let infos: Vec<IssueRecordInfo> = items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(IssuePagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/color-cards/issues/:record_id - 发放记录详情
pub async fn get_issue(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
) -> Result<Json<ApiResponse<IssueRecordInfo>>, AppError> {
    let service = ColorCardIssueService::from_state(&state);
    let record = service.get_by_id(record_id).await.map_err(issue_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// GET /api/v1/erp/color-cards/issues/export - 导出色卡发放记录（xlsx）
pub async fn export_issue_records(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ListIssuesQuery>,
) -> Result<axum::response::Response, AppError> {
    let mut q =
        color_card_issue::Entity::find().filter(color_card_issue::Column::IsDeleted.eq(false));
    if let Some(v) = query.color_card_id {
        q = q.filter(color_card_issue::Column::ColorCardId.eq(v));
    }
    if let Some(v) = query.customer_id {
        q = q.filter(color_card_issue::Column::CustomerId.eq(v));
    }
    if let Some(v) = query.status.clone() {
        q = q.filter(color_card_issue::Column::Status.eq(v));
    }
    if let Some(v) = query.from_date {
        q = q.filter(color_card_issue::Column::IssuedAt.gte(v));
    }
    if let Some(v) = query.to_date {
        q = q.filter(color_card_issue::Column::IssuedAt.lte(v));
    }
    q = q.order_by_desc(color_card_issue::Column::IssuedAt);

    let records = q.all(&*state.db).await?;
    let table = build_issue_records_xlsx_table(&records);
    let row_count = records.len();

    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("color_card_issue".to_string()),
        resource_id: None,
        resource_name: Some("color_card_issues_export.xlsx".to_string()),
        description: Some(format!(
            "用户 {} 导出色卡发放记录（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/color-cards/issues/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "color_card_id_filter": query.color_card_id,
            "customer_id_filter": query.customer_id,
            "status_filter": query.status,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    build_xlsx_response(&table, "color_card_issues_export")
}

/// 构造色卡发放记录导出表格
fn build_issue_records_xlsx_table(records: &[color_card_issue::Model]) -> XlsxTable {
    XlsxTable {
        sheet_name: "色卡发放记录".to_string(),
        headers: vec![
            "ID".to_string(),
            "色卡ID".to_string(),
            "客户ID".to_string(),
            "发放数量".to_string(),
            "发放人ID".to_string(),
            "发放时间".to_string(),
            "预计归还日期".to_string(),
            "实际归还日期".to_string(),
            "状态".to_string(),
            "用途".to_string(),
            "缸号".to_string(),
            "备注".to_string(),
            "赔偿金额".to_string(),
            "归还人ID".to_string(),
            "创建时间".to_string(),
        ],
        rows: records
            .iter()
            .map(|r| {
                vec![
                    r.id.to_string(),
                    r.color_card_id.to_string(),
                    r.customer_id.to_string(),
                    r.issue_qty.to_string(),
                    r.issued_by.to_string(),
                    r.issued_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    r.expected_return_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default(),
                    r.actual_return_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default(),
                    r.status.clone(),
                    r.purpose.clone().unwrap_or_default(),
                    r.dye_lot_no.clone().unwrap_or_default(),
                    r.remark.clone().unwrap_or_default(),
                    r.compensation_amount
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    r.returned_by.map(|v| v.to_string()).unwrap_or_default(),
                    r.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect(),
    }
}
