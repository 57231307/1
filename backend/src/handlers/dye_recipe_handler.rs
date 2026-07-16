//! 染色配方管理 Handler
//!
//! v14 批次 423A 重构：从直接 ActiveModel 操作改为调用 DyeRecipeService 抽象层，
//! 状态字符串统一引用 status::dye_recipe 常量，业务逻辑下沉到 service 便于单元测试。
//! 依据：面料行业真实业务调研文档 §11.1 化验室打样流程 + §13.1 批次 423 规划

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::models::dye_recipe;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::dye_recipe_service::{
    CreateDyeRecipeRequest, DyeRecipeQuery, DyeRecipeService, UpdateDyeRecipeRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};
use std::sync::Arc;

/// 列表查询参数（axum Query 反序列化用）
#[derive(Debug, Deserialize)]
pub struct DyeRecipeListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub recipe_no: Option<String>,
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub dye_type: Option<String>,
    pub status: Option<String>,
}

/// 审核请求体
#[derive(Debug, Deserialize)]
pub struct ApproveRecipeRequest {
    pub approved_by: i32,
}

/// 创建新版本请求体
#[derive(Debug, Deserialize)]
pub struct CreateVersionRequest {
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 从 AppState 构造 DyeRecipeService（每个请求构造轻量实例，无状态）
fn service(state: &AppState) -> DyeRecipeService {
    DyeRecipeService::new(state.db.clone())
}

pub async fn list_dye_recipes(
    State(state): State<AppState>,
    Query(query): Query<DyeRecipeListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<dye_recipe::Model>>>, AppError> {
    // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let svc_query = DyeRecipeQuery {
        recipe_no: query.recipe_no,
        color_code: query.color_code,
        color_name: query.color_name,
        dye_type: query.dye_type,
        status: query.status,
        page,
        page_size,
    };

    let (recipes, total) = service(&state).list(svc_query).await?;
    Ok(Json(ApiResponse::success_paginated(
        recipes, total, page, page_size,
    )))
}

pub async fn get_dye_recipe(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let recipe = service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(recipe)))
}

pub async fn create_dye_recipe(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateDyeRecipeRequest>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let created = service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "配方创建成功",
    )))
}

pub async fn update_dye_recipe(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<UpdateDyeRecipeRequest>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let updated = service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "配方更新成功",
    )))
}

pub async fn delete_dye_recipe(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "配方删除成功",
    )))
}

pub async fn approve_recipe(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<ApproveRecipeRequest>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let updated = service(&state).approve(id, req.approved_by).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "配方审核成功",
    )))
}

pub async fn create_new_version(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<CreateVersionRequest>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let created = service(&state)
        .create_new_version(id, req.remarks, req.created_by)
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "配方新版本创建成功",
    )))
}

pub async fn get_recipes_by_color(
    State(state): State<AppState>,
    Path(color_code): Path<String>,
) -> Result<Json<ApiResponse<Vec<dye_recipe::Model>>>, AppError> {
    let recipes = service(&state).get_recipes_by_color(&color_code).await?;
    Ok(Json(ApiResponse::success(recipes)))
}

pub async fn get_recipe_versions(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<dye_recipe::Model>>>, AppError> {
    let recipes = service(&state).get_recipe_versions(id).await?;
    Ok(Json(ApiResponse::success(recipes)))
}

/// POST /api/v1/erp/dye-recipes/:id/submit - 提交配方审核
///
/// 当前实现为轻量提交动作（仅刷新 updated_at + 标记 approved_by=-1 占位），
/// 状态保持草稿不变。批次 423B 化验室打样流程贯通时将重设计状态机，
/// 引入"待审核"中间态，由 service 层提供 submit 方法。
pub async fn submit_dye_recipe(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let recipe = service(&state).get_by_id(id).await?;
    // 校验：仅草稿状态可提交
    DyeRecipeService::validate_can_approve(recipe.status.as_deref())?;

    // 当前仅记录提交动作占位，不修改状态（保留向后兼容）
    // TODO(批次 423B)：引入"待审核"中间态，submit 改为 DRAFT → PENDING_APPROVAL 状态转换
    let mut active: dye_recipe::ActiveModel = recipe.into();
    active.approved_by = Set(Some(-1));
    active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    let updated = active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "配方已提交审核",
    )))
}

/// GET /api/v1/erp/dye-recipes/export - 导出配方列表（xlsx）
///
/// 注意：该接口返回 xlsx 二进制流（非 JSON），无法套用 `Result<Json<ApiResponse<T>>, AppError>`。
/// 此处返回 `Result<axum::response::Response, AppError>`：成功时通过 build_xlsx_response 构造带
/// xlsx Content-Type 的 200 响应；失败时通过 `?` 将 `sea_orm::DbErr` 自动转换为 `AppError`。
pub async fn export_dye_recipes(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DyeRecipeListQuery>,
) -> Result<axum::response::Response, AppError> {
    // 导出全量数据（不分页），保留 handler 直接查询以避免 service 暴露过多内部 select
    let mut q = dye_recipe::Entity::find().filter(dye_recipe::Column::IsDeleted.eq(false));

    if let Some(recipe_no) = &query.recipe_no {
        q = q.filter(dye_recipe::Column::RecipeNo.contains(recipe_no));
    }
    if let Some(color_code) = &query.color_code {
        q = q.filter(dye_recipe::Column::ColorCode.contains(color_code));
    }
    if let Some(color_name) = &query.color_name {
        q = q.filter(dye_recipe::Column::ColorName.contains(color_name));
    }
    if let Some(dye_type) = &query.dye_type {
        q = q.filter(dye_recipe::Column::DyeType.eq(dye_type));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_recipe::Column::Status.eq(status));
    }

    q = q.order_by_desc(dye_recipe::Column::CreatedAt);

    let recipes = q.all(&*state.db).await?;

    let table = XlsxTable {
        sheet_name: "染色配方列表".to_string(),
        headers: vec![
            "ID".to_string(),
            "配方编号".to_string(),
            "配方名称".to_string(),
            "色号".to_string(),
            "颜色名称".to_string(),
            "布种".to_string(),
            "染料类型".to_string(),
            "温度".to_string(),
            "时间".to_string(),
            "PH值".to_string(),
            "浴比".to_string(),
            "状态".to_string(),
            "版本".to_string(),
        ],
        rows: recipes
            .iter()
            .map(|r| {
                vec![
                    r.id.to_string(),
                    r.recipe_no.clone(),
                    r.recipe_name.clone().unwrap_or_default(),
                    r.color_code.clone().unwrap_or_default(),
                    r.color_name.clone().unwrap_or_default(),
                    r.fabric_type.clone().unwrap_or_default(),
                    r.dye_type.clone().unwrap_or_default(),
                    r.temperature.map(|d| d.to_string()).unwrap_or_default(),
                    r.time_minutes.map(|i| i.to_string()).unwrap_or_default(),
                    r.ph_value.map(|d| d.to_string()).unwrap_or_default(),
                    r.liquor_ratio.map(|d| d.to_string()).unwrap_or_default(),
                    r.status.clone().unwrap_or_default(),
                    r.version.map(|i| i.to_string()).unwrap_or_default(),
                ]
            })
            .collect(),
    };

    let row_count = recipes.len();

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("dye_recipe".to_string()),
        resource_id: None,
        resource_name: Some("dye_recipes_export.xlsx".to_string()),
        description: Some(format!(
            "用户 {} 导出染色配方列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/dye-recipes/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "recipe_no_filter": query.recipe_no,
            "color_code_filter": query.color_code,
            "color_name_filter": query.color_name,
            "dye_type_filter": query.dye_type,
            "status_filter": query.status,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    // 规则 3：导出统一使用 xlsx 格式，错误用 AppError 表达，成功返回 200 + xlsx 响应体
    build_xlsx_response(&table, "dye_recipes_export")
}
