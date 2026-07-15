//! 流转卡工序流转 Handler
//!
//! v14 批次 425：流转卡工序流转模块
//! 依据：面料行业真实业务调研文档 §14.1 流转卡工序流转（基于同凯印染 ERP/KESHTECH 真实开卡字段）
//! 真实业务流程：
//!   流转卡定义：流转卡=生产流程卡/工序流转卡/缸卡，一卡对应一缸布的生产任务
//!   扫码签入签出（PDA/工控终端）：扫码→登记→签入→签出→流转→入库
//!   分卡/合卡/拆卡/缸终止/内修卡

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::{flow_card, flow_card_operation};
use crate::services::flow_card_service::{
    CreateFlowCardRequest, CreateReworkCardRequest, FlowCardOperationService, FlowCardQuery,
    FlowCardService, MergeCardRequest, PauseRequest, SignInRequest, SignOutRequest, SplitCardRequest,
    SplitPieceRequest, TerminateCardRequest, UpdateFlowCardRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============================================================================
// 辅助函数
// ============================================================================

fn card_service(state: &AppState) -> FlowCardService {
    FlowCardService::new(state.db.clone())
}

fn operation_service(state: &AppState) -> FlowCardOperationService {
    FlowCardOperationService::new(state.db.clone())
}

// ============================================================================
// 查询参数
// ============================================================================

/// 流转卡列表查询参数
#[derive(Debug, Deserialize)]
pub struct FlowCardListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub dye_lot_no: Option<String>,
    pub work_order_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub status: Option<String>,
    pub is_rework: Option<bool>,
}

// ============================================================================
// 流转卡 CRUD Handler
// ============================================================================

/// GET /api/v1/erp/flow-cards - 分页查询流转卡
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<FlowCardListQuery>,
) -> Result<
    Json<ApiResponse<crate::utils::response::PaginatedResponse<flow_card::Model>>>,
    AppError,
> {
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let svc_query = FlowCardQuery {
        dye_lot_no: query.dye_lot_no,
        work_order_id: query.work_order_id,
        production_order_id: query.production_order_id,
        customer_id: query.customer_id,
        status: query.status,
        is_rework: query.is_rework,
        page: Some(page),
        page_size: Some(page_size),
    };

    let (items, total) = card_service(&state).list(svc_query).await?;
    Ok(Json(ApiResponse::success_paginated(items, total, page, page_size)))
}

/// POST /api/v1/erp/flow-cards - 创建流转卡
///
/// 真实业务：业务员/计划员接到生产订单后开具流转卡，承载一缸布从开卡到入库的全部工序信息
pub async fn create(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateFlowCardRequest>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let created = card_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "流转卡创建成功",
    )))
}

/// GET /api/v1/erp/flow-cards/:id - 查询流转卡详情
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let card = card_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(card)))
}

/// PUT /api/v1/erp/flow-cards/:id - 更新流转卡（仅 opened/waiting_dyeing 状态）
pub async fn update(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateFlowCardRequest>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let updated = card_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "流转卡更新成功",
    )))
}

/// DELETE /api/v1/erp/flow-cards/:id - 软删除流转卡（仅 opened/waiting_dyeing 状态）
pub async fn delete(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    card_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "流转卡删除成功",
    )))
}

/// GET /api/v1/erp/flow-cards/by-dye-lot/:dye_lot_no - 按缸号查询流转卡
///
/// 真实业务约束：一缸一卡（同缸号只能有一张主卡）
pub async fn get_by_dye_lot(
    State(state): State<AppState>,
    Path(dye_lot_no): Path<String>,
) -> Result<Json<ApiResponse<Option<flow_card::Model>>>, AppError> {
    let card = card_service(&state).get_by_dye_lot_no(&dye_lot_no).await?;
    Ok(Json(ApiResponse::success(card)))
}

/// GET /api/v1/erp/flow-cards/by-barcode/:barcode - 按条码查询流转卡（扫码用）
///
/// 真实业务：PDA/工控终端扫码识别流转卡
pub async fn get_by_barcode(
    State(state): State<AppState>,
    Path(barcode): Path<String>,
) -> Result<Json<ApiResponse<Option<flow_card::Model>>>, AppError> {
    let card = card_service(&state).get_by_barcode(&barcode).await?;
    Ok(Json(ApiResponse::success(card)))
}

// ============================================================================
// 扫码签入签出 Handler
// ============================================================================

/// POST /api/v1/erp/flow-cards/:id/sign-in - 扫码签入（待加工 → 加工中）
///
/// 真实业务：扫码 → 识别工单号/缸号/工序路线 → 工人刷卡登记 → 记录工号、设备编号、开始时间
pub async fn sign_in(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<SignInRequest>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let updated = card_service(&state).sign_in(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "流转卡签入成功",
    )))
}

/// POST /api/v1/erp/flow-cards/:id/sign-out - 扫码签出（加工中 → 完工）
///
/// 真实业务：扫码 → 记录结束时间、实际产量、疵点数
pub async fn sign_out(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<SignOutRequest>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let updated = card_service(&state).sign_out(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "流转卡签出成功",
    )))
}

/// POST /api/v1/erp/flow-cards/:id/transfer - 流转下道（完工 → 转入下道）
///
/// 真实业务：完工 → 转入下道（系统自动触发下一道工序开工准备）
pub async fn transfer(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let updated = card_service(&state).transfer_to_next(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "流转卡已转入下道工序",
    )))
}

/// POST /api/v1/erp/flow-cards/:id/complete - 完工入库（转入下道 → 完工入库，末道工序）
///
/// 真实业务：完工 → 完工入库（PDA 扫描卷唛条码）
pub async fn complete(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let updated = card_service(&state).complete_and_store(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "流转卡完工入库成功",
    )))
}

/// POST /api/v1/erp/flow-cards/:id/pause - 暂停流转卡
pub async fn pause(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<PauseRequest>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let updated = card_service(&state).pause(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "流转卡已暂停",
    )))
}

/// POST /api/v1/erp/flow-cards/:id/resume - 恢复流转卡（暂停 → 恢复生产）
pub async fn resume(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let updated = card_service(&state).resume(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "流转卡已恢复生产",
    )))
}

// ============================================================================
// 分卡/合卡/拆卡/缸终止/内修卡 Handler
// ============================================================================

/// POST /api/v1/erp/flow-cards/:id/split - 分卡（机缸容量不足，将坯布分成多部分分别染色）
///
/// 真实业务：机缸容量不足，将坯布分成两部分分别染色，生成新卡号
pub async fn split(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<SplitCardRequest>,
) -> Result<Json<ApiResponse<Vec<flow_card::Model>>>, AppError> {
    let new_cards = card_service(&state).split_card(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        new_cards,
        "分卡成功",
    )))
}

/// POST /api/v1/erp/flow-cards/merge - 合缸（多张小卡合并为一缸染色，共享缸号）
///
/// 真实业务：多张小卡合并为一缸染色，共享缸号但保留各自卡号
pub async fn merge(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<MergeCardRequest>,
) -> Result<Json<ApiResponse<Vec<flow_card::Model>>>, AppError> {
    let merged_cards = card_service(&state).merge_card(req).await?;
    Ok(Json(ApiResponse::success_with_message(
        merged_cards,
        "合缸成功",
    )))
}

/// POST /api/v1/erp/flow-cards/:id/split-piece - 拆卡（一匹布过长拆分为多匹，生成子卡号）
///
/// 真实业务：一匹布过长拆分为多匹，生成子卡号关联母卡号
pub async fn split_piece(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<SplitPieceRequest>,
) -> Result<Json<ApiResponse<Vec<flow_card::Model>>>, AppError> {
    let new_cards = card_service(&state).split_piece(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        new_cards,
        "拆卡成功",
    )))
}

/// POST /api/v1/erp/flow-cards/:id/terminate - 缸终止（因质量/工艺问题终止该缸生产）
///
/// 真实业务：因质量/工艺问题终止该缸生产
pub async fn terminate(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<TerminateCardRequest>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let updated = card_service(&state).terminate_card(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "流转卡已缸终止",
    )))
}

/// POST /api/v1/erp/flow-cards/:id/rework - 开内修卡（原卡号 + A/B/C 后缀）
///
/// 真实业务（KESHTECH）：
/// - 内修卡号 = 原始卡号 + A/B/C 后缀（一次回修+A，二次回修+B）
/// - 开内修卡前必须先在"质量异常登记"里登记
pub async fn rework(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<CreateReworkCardRequest>,
) -> Result<Json<ApiResponse<flow_card::Model>>, AppError> {
    let rework_card = card_service(&state).create_rework_card(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        rework_card,
        "内修卡创建成功",
    )))
}

// ============================================================================
// 工序操作记录 Handler
// ============================================================================

/// GET /api/v1/erp/flow-cards/:id/operations - 查询流转卡的工序操作记录
///
/// 真实业务：查询该流转卡的所有工序签入签出记录
pub async fn list_operations(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<flow_card_operation::Model>>>, AppError> {
    let items = card_service(&state).list_operations(id).await?;
    Ok(Json(ApiResponse::success(items)))
}
