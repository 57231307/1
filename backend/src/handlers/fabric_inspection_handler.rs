//! 验布打卷 Handler
//!
//! v14 批次 426：验布打卷流程贯通
//! 依据：面料行业真实业务调研文档 §12.4 验布打卷与成品入库
//! 真实业务流程：验布机对接码表/电子称 → 疵点采集 → 生成验布报告
//!   → 卷唛标签打印 → PDA 扫描卷唛条码 → 自动入库

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::models::{fabric_defect_record, fabric_inspection_record};
use crate::services::fabric_inspection_service::{
    CreateDefectRequest, CreateInspectionRequest, FabricDefectService, FabricInspectionService,
    GradeInspectionRequest, InspectionQuery, RollFabricRequest, UpdateInspectionRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

// ============================================================================
// 辅助函数
// ============================================================================

fn inspection_service(state: &AppState) -> FabricInspectionService {
    FabricInspectionService::new(state.db.clone())
}

fn defect_service(state: &AppState) -> FabricDefectService {
    FabricDefectService::new(state.db.clone())
}

// ============================================================================
// 查询参数
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct InspectionListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub inspection_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub flow_card_id: Option<i32>,
    pub product_id: Option<i32>,
    pub status: Option<String>,
}

// ============================================================================
// 验布记录 Handler
// ============================================================================

/// GET /api/v1/erp/fabric-inspections - 分页查询验布记录
pub async fn list_inspections(
    State(state): State<AppState>,
    Query(query): Query<InspectionListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<fabric_inspection_record::Model>>>, AppError> {
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let svc_query = InspectionQuery {
        inspection_no: query.inspection_no,
        dye_lot_no: query.dye_lot_no,
        flow_card_id: query.flow_card_id,
        product_id: query.product_id,
        status: query.status,
        page: Some(page),
        page_size: Some(page_size),
    };

    let (items, total) = inspection_service(&state).list(svc_query).await?;
    Ok(Json(ApiResponse::success_paginated(items, total, page, page_size)))
}

/// GET /api/v1/erp/fabric-inspections/:id - 查询验布记录详情
pub async fn get_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<fabric_inspection_record::Model>>, AppError> {
    let inspection = inspection_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(inspection)))
}

/// GET /api/v1/erp/fabric-inspections/by-no/:no - 按单号查询验布记录
pub async fn get_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<fabric_inspection_record::Model>>, AppError> {
    let inspection = inspection_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(inspection)))
}

/// POST /api/v1/erp/fabric-inspections - 创建验布记录
pub async fn create_inspection(
    State(state): State<AppState>,
    Json(req): Json<CreateInspectionRequest>,
) -> Result<Json<ApiResponse<fabric_inspection_record::Model>>, AppError> {
    let created = inspection_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(created, "验布记录创建成功")))
}

/// PUT /api/v1/erp/fabric-inspections/:id - 更新验布记录（仅 pending 状态）
pub async fn update_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateInspectionRequest>,
) -> Result<Json<ApiResponse<fabric_inspection_record::Model>>, AppError> {
    let updated = inspection_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "验布记录更新成功")))
}

/// DELETE /api/v1/erp/fabric-inspections/:id - 软删除验布记录（仅 pending 状态）
pub async fn delete_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    inspection_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success_with_message((), "验布记录删除成功")))
}

/// POST /api/v1/erp/fabric-inspections/:id/start - 开始验布（pending → inspecting）
pub async fn start_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<fabric_inspection_record::Model>>, AppError> {
    let updated = inspection_service(&state).start_inspection(id).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "已开始验布")))
}

/// POST /api/v1/erp/fabric-inspections/:id/grade - 评级（inspecting → graded）
pub async fn grade_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<GradeInspectionRequest>,
) -> Result<Json<ApiResponse<fabric_inspection_record::Model>>, AppError> {
    let updated = inspection_service(&state).grade_inspection(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "验布评级完成")))
}

/// POST /api/v1/erp/fabric-inspections/:id/roll - 打卷入库（graded → rolled）
pub async fn roll_fabric(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<RollFabricRequest>,
) -> Result<Json<ApiResponse<fabric_inspection_record::Model>>, AppError> {
    let updated = inspection_service(&state).roll_fabric(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "打卷入库成功")))
}

/// POST /api/v1/erp/fabric-inspections/:id/close - 关闭归档（rolled → closed）
pub async fn close_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<fabric_inspection_record::Model>>, AppError> {
    let updated = inspection_service(&state).close_inspection(id).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "验布记录已关闭归档")))
}

// ============================================================================
// 疵点明细 Handler
// ============================================================================

/// GET /api/v1/erp/fabric-inspections/:inspection_id/defects - 按验布记录查询疵点明细
pub async fn list_defects_by_inspection(
    State(state): State<AppState>,
    Path(inspection_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<fabric_defect_record::Model>>>, AppError> {
    let defects = defect_service(&state).list_by_inspection(inspection_id).await?;
    Ok(Json(ApiResponse::success(defects)))
}

/// GET /api/v1/erp/fabric-defects/:id - 查询疵点明细详情
pub async fn get_defect(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<fabric_defect_record::Model>>, AppError> {
    let defect = defect_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(defect)))
}

/// POST /api/v1/erp/fabric-defects - 添加疵点明细（自动计算扣分）
pub async fn create_defect(
    State(state): State<AppState>,
    Json(req): Json<CreateDefectRequest>,
) -> Result<Json<ApiResponse<fabric_defect_record::Model>>, AppError> {
    let created = defect_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(created, "疵点明细添加成功")))
}

/// DELETE /api/v1/erp/fabric-defects/:id - 删除疵点明细（仅 inspecting 状态）
pub async fn delete_defect(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    defect_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success_with_message((), "疵点明细删除成功")))
}
