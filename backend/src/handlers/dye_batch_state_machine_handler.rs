//! 缸号全生命周期状态机 Handler
//!
//! v14 批次 432：缸号全生命周期状态机
//! 依据：面料行业真实业务调研文档 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪
//! 真实业务流程：
//!   缸号生命周期日志（14 种状态流转记录）+ 状态流转规则（28 条预置规则）
//!   + 回修记录（4 种类型 + 5 种状态机）+ 操作记录（6 种操作类型）
//! 14 种状态：
//!   pending_schedule/scheduled/preparing/dyeing/washing/fixing/dehydrating/drying/inspecting/stored
//!   /shipped/cancelled/terminated/rework

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::models::{dye_batch_lifecycle_log, dye_batch_operation, dye_batch_rework, dye_batch_state_rule};
use crate::services::dye_batch_state_machine_service::{
    CreateOperationRequest, CreateReworkRequest, CreateStateRuleRequest, CreateTransitionRequest,
    DyeBatchLifecycleLogService, DyeBatchOperationService, DyeBatchReworkService,
    DyeBatchStateRuleService, LifecycleLogQuery, OperationQuery, ReworkQuery, StateRuleQuery,
    UpdateReworkRequest, UpdateStateRuleRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

// ============================================================================
// 辅助函数
// ============================================================================

fn lifecycle_log_service(state: &AppState) -> DyeBatchLifecycleLogService {
    DyeBatchLifecycleLogService::new(state.db.clone())
}

fn state_rule_service(state: &AppState) -> DyeBatchStateRuleService {
    DyeBatchStateRuleService::new(state.db.clone())
}

fn rework_service(state: &AppState) -> DyeBatchReworkService {
    DyeBatchReworkService::new(state.db.clone())
}

fn operation_service(state: &AppState) -> DyeBatchOperationService {
    DyeBatchOperationService::new(state.db.clone())
}

// ============================================================================
// 查询参数
// ============================================================================

/// 生命周期日志列表查询参数
#[derive(Debug, Deserialize)]
pub struct LifecycleLogListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub batch_id: Option<i32>,
    pub batch_no: Option<String>,
    pub transition_code: Option<String>,
    pub keyword: Option<String>,
}

/// 状态规则列表查询参数
#[derive(Debug, Deserialize)]
pub struct StateRuleListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub transition_code: Option<String>,
    pub is_active: Option<bool>,
}

/// 校验状态流转合法性查询参数
#[derive(Debug, Deserialize)]
pub struct CheckTransitionQuery {
    pub from_status: Option<String>,
    pub to_status: String,
    pub transition_code: String,
}

/// 查询允许的流转列表参数
#[derive(Debug, Deserialize)]
pub struct AllowedTransitionsQuery {
    pub from_status: Option<String>,
}

/// 回修记录列表查询参数
#[derive(Debug, Deserialize)]
pub struct ReworkListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub original_batch_id: Option<i32>,
    pub rework_batch_id: Option<i32>,
    pub rework_type: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

/// 操作记录列表查询参数
#[derive(Debug, Deserialize)]
pub struct OperationListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub operation_type: Option<String>,
    pub target_batch_id: Option<i32>,
    pub keyword: Option<String>,
}

/// 审批回修单请求
#[derive(Debug, Deserialize)]
pub struct ApproveReworkRequest {
    pub approved_by: i32,
}

// ============================================================================
// 缸号生命周期日志 Handler
// ============================================================================

/// GET /api/v1/erp/dye-batch-lifecycle-logs - 分页查询缸号生命周期日志
pub async fn list_lifecycle_logs(
    State(state): State<AppState>,
    Query(q): Query<LifecycleLogListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<dye_batch_lifecycle_log::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = LifecycleLogQuery {
        batch_id: q.batch_id,
        batch_no: q.batch_no,
        transition_code: q.transition_code,
        keyword: q.keyword,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = lifecycle_log_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// GET /api/v1/erp/dye-batch-lifecycle-logs/by-batch/:batch_id - 按缸号 ID 查询生命周期日志
pub async fn list_lifecycle_logs_by_batch(
    State(state): State<AppState>,
    Path(batch_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<dye_batch_lifecycle_log::Model>>>, AppError> {
    let items = lifecycle_log_service(&state)
        .list_by_batch(batch_id)
        .await?;
    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/v1/erp/dye-batch-lifecycle-logs/latest-status/:batch_id - 获取缸号最新状态
pub async fn get_latest_status(
    State(state): State<AppState>,
    Path(batch_id): Path<i32>,
) -> Result<Json<ApiResponse<Option<String>>>, AppError> {
    let status = lifecycle_log_service(&state)
        .get_latest_status(batch_id)
        .await?;
    Ok(Json(ApiResponse::success(status)))
}

/// POST /api/v1/erp/dye-batch-lifecycle-logs - 记录状态流转
pub async fn record_transition(
    State(state): State<AppState>,
    Json(req): Json<CreateTransitionRequest>,
) -> Result<Json<ApiResponse<dye_batch_lifecycle_log::Model>>, AppError> {
    let model = lifecycle_log_service(&state)
        .record_transition(req)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/dye-batch-lifecycle-logs/:id - 按ID查询生命周期日志
pub async fn get_lifecycle_log(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch_lifecycle_log::Model>>, AppError> {
    let model = lifecycle_log_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

// ============================================================================
// 缸号状态流转规则 Handler
// ============================================================================

/// GET /api/v1/erp/dye-batch-state-rules - 分页查询状态流转规则
pub async fn list_state_rules(
    State(state): State<AppState>,
    Query(q): Query<StateRuleListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<dye_batch_state_rule::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = StateRuleQuery {
        from_status: q.from_status,
        to_status: q.to_status,
        transition_code: q.transition_code,
        is_active: q.is_active,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = state_rule_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// GET /api/v1/erp/dye-batch-state-rules/allowed-transitions - 查询允许的流转列表
pub async fn list_allowed_transitions(
    State(state): State<AppState>,
    Query(q): Query<AllowedTransitionsQuery>,
) -> Result<Json<ApiResponse<Vec<dye_batch_state_rule::Model>>>, AppError> {
    let items = state_rule_service(&state)
        .list_allowed_transitions(q.from_status.as_deref())
        .await?;
    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/v1/erp/dye-batch-state-rules/check - 校验状态流转合法性
pub async fn check_transition(
    State(state): State<AppState>,
    Query(q): Query<CheckTransitionQuery>,
) -> Result<Json<ApiResponse<bool>>, AppError> {
    let allowed = state_rule_service(&state)
        .check_transition(q.from_status.as_deref(), &q.to_status, &q.transition_code)
        .await?;
    Ok(Json(ApiResponse::success(allowed)))
}

/// POST /api/v1/erp/dye-batch-state-rules - 创建状态流转规则
pub async fn create_state_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateStateRuleRequest>,
) -> Result<Json<ApiResponse<dye_batch_state_rule::Model>>, AppError> {
    let model = state_rule_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/dye-batch-state-rules/:id - 按ID查询状态流转规则
pub async fn get_state_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch_state_rule::Model>>, AppError> {
    let model = state_rule_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/dye-batch-state-rules/:id - 更新状态流转规则
pub async fn update_state_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateStateRuleRequest>,
) -> Result<Json<ApiResponse<dye_batch_state_rule::Model>>, AppError> {
    let model = state_rule_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/dye-batch-state-rules/:id - 删除状态流转规则
pub async fn delete_state_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state_rule_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 缸号回修记录 Handler
// ============================================================================

/// GET /api/v1/erp/dye-batch-reworks - 分页查询回修记录
pub async fn list_reworks(
    State(state): State<AppState>,
    Query(q): Query<ReworkListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<dye_batch_rework::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = ReworkQuery {
        original_batch_id: q.original_batch_id,
        rework_batch_id: q.rework_batch_id,
        rework_type: q.rework_type,
        status: q.status,
        keyword: q.keyword,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = rework_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/dye-batch-reworks - 创建回修记录
pub async fn create_rework(
    State(state): State<AppState>,
    Json(req): Json<CreateReworkRequest>,
) -> Result<Json<ApiResponse<dye_batch_rework::Model>>, AppError> {
    let model = rework_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/dye-batch-reworks/:id - 按ID查询回修记录
pub async fn get_rework(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch_rework::Model>>, AppError> {
    let model = rework_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/dye-batch-reworks/:id/approve - 审批回修单（draft → approved）
pub async fn approve_rework(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<ApproveReworkRequest>,
) -> Result<Json<ApiResponse<dye_batch_rework::Model>>, AppError> {
    let model = rework_service(&state).approve(id, req.approved_by).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/dye-batch-reworks/:id/start - 开始回修（approved → in_progress）
pub async fn start_rework(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch_rework::Model>>, AppError> {
    let model = rework_service(&state).start_rework(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/dye-batch-reworks/:id/complete - 完成回修（in_progress → completed）
pub async fn complete_rework(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch_rework::Model>>, AppError> {
    let model = rework_service(&state).complete_rework(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/dye-batch-reworks/:id/cancel - 取消回修单（非 completed → cancelled）
pub async fn cancel_rework(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch_rework::Model>>, AppError> {
    let model = rework_service(&state).cancel_rework(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/dye-batch-reworks/:id - 更新回修记录（仅 draft 状态可编辑）
pub async fn update_rework(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateReworkRequest>,
) -> Result<Json<ApiResponse<dye_batch_rework::Model>>, AppError> {
    let model = rework_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/dye-batch-reworks/:id - 软删除回修记录
pub async fn delete_rework(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    rework_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 缸号操作记录 Handler
// ============================================================================

/// GET /api/v1/erp/dye-batch-operations - 分页查询操作记录
pub async fn list_operations(
    State(state): State<AppState>,
    Query(q): Query<OperationListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<dye_batch_operation::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = OperationQuery {
        operation_type: q.operation_type,
        target_batch_id: q.target_batch_id,
        keyword: q.keyword,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = operation_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// GET /api/v1/erp/dye-batch-operations/by-type/:operation_type - 按操作类型查询操作记录
pub async fn list_operations_by_type(
    State(state): State<AppState>,
    Path(operation_type): Path<String>,
) -> Result<Json<ApiResponse<Vec<dye_batch_operation::Model>>>, AppError> {
    let items = operation_service(&state)
        .list_by_type(&operation_type)
        .await?;
    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/v1/erp/dye-batch-operations/by-batch/:batch_id - 按目标缸号查询操作记录
pub async fn list_operations_by_batch(
    State(state): State<AppState>,
    Path(batch_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<dye_batch_operation::Model>>>, AppError> {
    let items = operation_service(&state)
        .list_by_batch(batch_id)
        .await?;
    Ok(Json(ApiResponse::success(items)))
}

/// POST /api/v1/erp/dye-batch-operations - 创建操作记录
pub async fn create_operation(
    State(state): State<AppState>,
    Json(req): Json<CreateOperationRequest>,
) -> Result<Json<ApiResponse<dye_batch_operation::Model>>, AppError> {
    let model = operation_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/dye-batch-operations/:id - 按ID查询操作记录
pub async fn get_operation(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch_operation::Model>>, AppError> {
    let model = operation_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}
