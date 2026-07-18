//! 大货批色审批 Handler（V15 P0-F15/F16/F17 创建）
//!
//! 实现 9 个 HTTP 端点：
//!   - 列表/详情（GET）
//!   - 创建批色记录（POST）
//!   - 剪大货样（POST /:id/cut-sample）— P0-F16
//!   - 发送客户批色（POST /:id/send-to-customer）
//!   - 客户批色确认 通过/拒绝/返工（POST /:id/approve|reject|rework）— P0-F17
//!   - 降级（POST /:id/downgrade）
//!   - 报废（POST /:id/scrap）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::models::bulk_color_approval;
use crate::services::bulk_color_approval_service::{
    BulkColorApprovalError, BulkColorApprovalService, CreateBulkColorApprovalParams,
    CutSampleParams, ListBulkColorApprovalQuery,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ==================== DTO 定义 ====================

/// 创建批色记录请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateBulkColorApprovalDto {
    pub sales_order_id: i32,
    pub dye_batch_id: i32,
    pub customer_id: i64,
    pub production_order_id: Option<i32>,
    pub product_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub sample_type: Option<String>,
    pub remark: Option<String>,
}

/// 剪大货样请求 DTO（P0-F16）
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CutSampleDto {
    pub sample_length_m: Decimal,
    pub sample_piece_id: Option<i64>,
    pub attachment_url: Option<String>,
    pub delta_e_value: Option<Decimal>,
}

/// 客户批色确认请求 DTO（P0-F17）
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CustomerApproveDto {
    pub feedback: Option<String>,
    pub delta_e_value: Option<Decimal>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomerRejectDto {
    pub reject_reason: String,
    pub feedback: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomerReworkDto {
    pub reject_reason: String,
    pub feedback: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DowngradeDto {
    pub reject_reason: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScrapDto {
    pub reject_reason: String,
}

/// 批色记录响应 DTO
#[derive(Debug, Serialize, Clone)]
pub struct BulkColorApprovalInfo {
    pub id: i64,
    pub sales_order_id: i32,
    pub dye_batch_id: i32,
    pub customer_id: i64,
    pub production_order_id: Option<i32>,
    pub product_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub sample_type: String,
    pub sample_piece_id: Option<i64>,
    pub sample_length_m: Option<Decimal>,
    pub approval_status: String,
    pub approver_id: Option<i32>,
    pub approval_date: Option<DateTime<Utc>>,
    pub sent_to_customer_at: Option<DateTime<Utc>>,
    pub customer_feedback: Option<String>,
    pub delta_e_value: Option<Decimal>,
    pub reject_reason: Option<String>,
    pub delivery_blocking: bool,
    pub attachment_url: Option<String>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<bulk_color_approval::Model> for BulkColorApprovalInfo {
    fn from(m: bulk_color_approval::Model) -> Self {
        Self {
            id: m.id,
            sales_order_id: m.sales_order_id,
            dye_batch_id: m.dye_batch_id,
            customer_id: m.customer_id,
            production_order_id: m.production_order_id,
            product_id: m.product_id,
            color_no: m.color_no,
            dye_lot_no: m.dye_lot_no,
            batch_no: m.batch_no,
            sample_type: m.sample_type,
            sample_piece_id: m.sample_piece_id,
            sample_length_m: m.sample_length_m,
            approval_status: m.approval_status,
            approver_id: m.approver_id,
            approval_date: m.approval_date,
            sent_to_customer_at: m.sent_to_customer_at,
            customer_feedback: m.customer_feedback,
            delta_e_value: m.delta_e_value,
            reject_reason: m.reject_reason,
            delivery_blocking: m.delivery_blocking,
            attachment_url: m.attachment_url,
            remark: m.remark,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// 通用分页响应
#[derive(Debug, Serialize, Clone)]
pub struct BulkColorApprovalPagedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

// ==================== 错误转换 ====================

/// BulkColorApprovalError → AppError
pub fn bca_err(e: BulkColorApprovalError) -> AppError {
    match e {
        BulkColorApprovalError::NotFound => AppError::not_found("批色记录不存在"),
        BulkColorApprovalError::SalesOrderNotFound => AppError::not_found("销售订单不存在"),
        BulkColorApprovalError::DyeBatchNotFound => AppError::not_found("染色批次不存在"),
        BulkColorApprovalError::CustomerNotFound => AppError::not_found("客户不存在"),
        BulkColorApprovalError::InvalidState(msg) => AppError::business(msg),
        BulkColorApprovalError::Validation(msg) => AppError::validation(msg),
        BulkColorApprovalError::Database(e) => AppError::database(e.to_string()),
    }
}

// ==================== Handler 端点 ====================

/// POST /api/v1/erp/bulk-color-approvals - 创建批色记录
pub async fn create_bulk_color_approval(
    _auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateBulkColorApprovalDto>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);

    let params = CreateBulkColorApprovalParams {
        sales_order_id: dto.sales_order_id,
        dye_batch_id: dto.dye_batch_id,
        customer_id: dto.customer_id,
        production_order_id: dto.production_order_id,
        product_id: dto.product_id,
        color_no: dto.color_no,
        dye_lot_no: dto.dye_lot_no,
        batch_no: dto.batch_no,
        sample_type: dto.sample_type,
        remark: dto.remark,
    };

    let record = service.create(params).await.map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// GET /api/v1/erp/bulk-color-approvals - 批色记录列表
pub async fn list_bulk_color_approvals(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListBulkColorApprovalQuery>,
) -> Result<Json<ApiResponse<BulkColorApprovalPagedResponse<BulkColorApprovalInfo>>>, AppError>
{
    let service = BulkColorApprovalService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let (items, total) = service.list(query).await.map_err(bca_err)?;
    let infos: Vec<BulkColorApprovalInfo> = items.into_iter().map(Into::into).collect();

    Ok(Json(ApiResponse::success(
        BulkColorApprovalPagedResponse {
            items: infos,
            total,
            page,
            page_size,
        },
    )))
}

/// GET /api/v1/erp/bulk-color-approvals/:id - 批色记录详情
pub async fn get_bulk_color_approval(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);
    let record = service.get(id).await.map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bulk-color-approvals/:id/cut-sample - 剪大货样（P0-F16）
pub async fn cut_sample(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<CutSampleDto>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);
    let params = CutSampleParams {
        sample_length_m: dto.sample_length_m,
        sample_piece_id: dto.sample_piece_id,
        attachment_url: dto.attachment_url,
        delta_e_value: dto.delta_e_value,
        operator_id: auth.user_id,
    };
    let record = service.cut_sample(id, params).await.map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bulk-color-approvals/:id/send-to-customer - 发送客户批色
pub async fn send_to_customer(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);
    let record = service.send_to_customer(id).await.map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bulk-color-approvals/:id/approve - 客户批色确认通过（P0-F17）
pub async fn customer_approve(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<CustomerApproveDto>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);
    let record = service
        .customer_approve(id, auth.user_id, dto.feedback, dto.delta_e_value)
        .await
        .map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bulk-color-approvals/:id/reject - 客户批色拒绝
pub async fn customer_reject(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<CustomerRejectDto>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);
    let record = service
        .customer_reject(id, auth.user_id, dto.reject_reason, dto.feedback)
        .await
        .map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bulk-color-approvals/:id/rework - 客户批色要求返工
pub async fn customer_rework(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<CustomerReworkDto>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);
    let record = service
        .customer_rework(id, auth.user_id, dto.reject_reason, dto.feedback)
        .await
        .map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bulk-color-approvals/:id/downgrade - 降级处理
pub async fn downgrade(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<DowngradeDto>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);
    let record = service
        .downgrade(id, dto.reject_reason)
        .await
        .map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/bulk-color-approvals/:id/scrap - 报废
pub async fn scrap(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<ScrapDto>,
) -> Result<Json<ApiResponse<BulkColorApprovalInfo>>, AppError> {
    let service = BulkColorApprovalService::from_state(&state);
    let record = service.scrap(id, dto.reject_reason).await.map_err(bca_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}
