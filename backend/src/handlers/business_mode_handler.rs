//! 多业务模式支持 Handler
//!
//! v14 批次 431：多业务模式支持
//! 依据：面料行业真实业务调研文档 §6 业务模式 6 种
//! 真实业务流程：
//!   业务模式配置（含 6 种预置模式）+ 流程节点 + 业务规则 + 单据关联
//!   6 种业务模式：坯布经销/成品经销/染整加工/自织自染/委托加工/来料加工

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::models::{business_mode_config, business_mode_flow_step, business_mode_order_link, business_mode_rule};
use crate::services::business_mode_service::{
    BusinessModeConfigQuery, BusinessModeConfigService, BusinessModeFlowStepService,
    BusinessModeOrderLinkQuery, BusinessModeOrderLinkService, BusinessModeRuleService,
    CreateBusinessModeConfigRequest, CreateBusinessModeFlowStepRequest,
    CreateBusinessModeOrderLinkRequest, CreateBusinessModeRuleRequest,
    UpdateBusinessModeConfigRequest, UpdateBusinessModeFlowStepRequest,
    UpdateBusinessModeOrderLinkRequest, UpdateBusinessModeRuleRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

// ============================================================================
// 辅助函数
// ============================================================================

fn config_service(state: &AppState) -> BusinessModeConfigService {
    BusinessModeConfigService::new(state.db.clone())
}

fn flow_step_service(state: &AppState) -> BusinessModeFlowStepService {
    BusinessModeFlowStepService::new(state.db.clone())
}

fn rule_service(state: &AppState) -> BusinessModeRuleService {
    BusinessModeRuleService::new(state.db.clone())
}

fn order_link_service(state: &AppState) -> BusinessModeOrderLinkService {
    BusinessModeOrderLinkService::new(state.db.clone())
}

// ============================================================================
// 查询参数
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BusinessModeListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub mode_code: Option<String>,
    pub mode_category: Option<String>,
    pub is_active: Option<bool>,
    pub material_source: Option<String>,
    pub settlement_method: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FlowStepListByModeQuery {}

#[derive(Debug, Deserialize)]
pub struct RuleListByModeQuery {}

#[derive(Debug, Deserialize)]
pub struct OrderLinkListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub mode_id: Option<i32>,
    pub document_type: Option<String>,
    pub document_no: Option<String>,
}

// ============================================================================
// 业务模式详情响应（含流程节点+规则）
// ============================================================================

/// 业务模式完整详情响应
#[derive(Debug, Serialize)]
pub struct BusinessModeFullDetailResponse {
    pub config: business_mode_config::Model,
    pub flow_steps: Vec<business_mode_flow_step::Model>,
    pub rules: Vec<business_mode_rule::Model>,
}

// ============================================================================
// 业务模式配置 Handler
// ============================================================================

/// GET /api/v1/erp/business-modes - 分页查询业务模式配置
pub async fn list_business_modes(
    State(state): State<AppState>,
    Query(q): Query<BusinessModeListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<business_mode_config::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = BusinessModeConfigQuery {
        mode_code: q.mode_code,
        mode_category: q.mode_category,
        is_active: q.is_active,
        material_source: q.material_source,
        settlement_method: q.settlement_method,
        keyword: q.keyword,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = config_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/business-modes - 创建业务模式配置
pub async fn create_business_mode(
    State(state): State<AppState>,
    Json(req): Json<CreateBusinessModeConfigRequest>,
) -> Result<Json<ApiResponse<business_mode_config::Model>>, AppError> {
    let model = config_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/business-modes/default - 查询默认业务模式
pub async fn get_default_business_mode(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Option<business_mode_config::Model>>>, AppError> {
    let model = config_service(&state).get_default_mode().await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/business-modes/by-code/:code - 按模式代码查询业务模式
pub async fn get_business_mode_by_code(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<business_mode_config::Model>>, AppError> {
    let model = config_service(&state).get_by_code(&code).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/business-modes/:id - 查询业务模式详情
pub async fn get_business_mode(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<business_mode_config::Model>>, AppError> {
    let model = config_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/business-modes/:id - 更新业务模式配置
pub async fn update_business_mode(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateBusinessModeConfigRequest>,
) -> Result<Json<ApiResponse<business_mode_config::Model>>, AppError> {
    let model = config_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/business-modes/:id - 软删除业务模式配置
pub async fn delete_business_mode(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    config_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/business-modes/:id/set-default - 设置默认业务模式
pub async fn set_default_business_mode(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    config_service(&state).set_default(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/erp/business-modes/:id/detail - 查询业务模式完整详情（含流程节点+规则）
pub async fn get_business_mode_detail(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<BusinessModeFullDetailResponse>>, AppError> {
    let (config, flow_steps, rules) = config_service(&state).get_full_detail(id).await?;
    Ok(Json(ApiResponse::success(BusinessModeFullDetailResponse {
        config,
        flow_steps,
        rules,
    })))
}

// ============================================================================
// 业务模式流程节点 Handler
// ============================================================================

/// GET /api/v1/erp/business-modes/flow-steps/by-mode/:mode_id - 按业务模式查询流程节点
pub async fn list_flow_steps_by_mode(
    State(state): State<AppState>,
    Path(mode_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<business_mode_flow_step::Model>>>, AppError> {
    let items = flow_step_service(&state).list_by_mode(mode_id).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// POST /api/v1/erp/business-modes/flow-steps - 创建业务模式流程节点
pub async fn create_flow_step(
    State(state): State<AppState>,
    Json(req): Json<CreateBusinessModeFlowStepRequest>,
) -> Result<Json<ApiResponse<business_mode_flow_step::Model>>, AppError> {
    let model = flow_step_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/business-modes/flow-steps/:id - 更新业务模式流程节点
pub async fn update_flow_step(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateBusinessModeFlowStepRequest>,
) -> Result<Json<ApiResponse<business_mode_flow_step::Model>>, AppError> {
    let model = flow_step_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/business-modes/flow-steps/:id - 删除业务模式流程节点
pub async fn delete_flow_step(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    flow_step_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 业务模式规则 Handler
// ============================================================================

/// GET /api/v1/erp/business-modes/rules/by-mode/:mode_id - 按业务模式查询规则
pub async fn list_rules_by_mode(
    State(state): State<AppState>,
    Path(mode_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<business_mode_rule::Model>>>, AppError> {
    let items = rule_service(&state).list_by_mode(mode_id).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// POST /api/v1/erp/business-modes/rules - 创建业务模式规则
pub async fn create_rule(
    State(state): State<AppState>,
    Json(req): Json<CreateBusinessModeRuleRequest>,
) -> Result<Json<ApiResponse<business_mode_rule::Model>>, AppError> {
    let model = rule_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/business-modes/rules/:id - 更新业务模式规则
pub async fn update_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateBusinessModeRuleRequest>,
) -> Result<Json<ApiResponse<business_mode_rule::Model>>, AppError> {
    let model = rule_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/business-modes/rules/:id - 删除业务模式规则
pub async fn delete_rule(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    rule_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 单据-业务模式关联 Handler
// ============================================================================

/// GET /api/v1/erp/business-mode-links - 分页查询单据-业务模式关联
pub async fn list_order_links(
    State(state): State<AppState>,
    Query(q): Query<OrderLinkListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<business_mode_order_link::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = BusinessModeOrderLinkQuery {
        mode_id: q.mode_id,
        document_type: q.document_type,
        document_no: q.document_no,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = order_link_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/business-mode-links - 关联单据到业务模式
pub async fn link_order(
    State(state): State<AppState>,
    Json(req): Json<CreateBusinessModeOrderLinkRequest>,
) -> Result<Json<ApiResponse<business_mode_order_link::Model>>, AppError> {
    let model = order_link_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/business-mode-links/by-document/:doc_type/:doc_id - 按单据查询关联
pub async fn get_order_link_by_document(
    State(state): State<AppState>,
    Path((doc_type, doc_id)): Path<(String, i32)>,
) -> Result<Json<ApiResponse<Option<business_mode_order_link::Model>>>, AppError> {
    let model = order_link_service(&state)
        .get_by_document(&doc_type, doc_id)
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/business-mode-links/:id - 更新单据-业务模式关联
pub async fn update_order_link(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateBusinessModeOrderLinkRequest>,
) -> Result<Json<ApiResponse<business_mode_order_link::Model>>, AppError> {
    let model = order_link_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/business-mode-links/:id - 删除单据-业务模式关联
pub async fn delete_order_link(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    order_link_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}
