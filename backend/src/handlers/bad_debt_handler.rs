//! 坏账管理 Handler（V15 P0-B01/B02 Batch 481 创建）
//!
//! 实现 12 个 HTTP 端点：
//!   **B01 坏账准备计提**：
//!   - POST   /run-provision              期末触发计提
//!   - GET    /                            计提记录列表
//!   - GET    /:id                         计提记录详情
//!   - POST   /:id/confirm                确认计提（draft → confirmed）
//!   - POST   /:id/reverse                转回计提（confirmed → reversed）
//!
//!   **B02 坏账核销审批**：
//!   - POST   /writeoffs                  申请核销（创建 pending）
//!   - GET    /writeoffs                   核销申请列表
//!   - GET    /writeoffs/:id              核销申请详情
//!   - POST   /writeoffs/:id/finance-approve    一级审批（pending → finance_approved）
//!   - POST   /writeoffs/:id/general-manager-approve  二级审批（finance_approved → approved）
//!   - POST   /writeoffs/:id/reject            拒绝核销
//!   - POST   /writeoffs/:id/cancel           申请人取消

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::models::bad_debt_dto::{
    ApproveWriteoffRequest, CancelWriteoffRequest, CreateWriteoffRequest, ListProvisionQuery,
    ListWriteoffQuery, RejectWriteoffRequest, ReverseProvisionRequest, RunProvisionRequest,
};
use crate::models::bad_debt_provision;
use crate::models::bad_debt_writeoff;
use crate::services::bad_debt_service::{BadDebtError, BadDebtService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ==================== 响应 DTO ====================

/// 坏账准备计提响应
#[derive(Debug, Serialize, Clone)]
pub struct ProvisionInfo {
    pub id: i64,
    pub customer_id: i64,
    pub customer_name: Option<String>,
    pub period_year: i32,
    pub period_month: i32,
    pub aging_bucket: String,
    pub base_amount: Decimal,
    pub provision_rate: Decimal,
    pub provision_amount: Decimal,
    pub voucher_id: Option<i64>,
    pub status: String,
    pub created_by: i32,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub reversed_at: Option<DateTime<Utc>>,
    pub reverse_voucher_id: Option<i64>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<bad_debt_provision::Model> for ProvisionInfo {
    fn from(m: bad_debt_provision::Model) -> Self {
        Self {
            id: m.id,
            customer_id: m.customer_id,
            customer_name: m.customer_name,
            period_year: m.period_year,
            period_month: m.period_month,
            aging_bucket: m.aging_bucket,
            base_amount: m.base_amount,
            provision_rate: m.provision_rate,
            provision_amount: m.provision_amount,
            voucher_id: m.voucher_id,
            status: m.status,
            created_by: m.created_by,
            confirmed_at: m.confirmed_at,
            reversed_at: m.reversed_at,
            reverse_voucher_id: m.reverse_voucher_id,
            remark: m.remark,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// 坏账核销申请响应
#[derive(Debug, Serialize, Clone)]
pub struct WriteoffInfo {
    pub id: i64,
    pub customer_id: i64,
    pub ar_invoice_id: i32,
    pub writeoff_amount: Decimal,
    pub reason: String,
    pub applicant_user_id: i32,
    pub applicant_username: String,
    pub applicant_at: DateTime<Utc>,
    pub approval_level: i16,
    pub approval_status: String,
    pub finance_manager_id: Option<i32>,
    pub finance_manager_at: Option<DateTime<Utc>>,
    pub finance_manager_comment: Option<String>,
    pub general_manager_id: Option<i32>,
    pub general_manager_at: Option<DateTime<Utc>>,
    pub general_manager_comment: Option<String>,
    pub voucher_id: Option<i64>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<bad_debt_writeoff::Model> for WriteoffInfo {
    fn from(m: bad_debt_writeoff::Model) -> Self {
        Self {
            id: m.id,
            customer_id: m.customer_id,
            ar_invoice_id: m.ar_invoice_id,
            writeoff_amount: m.writeoff_amount,
            reason: m.reason,
            applicant_user_id: m.applicant_user_id,
            applicant_username: m.applicant_username,
            applicant_at: m.applicant_at,
            approval_level: m.approval_level,
            approval_status: m.approval_status,
            finance_manager_id: m.finance_manager_id,
            finance_manager_at: m.finance_manager_at,
            finance_manager_comment: m.finance_manager_comment,
            general_manager_id: m.general_manager_id,
            general_manager_at: m.general_manager_at,
            general_manager_comment: m.general_manager_comment,
            voucher_id: m.voucher_id,
            completed_at: m.completed_at,
            cancelled_at: m.cancelled_at,
            cancel_reason: m.cancel_reason,
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

/// 批量计提响应
#[derive(Debug, Serialize, Clone)]
pub struct RunProvisionResponse {
    pub created: Vec<ProvisionInfo>,
    pub created_count: usize,
}

/// BadDebtError → AppError
pub fn bad_debt_err(e: BadDebtError) -> AppError {
    match e {
        BadDebtError::ProvisionNotFound => AppError::not_found("坏账准备记录不存在"),
        BadDebtError::WriteoffNotFound => AppError::not_found("坏账核销申请不存在"),
        BadDebtError::ArInvoiceNotFound => AppError::not_found("应收单不存在"),
        BadDebtError::InvalidState { current, expected } => AppError::business(format!(
            "当前状态 {} 不允许此操作（期望 {}）",
            current, expected
        )),
        BadDebtError::WriteoffAmountExceeds { requested, unpaid } => AppError::business(format!(
            "核销金额超过应收单未收金额：申请 {}，未收 {}",
            requested, unpaid
        )),
        BadDebtError::SelfApprovalForbidden => {
            AppError::permission_denied("不能审批自己提交的核销申请")
        }
        BadDebtError::NotApplicant => AppError::permission_denied("只有申请人可以取消核销申请"),
        BadDebtError::Validation(msg) => AppError::validation(msg),
        BadDebtError::Database(e) => AppError::database(e.to_string()),
        // paginate_with_total 返回的 AppError 直接透传
        BadDebtError::App(e) => e,
    }
}

// ==================== B01 坏账准备计提端点 ====================

/// POST /api/v1/erp/bad-debts/run-provision - 期末触发计提
pub async fn run_provision(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<RunProvisionRequest>,
) -> Result<Json<ApiResponse<RunProvisionResponse>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let created = service.run_monthly_provision(req, auth.user_id).await.map_err(bad_debt_err)?;
    let count = created.len();
    let infos: Vec<ProvisionInfo> = created.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(RunProvisionResponse {
        created: infos,
        created_count: count,
    })))
}

/// GET /api/v1/erp/bad-debts - 计提记录列表
pub async fn list_provisions(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListProvisionQuery>,
) -> Result<Json<ApiResponse<PagedResponse<ProvisionInfo>>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let (items, total) = service.list_provisions(query).await.map_err(bad_debt_err)?;
    let infos: Vec<ProvisionInfo> = items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/bad-debts/:id - 计提记录详情
pub async fn get_provision(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ProvisionInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service.get_provision(id).await.map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bad-debts/:id/confirm - 确认计提（draft → confirmed）
pub async fn confirm_provision(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ProvisionInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service.confirm_provision(id).await.map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bad-debts/:id/reverse - 转回计提（confirmed → reversed）
pub async fn reverse_provision(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ReverseProvisionRequest>,
) -> Result<Json<ApiResponse<ProvisionInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service
        .reverse_provision(id, req)
        .await
        .map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

// ==================== B02 坏账核销审批端点 ====================

/// POST /api/v1/erp/bad-debts/writeoffs - 申请核销
pub async fn create_writeoff(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<CreateWriteoffRequest>,
) -> Result<Json<ApiResponse<WriteoffInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service
        .create_writeoff(req, auth.user_id, auth.username)
        .await
        .map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// GET /api/v1/erp/bad-debts/writeoffs - 核销申请列表
pub async fn list_writeoffs(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListWriteoffQuery>,
) -> Result<Json<ApiResponse<PagedResponse<WriteoffInfo>>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let (items, total) = service.list_writeoffs(query).await.map_err(bad_debt_err)?;
    let infos: Vec<WriteoffInfo> = items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/bad-debts/writeoffs/:id - 核销申请详情
pub async fn get_writeoff(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<WriteoffInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service.get_writeoff(id).await.map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bad-debts/writeoffs/:id/finance-approve - 一级审批（pending → finance_approved）
pub async fn finance_approve(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ApproveWriteoffRequest>,
) -> Result<Json<ApiResponse<WriteoffInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service
        .finance_approve(id, auth.user_id, req)
        .await
        .map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bad-debts/writeoffs/:id/general-manager-approve - 二级审批（finance_approved → approved）
pub async fn general_manager_approve(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ApproveWriteoffRequest>,
) -> Result<Json<ApiResponse<WriteoffInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service
        .general_manager_approve(id, auth.user_id, req)
        .await
        .map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bad-debts/writeoffs/:id/reject - 拒绝核销
pub async fn reject_writeoff(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<RejectWriteoffRequest>,
) -> Result<Json<ApiResponse<WriteoffInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service
        .reject(id, auth.user_id, req)
        .await
        .map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bad-debts/writeoffs/:id/cancel - 申请人取消
pub async fn cancel_writeoff(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<CancelWriteoffRequest>,
) -> Result<Json<ApiResponse<WriteoffInfo>>, AppError> {
    let service = BadDebtService::from_state(&state);
    let record = service
        .cancel(id, auth.user_id, req)
        .await
        .map_err(bad_debt_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}
