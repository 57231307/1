//! 大货处方与加料处方 Handler
//!
//! v14 批次 424：大货处方与加料处方流程
//! 依据：面料行业真实业务调研文档 §11.2 大货处方（染色配料单）与加料处方（染色补料单）
//! 真实业务流程：
//!   大货处方单：扫描流转卡条码 → 依据备布数量 → 加载小样处方/历史大货处方 → 根据浴比/浴量
//!              → 填写物料明细 → 计算用量 → 开具大货处方单 → 审核后自动建立生产领用单据
//!   加料处方单：扫描流转卡 → 加载已审核大货处方 → 登记加料物料 → 生成加料处方单

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::production_recipe::{self, RecipeMaterialItem};
use crate::models::production_recipe_addition;
use crate::services::production_recipe_service::{
    ApproveRecipeRequest, CalculateAmountsRequest, CreateProductionRecipeAdditionRequest,
    CreateProductionRecipeRequest, ProductionRecipeAdditionService, ProductionRecipeQuery,
    ProductionRecipeService, UpdateProductionRecipeRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============================================================================
// 辅助函数
// ============================================================================

fn recipe_service(state: &AppState) -> ProductionRecipeService {
    ProductionRecipeService::new(state.db.clone())
}

fn addition_service(state: &AppState) -> ProductionRecipeAdditionService {
    ProductionRecipeAdditionService::new(state.db.clone())
}

// ============================================================================
// 查询参数
// ============================================================================

/// 大货处方列表查询参数
#[derive(Debug, Deserialize)]
pub struct ProductionRecipeListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub work_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub color_no: Option<String>,
    pub status: Option<String>,
}

// ============================================================================
// 大货处方 Handler
// ============================================================================

/// GET /api/v1/erp/production-recipes - 分页查询大货处方
pub async fn list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ProductionRecipeListQuery>,
) -> Result<
    Json<ApiResponse<crate::utils::response::PaginatedResponse<production_recipe::Model>>>,
    AppError,
> {
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let svc_query = ProductionRecipeQuery {
        work_order_id: query.work_order_id,
        dye_batch_id: query.dye_batch_id,
        customer_id: query.customer_id,
        color_no: query.color_no,
        status: query.status,
        page: Some(page),
        page_size: Some(page_size),
    };

    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let (items, total) = recipe_service(&state)
        .list(svc_query, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success_paginated(items, total, page, page_size)))
}

/// POST /api/v1/erp/production-recipes - 创建大货处方
///
/// 真实业务：扫描流转卡条码 → 依据备布数量 → 加载小样处方/历史大货处方 → 开具大货处方单
pub async fn create(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateProductionRecipeRequest>,
) -> Result<Json<ApiResponse<production_recipe::Model>>, AppError> {
    let created = recipe_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "大货处方单创建成功",
    )))
}

/// GET /api/v1/erp/production-recipes/:id - 查询大货处方详情
pub async fn get(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_recipe::Model>>, AppError> {
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let recipe = recipe_service(&state)
        .get_by_id(id, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(recipe)))
}

/// PUT /api/v1/erp/production-recipes/:id - 更新大货处方（仅 draft 状态）
pub async fn update(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProductionRecipeRequest>,
) -> Result<Json<ApiResponse<production_recipe::Model>>, AppError> {
    let updated = recipe_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "大货处方单更新成功",
    )))
}

/// DELETE /api/v1/erp/production-recipes/:id - 软删除大货处方（仅 draft 状态）
pub async fn delete(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    recipe_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "大货处方单删除成功",
    )))
}

/// POST /api/v1/erp/production-recipes/:id/approve - 审核大货处方（draft → approved）
///
/// 真实业务：审核后自动建立生产领用单据
pub async fn approve(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ApproveRecipeRequest>,
) -> Result<Json<ApiResponse<production_recipe::Model>>, AppError> {
    let updated = recipe_service(&state).approve(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "大货处方单审核成功",
    )))
}

/// POST /api/v1/erp/production-recipes/:id/close - 关闭大货处方（approved → closed）
///
/// 真实业务：生产完成，处方归档
pub async fn close(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_recipe::Model>>, AppError> {
    let updated = recipe_service(&state).close(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "大货处方单已关闭",
    )))
}

/// POST /api/v1/erp/production-recipes/:id/cancel - 取消大货处方（draft → cancelled）
///
/// 真实业务：草稿状态作废
pub async fn cancel(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_recipe::Model>>, AppError> {
    let updated = recipe_service(&state).cancel(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "大货处方单已取消",
    )))
}

/// POST /api/v1/erp/production-recipes/calculate - 用量计算
///
/// 真实业务公式：用量 = 浓度% × 布重 × 浴比 / 100 × 加成系数
pub async fn calculate(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CalculateAmountsRequest>,
) -> Result<Json<ApiResponse<Vec<RecipeMaterialItem>>>, AppError> {
    // 用量计算为纯函数，无需数据库
    let items = ProductionRecipeService::calculate_amounts(req)?;
    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/v1/erp/production-recipes/by-work-order/:work_order_id - 按工单查询大货处方
///
/// 真实业务约束：同一工单号只能开一张大货处方单
pub async fn get_by_work_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(work_order_id): Path<i32>,
) -> Result<Json<ApiResponse<Option<production_recipe::Model>>>, AppError> {
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let recipe = recipe_service(&state)
        .get_by_work_order(work_order_id, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(recipe)))
}

// ============================================================================
// 加料处方 Handler
// ============================================================================

/// GET /api/v1/erp/production-recipes/:id/additions - 查询大货处方下的加料单列表
pub async fn list_additions(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<production_recipe_addition::Model>>>, AppError> {
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护，透传给 list_additions_by_recipe）
    let data_scope_ctx = auth.to_data_scope_context();
    let items = recipe_service(&state)
        .list_additions_by_recipe(id, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(items)))
}

/// POST /api/v1/erp/production-recipes/:id/additions - 创建加料处方
///
/// 真实业务：扫描流转卡 → 加载已审核大货处方 → 登记加料物料 → 生成加料处方单
/// 关键约束：关联的大货处方必须为 approved 状态
pub async fn create_addition(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(mut req): Json<CreateProductionRecipeAdditionRequest>,
) -> Result<Json<ApiResponse<production_recipe_addition::Model>>, AppError> {
    // 路径参数 id 即大货处方 ID，覆盖请求体中的 production_recipe_id 以保证一致性
    req.production_recipe_id = id;
    let created = addition_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "加料处方单创建成功",
    )))
}

/// GET /api/v1/erp/production-recipes/additions/:id - 查询加料处方详情
pub async fn get_addition(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_recipe_addition::Model>>, AppError> {
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let item = addition_service(&state)
        .get_by_id(id, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(item)))
}

/// POST /api/v1/erp/production-recipes/additions/:id/approve - 审核加料处方（draft → approved）
pub async fn approve_addition(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ApproveRecipeRequest>,
) -> Result<Json<ApiResponse<production_recipe_addition::Model>>, AppError> {
    let updated = addition_service(&state).approve(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "加料处方单审核成功",
    )))
}

/// POST /api/v1/erp/production-recipes/additions/:id/close - 关闭加料处方（approved → closed）
pub async fn close_addition(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<production_recipe_addition::Model>>, AppError> {
    let updated = addition_service(&state).close(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "加料处方单已关闭",
    )))
}
