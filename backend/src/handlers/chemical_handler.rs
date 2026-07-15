//! 染化料主数据 Handler
//!
//! v14 批次 429：染化料主数据完善
//! 依据：面料行业真实业务调研文档 §4.3 染化料管理 + §11.4 染化料主数据管理
//! 真实业务流程：
//!   染化料分类树 → 染化料主数据（GHS 危化品 + MSDS）→ 来料批次（lot_no + 效期 + 检验状态）
//!   → 领用单（生产/化验室/研发，关联染色缸号）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::models::{
    chemical_category, chemical_lot, chemical_master, chemical_requisition,
};
use crate::services::chemical_service::{
    ChemicalCategoryService, ChemicalLotService, ChemicalMasterService,
    ChemicalRequisitionService,
    ChemicalCategoryQuery, ChemicalLotQuery, ChemicalMasterQuery, ChemicalRequisitionQuery,
    CreateChemicalCategoryRequest, CreateChemicalLotRequest,
    CreateChemicalMasterRequest, CreateChemicalRequisitionRequest,
    UpdateChemicalCategoryRequest, UpdateChemicalLotRequest,
    UpdateChemicalMasterRequest, UpdateChemicalRequisitionRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

// ============================================================================
// 辅助函数
// ============================================================================

fn master_service(state: &AppState) -> ChemicalMasterService {
    ChemicalMasterService::new(state.db.clone())
}

fn category_service(state: &AppState) -> ChemicalCategoryService {
    ChemicalCategoryService::new(state.db.clone())
}

fn lot_service(state: &AppState) -> ChemicalLotService {
    ChemicalLotService::new(state.db.clone())
}

fn requisition_service(state: &AppState) -> ChemicalRequisitionService {
    ChemicalRequisitionService::new(state.db.clone())
}

// ============================================================================
// 查询参数（HTTP Query 转 Service Query）
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ChemicalMasterListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub chemical_type: Option<String>,
    pub category_id: Option<i32>,
    pub dye_category: Option<String>,
    pub auxiliary_category: Option<String>,
    pub supplier_id: Option<i32>,
    pub status: Option<String>,
    pub cas_number: Option<String>,
    pub ghs_classification: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChemicalCategoryListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub parent_id: Option<i32>,
    pub category_type: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ChemicalCategoryTreeQuery {
    pub parent_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ChemicalLotListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub chemical_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub inspection_status: Option<String>,
    pub storage_zone: Option<String>,
    pub status: Option<String>,
    pub expiry_before: Option<chrono::NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct ChemicalRequisitionListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub requisition_type: Option<String>,
    pub department_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub status: Option<String>,
    pub requisition_date_start: Option<chrono::NaiveDate>,
    pub requisition_date_end: Option<chrono::NaiveDate>,
}

/// 检验报告请求体（用于 pass_inspection / fail_inspection 接口）
#[derive(Debug, Deserialize)]
pub struct InspectionReportRequest {
    pub inspection_report_url: Option<String>,
}

/// 发料/审批请求体（用于 approve / issue 接口）
#[derive(Debug, Deserialize)]
pub struct OperatorRequest {
    pub operator_id: Option<i32>,
}

// ============================================================================
// 染化料主数据 Handler
// ============================================================================

/// GET /api/v1/erp/chemicals - 分页查询染化料主数据
pub async fn list_chemicals(
    State(state): State<AppState>,
    Query(q): Query<ChemicalMasterListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<chemical_master::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = ChemicalMasterQuery {
        chemical_type: q.chemical_type,
        category_id: q.category_id,
        dye_category: q.dye_category,
        auxiliary_category: q.auxiliary_category,
        supplier_id: q.supplier_id,
        status: q.status,
        cas_number: q.cas_number,
        ghs_classification: q.ghs_classification,
        keyword: q.keyword,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = master_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/chemicals - 创建染化料主数据
pub async fn create_chemical(
    State(state): State<AppState>,
    Json(req): Json<CreateChemicalMasterRequest>,
) -> Result<Json<ApiResponse<chemical_master::Model>>, AppError> {
    let model = master_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/chemicals/by-code/:code - 按编码查询染化料主数据
pub async fn get_chemical_by_code(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<chemical_master::Model>>, AppError> {
    let model = master_service(&state).get_by_code(&code).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/chemicals/:id - 查询染化料主数据详情
pub async fn get_chemical(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<chemical_master::Model>>, AppError> {
    let model = master_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/chemicals/:id - 更新染化料主数据
pub async fn update_chemical(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateChemicalMasterRequest>,
) -> Result<Json<ApiResponse<chemical_master::Model>>, AppError> {
    let model = master_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/chemicals/:id - 软删除染化料主数据
pub async fn delete_chemical(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    master_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 染化料分类 Handler
// ============================================================================

/// GET /api/v1/erp/chemical-categories - 分页查询染化料分类
pub async fn list_chemical_categories(
    State(state): State<AppState>,
    Query(q): Query<ChemicalCategoryListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<chemical_category::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = ChemicalCategoryQuery {
        parent_id: q.parent_id,
        category_type: q.category_type,
        is_active: q.is_active,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = category_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/chemical-categories - 创建染化料分类
pub async fn create_chemical_category(
    State(state): State<AppState>,
    Json(req): Json<CreateChemicalCategoryRequest>,
) -> Result<Json<ApiResponse<chemical_category::Model>>, AppError> {
    let model = category_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/chemical-categories/:id - 查询染化料分类详情
pub async fn get_chemical_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<chemical_category::Model>>, AppError> {
    let model = category_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/chemical-categories/:id - 更新染化料分类
pub async fn update_chemical_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateChemicalCategoryRequest>,
) -> Result<Json<ApiResponse<chemical_category::Model>>, AppError> {
    let model = category_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/chemical-categories/:id - 软删除染化料分类
pub async fn delete_chemical_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    category_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/erp/chemical-categories/tree - 查询分类树
pub async fn get_chemical_category_tree(
    State(state): State<AppState>,
    Query(q): Query<ChemicalCategoryTreeQuery>,
) -> Result<Json<ApiResponse<Vec<chemical_category::Model>>>, AppError> {
    let items = category_service(&state).get_tree(q.parent_id).await?;
    Ok(Json(ApiResponse::success(items)))
}

// ============================================================================
// 染化料批次 Handler
// ============================================================================

/// GET /api/v1/erp/chemical-lots - 分页查询染化料批次
pub async fn list_chemical_lots(
    State(state): State<AppState>,
    Query(q): Query<ChemicalLotListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<chemical_lot::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = ChemicalLotQuery {
        chemical_id: q.chemical_id,
        supplier_id: q.supplier_id,
        warehouse_id: q.warehouse_id,
        inspection_status: q.inspection_status,
        storage_zone: q.storage_zone,
        status: q.status,
        expiry_before: q.expiry_before,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = lot_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/chemical-lots - 创建染化料批次
pub async fn create_chemical_lot(
    State(state): State<AppState>,
    Json(req): Json<CreateChemicalLotRequest>,
) -> Result<Json<ApiResponse<chemical_lot::Model>>, AppError> {
    let model = lot_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/chemical-lots/by-no/:no - 按批号查询染化料批次
pub async fn get_chemical_lot_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<chemical_lot::Model>>, AppError> {
    let model = lot_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/chemical-lots/:id - 查询染化料批次详情
pub async fn get_chemical_lot(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<chemical_lot::Model>>, AppError> {
    let model = lot_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/chemical-lots/:id - 更新染化料批次
pub async fn update_chemical_lot(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateChemicalLotRequest>,
) -> Result<Json<ApiResponse<chemical_lot::Model>>, AppError> {
    let model = lot_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/chemical-lots/:id - 软删除染化料批次
pub async fn delete_chemical_lot(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    lot_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/chemical-lots/:id/pass-inspection - 来料检验合格
pub async fn pass_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<InspectionReportRequest>,
) -> Result<Json<ApiResponse<chemical_lot::Model>>, AppError> {
    let model = lot_service(&state)
        .pass_inspection(id, req.inspection_report_url)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/chemical-lots/:id/fail-inspection - 来料检验不合格
pub async fn fail_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<InspectionReportRequest>,
) -> Result<Json<ApiResponse<chemical_lot::Model>>, AppError> {
    let model = lot_service(&state)
        .fail_inspection(id, req.inspection_report_url)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/chemical-lots/:id/consume - 标记批次已耗尽
pub async fn consume_lot(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<chemical_lot::Model>>, AppError> {
    let model = lot_service(&state).consume(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/chemical-lots/:id/scrap - 报废批次
pub async fn scrap_lot(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<chemical_lot::Model>>, AppError> {
    let model = lot_service(&state).scrap(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

// ============================================================================
// 染化料领用单 Handler
// ============================================================================

/// GET /api/v1/erp/chemical-requisitions - 分页查询染化料领用单
pub async fn list_requisitions(
    State(state): State<AppState>,
    Query(q): Query<ChemicalRequisitionListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<chemical_requisition::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = ChemicalRequisitionQuery {
        requisition_type: q.requisition_type,
        department_id: q.department_id,
        dye_batch_id: q.dye_batch_id,
        production_order_id: q.production_order_id,
        status: q.status,
        requisition_date_start: q.requisition_date_start,
        requisition_date_end: q.requisition_date_end,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = requisition_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/chemical-requisitions - 创建染化料领用单
pub async fn create_requisition(
    State(state): State<AppState>,
    Json(req): Json<CreateChemicalRequisitionRequest>,
) -> Result<Json<ApiResponse<chemical_requisition::Model>>, AppError> {
    let model = requisition_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/chemical-requisitions/by-no/:no - 按单号查询领用单
pub async fn get_requisition_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<chemical_requisition::Model>>, AppError> {
    let model = requisition_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/chemical-requisitions/:id - 查询领用单详情
pub async fn get_requisition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<chemical_requisition::Model>>, AppError> {
    let model = requisition_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/chemical-requisitions/:id - 更新领用单（仅 draft 状态）
pub async fn update_requisition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateChemicalRequisitionRequest>,
) -> Result<Json<ApiResponse<chemical_requisition::Model>>, AppError> {
    let model = requisition_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/chemical-requisitions/:id - 软删除领用单（仅 draft 状态）
pub async fn delete_requisition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    requisition_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/chemical-requisitions/:id/approve - 审批领用单
pub async fn approve_requisition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<OperatorRequest>,
) -> Result<Json<ApiResponse<chemical_requisition::Model>>, AppError> {
    let model = requisition_service(&state)
        .approve(id, req.operator_id)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/chemical-requisitions/:id/issue - 发料
pub async fn issue_requisition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<OperatorRequest>,
) -> Result<Json<ApiResponse<chemical_requisition::Model>>, AppError> {
    let model = requisition_service(&state)
        .issue(id, req.operator_id)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/chemical-requisitions/:id/close - 关闭领用单
pub async fn close_requisition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<chemical_requisition::Model>>, AppError> {
    let model = requisition_service(&state).close(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/chemical-requisitions/:id/cancel - 取消领用单
pub async fn cancel_requisition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<chemical_requisition::Model>>, AppError> {
    let model = requisition_service(&state).cancel(id).await?;
    Ok(Json(ApiResponse::success(model)))
}
