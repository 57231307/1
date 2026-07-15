//! 产量工资 Handler
//!
//! v14 批次 427：产量工资核算贯通
//! 依据：面料行业真实业务调研文档 §12.5 产量工资（计件计时）
//! 真实业务流程：
//!   工序流转扫码 → 工价方案定义 → 工资计算 → 班组汇总 → 进入财务工资核算

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use crate::models::{process_wage_rate, wage_record, wage_record_detail};
use crate::services::wage_service::{
    CalculateWageRequest, CreateWageRateRequest, CreateWageRecordRequest, UpdateWageRateRequest,
    UpdateWageRecordRequest, WageCalculationService, WageRateQuery, WageRateService,
    WageRecordQuery, WageRecordService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

// ============================================================================
// 辅助函数
// ============================================================================

fn rate_service(state: &AppState) -> WageRateService {
    WageRateService::new(state.db.clone())
}

fn record_service(state: &AppState) -> WageRecordService {
    WageRecordService::new(state.db.clone())
}

fn calc_service(state: &AppState) -> WageCalculationService {
    WageCalculationService::new(state.db.clone())
}

// ============================================================================
// 查询参数
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WageRateListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub route_code: Option<String>,
    pub process_route_id: Option<i32>,
    pub workshop: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WageRecordListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub record_no: Option<String>,
    pub workshop: Option<String>,
    pub status: Option<String>,
    pub period_start: Option<chrono::NaiveDate>,
    pub period_end: Option<chrono::NaiveDate>,
}

// ============================================================================
// 工序工价 Handler
// ============================================================================

/// GET /api/v1/erp/wage-rates - 分页查询工价
pub async fn list_wage_rates(
    State(state): State<AppState>,
    Query(q): Query<WageRateListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<process_wage_rate::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = WageRateQuery {
        route_code: q.route_code,
        process_route_id: q.process_route_id,
        workshop: q.workshop,
        status: q.status,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = rate_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/wage-rates - 创建工价
pub async fn create_wage_rate(
    State(state): State<AppState>,
    Json(req): Json<CreateWageRateRequest>,
) -> Result<Json<ApiResponse<process_wage_rate::Model>>, AppError> {
    let model = rate_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/wage-rates/by-no/:no - 按单号查询工价
pub async fn get_wage_rate_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<process_wage_rate::Model>>, AppError> {
    let model = rate_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/wage-rates/:id - 查询工价详情
pub async fn get_wage_rate(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<process_wage_rate::Model>>, AppError> {
    let model = rate_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/wage-rates/:id - 更新工价（仅 draft 状态）
pub async fn update_wage_rate(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWageRateRequest>,
) -> Result<Json<ApiResponse<process_wage_rate::Model>>, AppError> {
    let model = rate_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/wage-rates/:id - 删除工价（仅 draft 状态）
pub async fn delete_wage_rate(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    rate_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/wage-rates/:id/activate - 启用工价（draft → active）
pub async fn activate_wage_rate(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<process_wage_rate::Model>>, AppError> {
    let model = rate_service(&state).activate(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/wage-rates/:id/disable - 停用工价（active → disabled）
pub async fn disable_wage_rate(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<process_wage_rate::Model>>, AppError> {
    let model = rate_service(&state).disable(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/wage-rates/effective/:route_id - 查询工序当前生效的工价
pub async fn get_effective_wage_rate(
    State(state): State<AppState>,
    Path(route_id): Path<i32>,
    Query(q): Query<EffectiveDateQuery>,
) -> Result<Json<ApiResponse<Option<process_wage_rate::Model>>>, AppError> {
    let on_date = q.on_date.unwrap_or_else(chrono::Utc::now).date_naive();
    let model = rate_service(&state)
        .get_effective_by_route(route_id, on_date)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

#[derive(Debug, Deserialize)]
pub struct EffectiveDateQuery {
    pub on_date: Option<chrono::DateTime<chrono::Utc>>,
}

// ============================================================================
// 工资记录 Handler
// ============================================================================

/// GET /api/v1/erp/wage-records - 分页查询工资记录
pub async fn list_wage_records(
    State(state): State<AppState>,
    Query(q): Query<WageRecordListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<wage_record::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = WageRecordQuery {
        record_no: q.record_no,
        workshop: q.workshop,
        status: q.status,
        period_start: q.period_start,
        period_end: q.period_end,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = record_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/wage-records - 创建工资记录（仅创建空记录，需调用 calculate 触发计算）
pub async fn create_wage_record(
    State(state): State<AppState>,
    Json(req): Json<CreateWageRecordRequest>,
) -> Result<Json<ApiResponse<wage_record::Model>>, AppError> {
    let model = record_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/wage-records/by-no/:no - 按单号查询工资记录
pub async fn get_wage_record_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<wage_record::Model>>, AppError> {
    let model = record_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/wage-records/:id - 查询工资记录详情
pub async fn get_wage_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<wage_record::Model>>, AppError> {
    let model = record_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/wage-records/:id - 更新工资记录（仅 draft 状态）
pub async fn update_wage_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWageRecordRequest>,
) -> Result<Json<ApiResponse<wage_record::Model>>, AppError> {
    let model = record_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/wage-records/:id - 删除工资记录（仅 draft 状态）
pub async fn delete_wage_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    record_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/wage-records/:id/calculate - 触发工资计算
pub async fn calculate_wage(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<CalculateWageRequest>,
) -> Result<Json<ApiResponse<wage_record::Model>>, AppError> {
    let model = calc_service(&state).calculate(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/wage-records/:id/confirm - 确认工资（draft → confirmed）
pub async fn confirm_wage_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<OperatorRequest>,
) -> Result<Json<ApiResponse<wage_record::Model>>, AppError> {
    let model = record_service(&state).confirm(id, req.operator_id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/wage-records/:id/pay - 发放工资（confirmed → paid）
pub async fn pay_wage_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<OperatorRequest>,
) -> Result<Json<ApiResponse<wage_record::Model>>, AppError> {
    let model = record_service(&state).pay(id, req.operator_id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/wage-records/:id/cancel - 取消工资（draft/confirmed → cancelled）
pub async fn cancel_wage_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<wage_record::Model>>, AppError> {
    let model = record_service(&state).cancel(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

#[derive(Debug, Deserialize)]
pub struct OperatorRequest {
    pub operator_id: i32,
}

// ============================================================================
// 工资明细 Handler
// ============================================================================

/// GET /api/v1/erp/wage-records/:id/details - 查询工资记录的明细列表
pub async fn list_wage_details(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(q): Query<DetailListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<wage_record_detail::Model>>>, AppError> {
    let page = q.page.unwrap_or(1).clamp(1, 1000);
    let page_size = q.page_size.unwrap_or(50).clamp(1, 200);

    let mut query = wage_record_detail::Entity::find()
        .filter(wage_record_detail::Column::WageRecordId.eq(id))
        .filter(wage_record_detail::Column::IsDeleted.eq(false));

    if let Some(v) = q.worker_id {
        query = query.filter(wage_record_detail::Column::WorkerId.eq(v));
    }
    if let Some(v) = q.grade {
        query = query.filter(wage_record_detail::Column::Grade.eq(v));
    }
    if let Some(v) = q.process_type {
        query = query.filter(wage_record_detail::Column::ProcessType.eq(v));
    }
    if let Some(v) = q.flow_card_id {
        query = query.filter(wage_record_detail::Column::FlowCardId.eq(v));
    }

    let total = query.clone().count(&*state.db).await?;
    let items = query
        .order_by_desc(wage_record_detail::Column::WageAmount)
        .paginate(&*state.db, page_size)
        .fetch_page(page - 1)
        .await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

#[derive(Debug, Deserialize)]
pub struct DetailListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub worker_id: Option<i32>,
    pub grade: Option<String>,
    pub process_type: Option<String>,
    pub flow_card_id: Option<i32>,
}

/// GET /api/v1/erp/wage-details/by-worker/:worker_id - 按工人查询历史工资
pub async fn list_wage_details_by_worker(
    State(state): State<AppState>,
    Path(worker_id): Path<i32>,
    Query(q): Query<DetailListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<wage_record_detail::Model>>>, AppError> {
    let page = q.page.unwrap_or(1).clamp(1, 1000);
    let page_size = q.page_size.unwrap_or(50).clamp(1, 200);

    let mut query = wage_record_detail::Entity::find()
        .filter(wage_record_detail::Column::WorkerId.eq(worker_id))
        .filter(wage_record_detail::Column::IsDeleted.eq(false));

    if let Some(v) = q.grade {
        query = query.filter(wage_record_detail::Column::Grade.eq(v));
    }
    if let Some(v) = q.process_type {
        query = query.filter(wage_record_detail::Column::ProcessType.eq(v));
    }

    let total = query.clone().count(&*state.db).await?;
    let items = query
        .order_by_desc(wage_record_detail::Column::CreatedAt)
        .paginate(&*state.db, page_size)
        .fetch_page(page - 1)
        .await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}
