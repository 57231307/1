//! 流转卡与工序流转 Handler
//!
//! v14 批次 425：流转卡条码与车间工序流转
//! 依据：面料行业真实业务调研文档 §12.1 流转卡条码管理 + §12.2 生产计划与排缸 + §12.3 车间工序流转
//! 真实业务流程：生产计划单 → 备布 → 排缸执行 → 流转卡打印（含条码）
//!   扫码应用：白坯出库/染色进度/称料/工序流转/成品入库/发货

use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::models::{
    process_quality_feedback, process_route, process_step_record, production_flow_card,
};
use crate::services::flow_card_service::{
    CompleteStepRequest, CreateFeedbackRequest, CreateFlowCardRequest, CreateProcessRouteRequest,
    FlowCardQuery, FlowCardService, HandleFeedbackRequest, ProcessRouteService,
    QualityFeedbackService, StartStepRequest, StepRecordService, UpdateFlowCardRequest,
    UpdateProcessRouteRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

// ============================================================================
// 辅助函数
// ============================================================================

fn route_service(state: &AppState) -> ProcessRouteService {
    ProcessRouteService::new(state.db.clone())
}

fn card_service(state: &AppState) -> FlowCardService {
    FlowCardService::new(state.db.clone())
}

fn step_service(state: &AppState) -> StepRecordService {
    StepRecordService::new(state.db.clone())
}

fn feedback_service(state: &AppState) -> QualityFeedbackService {
    QualityFeedbackService::new(state.db.clone())
}

// ============================================================================
// 查询参数
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct FlowCardListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub card_no: Option<String>,
    pub barcode: Option<String>,
    pub dye_lot_no: Option<String>,
    pub production_order_id: Option<i32>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct BarcodeQuery {
    pub barcode: Option<String>,
    pub dye_lot_no: Option<String>,
}

// ============================================================================
// 工序路线 Handler
// ============================================================================

/// GET /api/v1/erp/process-routes - 查询所有启用的工序路线
pub async fn list_process_routes(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<process_route::Model>>>, AppError> {
    let list = route_service(&state).list_active().await?;
    Ok(Json(ApiResponse::success(list)))
}

/// POST /api/v1/erp/process-routes - 创建工序路线
pub async fn create_process_route(
    State(state): State<AppState>,
    Json(req): Json<CreateProcessRouteRequest>,
) -> Result<Json<ApiResponse<process_route::Model>>, AppError> {
    let model = route_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/process-routes/:id - 查询工序路线详情
pub async fn get_process_route(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<process_route::Model>>, AppError> {
    let model = route_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/process-routes/:id - 更新工序路线
pub async fn update_process_route(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProcessRouteRequest>,
) -> Result<Json<ApiResponse<process_route::Model>>, AppError> {
    let model = route_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/process-routes/:id - 删除工序路线
pub async fn delete_process_route(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    route_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 流转卡 Handler
// ============================================================================

/// GET /api/v1/erp/flow-cards - 分页查询流转卡
pub async fn list_flow_cards(
    State(state): State<AppState>,
    Query(query): Query<FlowCardListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<production_flow_card::Model>>>, AppError> {
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let svc_query = FlowCardQuery {
        card_no: query.card_no,
        barcode: query.barcode,
        dye_lot_no: query.dye_lot_no,
        production_order_id: query.production_order_id,
        status: query.status,
        customer_id: query.customer_id,
        page: Some(page),
        page_size: Some(page_size),
    };

    let (items, total) = card_service(&state).list(svc_query).await?;
    Ok(Json(ApiResponse::success_paginated(
        items, total, page, page_size,
    )))
}

/// POST /api/v1/erp/flow-cards - 创建流转卡
pub async fn create_flow_card(
    State(state): State<AppState>,
    Json(req): Json<CreateFlowCardRequest>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/flow-cards/:id - 查询流转卡详情
pub async fn get_flow_card(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/flow-cards/:id - 更新流转卡
pub async fn update_flow_card(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateFlowCardRequest>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/flow-cards/:id - 删除流转卡
pub async fn delete_flow_card(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    card_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/erp/flow-cards/by-barcode - 按条码查询流转卡（扫码场景）
pub async fn get_by_barcode(
    State(state): State<AppState>,
    Query(query): Query<BarcodeQuery>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    if let Some(barcode) = query.barcode {
        let model = card_service(&state).get_by_barcode(&barcode).await?;
        Ok(Json(ApiResponse::success(model)))
    } else if let Some(dye_lot_no) = query.dye_lot_no {
        let model = card_service(&state).get_by_dye_lot(&dye_lot_no).await?;
        Ok(Json(ApiResponse::success(model)))
    } else {
        Err(AppError::business(
            "必须提供 barcode 或 dye_lot_no 查询参数",
        ))
    }
}

// ===== 流转卡状态流转 =====

/// POST /api/v1/erp/flow-cards/:id/schedule - 排缸
#[derive(Debug, Deserialize)]
pub struct ScheduleRequest {
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
}

pub async fn schedule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<ScheduleRequest>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state)
        .schedule(id, req.dye_batch_id, req.dye_lot_no)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/start-preparing - 开始备布
pub async fn start_preparing(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).start_preparing(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/complete-preparing - 完成备布
#[derive(Debug, Deserialize)]
pub struct CompletePreparingRequest {
    pub actual_fabric_weight: Decimal,
}

pub async fn complete_preparing(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<CompletePreparingRequest>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state)
        .complete_preparing(id, req.actual_fabric_weight)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/start-dyeing - 进缸染色
pub async fn start_dyeing(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).start_dyeing(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/complete-dyeing - 出缸
pub async fn complete_dyeing(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).complete_dyeing(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/start-inspecting - 开始验布
pub async fn start_inspecting(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).start_inspecting(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/complete - 完成验布入库
pub async fn complete_flow_card(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).complete(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/ship - 发货
pub async fn ship_flow_card(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).ship(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/terminate - 终止
#[derive(Debug, Deserialize)]
pub struct TerminateRequest {
    pub reason: Option<String>,
}

pub async fn terminate_flow_card(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<TerminateRequest>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).terminate(id, req.reason).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/:id/reactivate - 重新激活
pub async fn reactivate_flow_card(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_flow_card::Model>>, AppError> {
    let model = card_service(&state).reactivate(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

// ============================================================================
// 工序流转记录 Handler
// ============================================================================

/// POST /api/v1/erp/flow-cards/steps/start - 扫码开始工序
pub async fn start_step(
    State(state): State<AppState>,
    Json(req): Json<StartStepRequest>,
) -> Result<Json<ApiResponse<process_step_record::Model>>, AppError> {
    let model = step_service(&state).start_step(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/steps/:id/complete - 扫码结束工序
pub async fn complete_step(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<CompleteStepRequest>,
) -> Result<Json<ApiResponse<process_step_record::Model>>, AppError> {
    let model = step_service(&state).complete_step(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/flow-cards/steps/:id - 查询工序记录详情
pub async fn get_step(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<process_step_record::Model>>, AppError> {
    let model = step_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/flow-cards/:flow_card_id/steps - 查询流转卡的所有工序记录
pub async fn list_steps_by_card(
    State(state): State<AppState>,
    Path(flow_card_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<process_step_record::Model>>>, AppError> {
    let list = step_service(&state).list_by_flow_card(flow_card_id).await?;
    Ok(Json(ApiResponse::success(list)))
}

/// POST /api/v1/erp/flow-cards/steps/:source_step_id/rework - 创建回修工序
pub async fn create_rework_step(
    State(state): State<AppState>,
    Path(source_step_id): Path<i32>,
    Json(req): Json<StartStepRequest>,
) -> Result<Json<ApiResponse<process_step_record::Model>>, AppError> {
    let model = step_service(&state)
        .create_rework(source_step_id, req)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

// ============================================================================
// 工序质量反馈单 Handler
// ============================================================================

/// POST /api/v1/erp/flow-cards/feedbacks - 创建质量反馈单
pub async fn create_feedback(
    State(state): State<AppState>,
    Json(req): Json<CreateFeedbackRequest>,
) -> Result<Json<ApiResponse<process_quality_feedback::Model>>, AppError> {
    let model = feedback_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/flow-cards/feedbacks/:id - 查询反馈单详情
pub async fn get_feedback(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<process_quality_feedback::Model>>, AppError> {
    let model = feedback_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/feedbacks/:id/handle - 处理反馈单
pub async fn handle_feedback(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<HandleFeedbackRequest>,
) -> Result<Json<ApiResponse<process_quality_feedback::Model>>, AppError> {
    let model = feedback_service(&state).handle(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/flow-cards/feedbacks/:id/close - 关闭反馈单
pub async fn close_feedback(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<process_quality_feedback::Model>>, AppError> {
    let model = feedback_service(&state).close(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/flow-cards/:flow_card_id/feedbacks - 查询流转卡的所有反馈单
pub async fn list_feedbacks_by_card(
    State(state): State<AppState>,
    Path(flow_card_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<process_quality_feedback::Model>>>, AppError> {
    let list = feedback_service(&state)
        .list_by_flow_card(flow_card_id)
        .await?;
    Ok(Json(ApiResponse::success(list)))
}
