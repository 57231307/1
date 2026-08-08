//! 期末调整 Handler（V15 P2 B05-P2-10）
//!
//! 实现 6 个 HTTP 端点：
//! - POST   /                  创建期末调整（暂估/摊销/预提，draft 状态）
//! - POST   /:id/confirm       确认（生成调整凭证）
//! - POST   /:id/reverse       红字冲销（生成红字凭证，暂估类下月初冲销）
//! - POST   /:id/cancel        取消（draft → cancelled）
//! - GET    /:id               详情
//! - GET    /                  列表（按类型/期间/状态过滤分页）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;

use crate::container::AppState;
use crate::handlers::bad_debt_handler::PagedResponse;
use crate::middleware::auth_context::AuthContext;
use crate::models::period_adjustment_record::Model;
use crate::services::period_adjustment_service::{
    CreatePeriodAdjustmentRequest, PeriodAdjustmentQuery, PeriodAdjustmentService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 期末调整信息（响应前端，与 Model 字段一一对应）
#[derive(Debug, Serialize, Clone)]
pub struct PeriodAdjustmentInfo {
    pub id: i32,
    pub adjustment_no: String,
    pub adjustment_type: String,
    pub period: String,
    pub description: String,
    pub debit_subject_code: String,
    pub debit_subject_name: String,
    pub credit_subject_code: String,
    pub credit_subject_name: String,
    pub amount: rust_decimal::Decimal,
    pub source_type: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub voucher_id: Option<i32>,
    pub reverse_voucher_id: Option<i32>,
    pub status: String,
    pub confirmed_by: Option<i32>,
    pub confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reversed_by: Option<i32>,
    pub reversed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Model> for PeriodAdjustmentInfo {
    fn from(m: Model) -> Self {
        Self {
            id: m.id,
            adjustment_no: m.adjustment_no,
            adjustment_type: m.adjustment_type,
            period: m.period,
            description: m.description,
            debit_subject_code: m.debit_subject_code,
            debit_subject_name: m.debit_subject_name,
            credit_subject_code: m.credit_subject_code,
            credit_subject_name: m.credit_subject_name,
            amount: m.amount,
            source_type: m.source_type,
            source_bill_id: m.source_bill_id,
            source_bill_no: m.source_bill_no,
            voucher_id: m.voucher_id,
            reverse_voucher_id: m.reverse_voucher_id,
            status: m.status,
            confirmed_by: m.confirmed_by,
            confirmed_at: m.confirmed_at.map(|dt| dt.with_timezone(&chrono::Utc)),
            reversed_by: m.reversed_by,
            reversed_at: m.reversed_at.map(|dt| dt.with_timezone(&chrono::Utc)),
            remarks: m.remarks,
            created_by: m.created_by,
            created_at: m.created_at.with_timezone(&chrono::Utc),
            updated_at: m.updated_at.with_timezone(&chrono::Utc),
        }
    }
}

/// POST /api/v1/erp/period-adjustments - 创建期末调整
pub async fn create_adjustment(
    auth: AuthContext,
    State(state): State<AppState>,
    mut req: Json<CreatePeriodAdjustmentRequest>,
) -> Result<Json<ApiResponse<PeriodAdjustmentInfo>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = PeriodAdjustmentService::new(state.db.clone());
    let model = service.create(req.0).await?;
    Ok(Json(ApiResponse::success(model.into())))
}

/// POST /api/v1/erp/period-adjustments/:id/confirm - 确认（生成调整凭证）
pub async fn confirm_adjustment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<PeriodAdjustmentInfo>>, AppError> {
    let service = PeriodAdjustmentService::new(state.db.clone());
    let model = service.confirm(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(model.into())))
}

/// POST /api/v1/erp/period-adjustments/:id/reverse - 红字冲销
pub async fn reverse_adjustment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<PeriodAdjustmentInfo>>, AppError> {
    let service = PeriodAdjustmentService::new(state.db.clone());
    let model = service.reverse(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(model.into())))
}

/// POST /api/v1/erp/period-adjustments/:id/cancel - 取消
pub async fn cancel_adjustment(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<PeriodAdjustmentInfo>>, AppError> {
    let service = PeriodAdjustmentService::new(state.db.clone());
    let model = service.cancel(id).await?;
    Ok(Json(ApiResponse::success(model.into())))
}

/// GET /api/v1/erp/period-adjustments/:id - 详情
pub async fn get_adjustment(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<PeriodAdjustmentInfo>>, AppError> {
    let service = PeriodAdjustmentService::new(state.db.clone());
    let model = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model.into())))
}

/// GET /api/v1/erp/period-adjustments - 列表（按类型/期间/状态过滤分页）
pub async fn list_adjustments(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<PeriodAdjustmentQuery>,
) -> Result<Json<ApiResponse<PagedResponse<PeriodAdjustmentInfo>>>, AppError> {
    let service = PeriodAdjustmentService::new(state.db.clone());
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
    let (items, total) = service.list(query).await?;
    let infos: Vec<PeriodAdjustmentInfo> = items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}
