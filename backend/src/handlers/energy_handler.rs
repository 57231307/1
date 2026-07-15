//! 能耗管理 Handler
//!
//! v14 批次 428：能耗管理贯通
//! 依据：面料行业真实业务调研文档 §12.6 能耗管理
//! 真实业务流程：
//!   能源计量设备管理 → 时间段能耗登记 → 分摊规则定义 → 月末分摊到缸号/工序/订单

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::models::{energy_allocation_record, energy_allocation_rule, energy_consumption_record, energy_meter};
use crate::services::energy_service::{
    AllocationRecordQuery, AllocationRecordService, ConsumptionQuery,
    CreateAllocationRecordRequest, CreateConsumptionRequest,
    CreateMeterRequest, CreateRuleRequest, EnergyAllocationRuleService, EnergyConsumptionService,
    EnergyMeterService, MeterQuery, MonthlyAllocationRequest, RuleQuery,
    UpdateAllocationRecordRequest, UpdateConsumptionRequest, UpdateMeterRequest, UpdateRuleRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

// ============================================================================
// 辅助函数
// ============================================================================

fn meter_service(state: &AppState) -> EnergyMeterService {
    EnergyMeterService::new(state.db.clone())
}

fn consumption_service(state: &AppState) -> EnergyConsumptionService {
    EnergyConsumptionService::new(state.db.clone())
}

fn rule_service(state: &AppState) -> EnergyAllocationRuleService {
    EnergyAllocationRuleService::new(state.db.clone())
}

fn allocation_record_service(state: &AppState) -> AllocationRecordService {
    AllocationRecordService::new(state.db.clone())
}

// ============================================================================
// 查询参数（HTTP Query 转 Service Query）
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct MeterListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub meter_type: Option<String>,
    pub workshop: Option<String>,
    pub status: Option<String>,
    pub equipment_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ConsumptionListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub meter_id: Option<i32>,
    pub meter_type: Option<String>,
    pub workshop: Option<String>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub equipment_id: Option<i32>,
    pub status: Option<String>,
    pub recording_method: Option<String>,
    pub period_start: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub period_end: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Debug, Deserialize)]
pub struct RuleListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub meter_type: Option<String>,
    pub workshop: Option<String>,
    pub process_route_id: Option<i32>,
    pub allocation_basis: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AllocationRecordListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub meter_type: Option<String>,
    pub workshop: Option<String>,
    pub dye_lot_no: Option<String>,
    pub production_order_id: Option<i32>,
    pub process_route_id: Option<i32>,
    pub allocation_rule_id: Option<i32>,
    pub status: Option<String>,
    pub period_start: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub period_end: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Debug, Deserialize)]
pub struct EffectiveRuleQuery {
    pub workshop: String,
    pub meter_type: String,
    pub process_route_id: Option<i32>,
    pub date: chrono::NaiveDate,
}

// ============================================================================
// 能源计量设备 Handler
// ============================================================================

/// GET /api/v1/erp/energy-meters - 分页查询计量设备
pub async fn list_energy_meters(
    State(state): State<AppState>,
    Query(q): Query<MeterListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<energy_meter::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = MeterQuery {
        meter_type: q.meter_type,
        workshop: q.workshop,
        status: q.status,
        equipment_id: q.equipment_id,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = meter_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/energy-meters - 创建计量设备
pub async fn create_energy_meter(
    State(state): State<AppState>,
    Json(req): Json<CreateMeterRequest>,
) -> Result<Json<ApiResponse<energy_meter::Model>>, AppError> {
    let model = meter_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-meters/by-no/:no - 按编号查询计量设备
pub async fn get_energy_meter_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<energy_meter::Model>>, AppError> {
    let model = meter_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-meters/:id - 查询计量设备详情
pub async fn get_energy_meter(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_meter::Model>>, AppError> {
    let model = meter_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/energy-meters/:id - 更新计量设备
pub async fn update_energy_meter(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateMeterRequest>,
) -> Result<Json<ApiResponse<energy_meter::Model>>, AppError> {
    let model = meter_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/energy-meters/:id - 软删除计量设备
pub async fn delete_energy_meter(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    meter_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 能耗记录 Handler
// ============================================================================

/// GET /api/v1/erp/energy-consumptions - 分页查询能耗记录
pub async fn list_energy_consumptions(
    State(state): State<AppState>,
    Query(q): Query<ConsumptionListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<energy_consumption_record::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = ConsumptionQuery {
        meter_id: q.meter_id,
        meter_type: q.meter_type,
        workshop: q.workshop,
        dye_lot_no: q.dye_lot_no,
        process_route_id: q.process_route_id,
        equipment_id: q.equipment_id,
        status: q.status,
        recording_method: q.recording_method,
        period_start: q.period_start,
        period_end: q.period_end,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = consumption_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/energy-consumptions - 创建能耗记录
pub async fn create_energy_consumption(
    State(state): State<AppState>,
    Json(req): Json<CreateConsumptionRequest>,
) -> Result<Json<ApiResponse<energy_consumption_record::Model>>, AppError> {
    let model = consumption_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-consumptions/by-no/:no - 按编号查询能耗记录
pub async fn get_energy_consumption_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<energy_consumption_record::Model>>, AppError> {
    let model = consumption_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-consumptions/:id - 查询能耗记录详情
pub async fn get_energy_consumption(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_consumption_record::Model>>, AppError> {
    let model = consumption_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/energy-consumptions/:id - 更新能耗记录（仅 draft 状态）
pub async fn update_energy_consumption(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateConsumptionRequest>,
) -> Result<Json<ApiResponse<energy_consumption_record::Model>>, AppError> {
    let model = consumption_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/energy-consumptions/:id - 软删除能耗记录（仅 draft 状态）
pub async fn delete_energy_consumption(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    consumption_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/energy-consumptions/:id/confirm - 确认能耗记录
pub async fn confirm_energy_consumption(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_consumption_record::Model>>, AppError> {
    let model = consumption_service(&state).confirm(id, None).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/energy-consumptions/:id/cancel - 取消能耗记录
pub async fn cancel_energy_consumption(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_consumption_record::Model>>, AppError> {
    let model = consumption_service(&state).cancel(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

// ============================================================================
// 能耗分摊规则 Handler
// ============================================================================

/// GET /api/v1/erp/energy-rules - 分页查询分摊规则
pub async fn list_energy_rules(
    State(state): State<AppState>,
    Query(q): Query<RuleListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<energy_allocation_rule::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = RuleQuery {
        meter_type: q.meter_type,
        workshop: q.workshop,
        process_route_id: q.process_route_id,
        allocation_basis: q.allocation_basis,
        status: q.status,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = rule_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/energy-rules - 创建分摊规则
pub async fn create_energy_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateRuleRequest>,
) -> Result<Json<ApiResponse<energy_allocation_rule::Model>>, AppError> {
    let model = rule_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-rules/by-no/:no - 按编号查询分摊规则
pub async fn get_energy_rule_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<energy_allocation_rule::Model>>, AppError> {
    let model = rule_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-rules/:id - 查询分摊规则详情
pub async fn get_energy_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_allocation_rule::Model>>, AppError> {
    let model = rule_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/energy-rules/:id - 更新分摊规则（仅 draft 状态）
pub async fn update_energy_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateRuleRequest>,
) -> Result<Json<ApiResponse<energy_allocation_rule::Model>>, AppError> {
    let model = rule_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/energy-rules/:id - 软删除分摊规则（仅 draft 状态）
pub async fn delete_energy_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    rule_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/energy-rules/:id/activate - 启用规则
pub async fn activate_energy_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_allocation_rule::Model>>, AppError> {
    let model = rule_service(&state).activate(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/energy-rules/:id/disable - 停用规则
pub async fn disable_energy_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_allocation_rule::Model>>, AppError> {
    let model = rule_service(&state).disable(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-rules/effective - 查询生效规则
pub async fn get_effective_energy_rule(
    State(state): State<AppState>,
    Query(q): Query<EffectiveRuleQuery>,
) -> Result<Json<ApiResponse<Option<energy_allocation_rule::Model>>>, AppError> {
    let model = rule_service(&state)
        .get_effective_rule(&q.workshop, &q.meter_type, q.process_route_id, q.date)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

// ============================================================================
// 能耗分摊记录 Handler
// ============================================================================

/// GET /api/v1/erp/energy-allocations - 分页查询分摊记录
pub async fn list_energy_allocations(
    State(state): State<AppState>,
    Query(q): Query<AllocationRecordListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<energy_allocation_record::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = AllocationRecordQuery {
        meter_type: q.meter_type,
        workshop: q.workshop,
        dye_lot_no: q.dye_lot_no,
        production_order_id: q.production_order_id,
        process_route_id: q.process_route_id,
        allocation_rule_id: q.allocation_rule_id,
        status: q.status,
        period_start: q.period_start,
        period_end: q.period_end,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = allocation_record_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/energy-allocations - 创建分摊记录
pub async fn create_energy_allocation(
    State(state): State<AppState>,
    Json(req): Json<CreateAllocationRecordRequest>,
) -> Result<Json<ApiResponse<energy_allocation_record::Model>>, AppError> {
    let model = allocation_record_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-allocations/by-no/:no - 按编号查询分摊记录
pub async fn get_energy_allocation_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<energy_allocation_record::Model>>, AppError> {
    let model = allocation_record_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/energy-allocations/:id - 查询分摊记录详情
pub async fn get_energy_allocation(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_allocation_record::Model>>, AppError> {
    let model = allocation_record_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/energy-allocations/:id - 更新分摊记录（仅 draft 状态）
pub async fn update_energy_allocation(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateAllocationRecordRequest>,
) -> Result<Json<ApiResponse<energy_allocation_record::Model>>, AppError> {
    let model = allocation_record_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/energy-allocations/:id - 软删除分摊记录（仅 draft 状态）
pub async fn delete_energy_allocation(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    allocation_record_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/energy-allocations/:id/confirm - 确认分摊记录
pub async fn confirm_energy_allocation(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_allocation_record::Model>>, AppError> {
    let model = allocation_record_service(&state).confirm(id, None).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/energy-allocations/:id/cancel - 取消分摊记录
pub async fn cancel_energy_allocation(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<energy_allocation_record::Model>>, AppError> {
    let model = allocation_record_service(&state).cancel(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/energy-allocations/monthly - 月末按工时自动分摊
pub async fn monthly_allocation(
    State(state): State<AppState>,
    Json(req): Json<MonthlyAllocationRequest>,
) -> Result<Json<ApiResponse<Vec<energy_allocation_record::Model>>>, AppError> {
    let consumption_svc = consumption_service(&state);
    let rule_svc = rule_service(&state);
    let allocation_svc = allocation_record_service(&state);
    let results = allocation_svc
        .monthly_allocation_by_duration(req, &consumption_svc, &rule_svc)
        .await?;
    Ok(Json(ApiResponse::success(results)))
}
