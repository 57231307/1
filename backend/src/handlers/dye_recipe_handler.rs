//! 染色配方管理 Handler
//!
//! v14 批次 423A 重构：从直接 ActiveModel 操作改为调用 DyeRecipeService 抽象层，
//! 状态字符串统一引用 status::dye_recipe 常量，业务逻辑下沉到 service 便于单元测试。
//! 依据：面料行业真实业务调研文档 §11.1 化验室打样流程 + §13.1 批次 423 规划

use axum::{
    Json,
    extract::{Path, Query, State},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::models::dye_recipe;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::dye_recipe_service::{
    CreateDyeRecipeRequest, DyeRecipeQuery, DyeRecipeService, UpdateDyeRecipeRequest,
};
use crate::utils::data_scope::{DataScopeContext, apply_data_scope};
use crate::utils::error::AppError;
use crate::utils::optional_json::OptionalJson;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use crate::utils::xlsx_export::{XlsxTable, build_xlsx_response};
use std::sync::Arc;

/// 列表查询参数（axum Query 反序列化用）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct DyeRecipeListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub recipe_no: Option<String>,
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub dye_type: Option<String>,
    pub status: Option<String>,
    /// 敏感导出 fail-closed：导出审批令牌
    pub download_token: Option<String>,
}

/// 创建新版本请求体（仅 remarks 选填；建版人身份取 AuthContext.user_id，不由请求体提供）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateVersionRequest {
    pub remarks: Option<String>,
}

/// 拒绝配方请求体：拒绝理由 reason 必填（服务端 trim 非空强制），身份不由请求体承载
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct RejectRecipeRequest {
    pub reason: String,
}

/// 从 AppState 构造 DyeRecipeService（每个请求构造轻量实例，无状态）
fn service(state: &AppState) -> DyeRecipeService {
    DyeRecipeService::new(state.db.clone())
}

pub async fn list_dye_recipes(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DyeRecipeListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<dye_recipe::Model>>>, AppError> {
    // 分页 clamp 防 DoS
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

    // 行级数据权限：染色配方属工艺机密面（染化料、配比、温度曲线），主列表必须按持键用户的
    // data_scope 下推行级过滤，否则任意持读键用户可枚举全库配方。归属列 created_by、表无
    // department_id，service.list 内以 apply_data_scope(..., CreatedBy, CreatedBy) 下推，
    // total 由同一已过滤 paginator 派生（与列表同源）。auth 置于 Query 之前符合 axum
    // 提取器规则（仅最后一个 body 提取器受限，FromRequestParts 之间顺序自由）。
    let data_scope_ctx = auth.to_data_scope_context();
    let (recipes, total) = service(&state)
        .list(svc_query, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success_paginated(
        recipes, total, page, page_size,
    )))
}

pub async fn get_dye_recipe(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    // 详情按 created_by 归属门（check_resource_owner_by_member_scope，与主列表 apply_data_scope
    // Dept 分支同源判据）：All 放行 / Dept 本人或可见成员 / Self_ 仅本人 / NULL 历史行拒。
    // 若不加门，主列表已收窄的机密行仍可被直查 :id 读出，形成"列表看不到、详情能拿到"的绕行。
    let data_scope_ctx = auth.to_data_scope_context();
    let recipe = service(&state).get_by_id(id, Some(&data_scope_ctx)).await?;
    Ok(Json(ApiResponse::success(recipe)))
}

pub async fn create_dye_recipe(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateDyeRecipeRequest>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    // 建单人取服务端会话（AuthContext.user_id），请求体不承载身份。
    let created = service(&state).create(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "配方创建成功",
    )))
}

pub async fn update_dye_recipe(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateDyeRecipeRequest>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let service = service(&state);
    // 写端点归属门必须前置于落库：先按 created_by 校验对目标行的数据范围，越权 403，
    // 未通过绝不进入 service.update 写路径（防对他人机密配方做字段级篡改）。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;
    let updated = service.update(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "配方更新成功",
    )))
}

pub async fn delete_dye_recipe(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = service(&state);
    // 删除归属门前置于软删落库：按 created_by 校验数据范围，越权 403，不得删他人机密配方。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;
    service.delete(id).await?;
    Ok(Json(ApiResponse::success_with_message((), "配方删除成功")))
}

/// POST /api/v1/erp/production/dye-recipes/:id/approve - 审核配方
// 审批人身份唯一来源是服务端会话（AuthContext.user_id），请求体不承载审批身份。
// 端点无必填报文字段，故不绑定 body 提取器：缺体是合法输入，状态门在 service 层。
pub async fn approve_recipe(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let service = service(&state);
    // 审批类端点归属门前置于状态流转落库：按 created_by 校验数据范围，越权 403，
    // 不得审批他人机密配方（审批人身份仍取会话 auth.user_id，不由请求体承载）。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;
    let updated = service.approve(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "配方审核成功",
    )))
}

/// POST /api/v1/erp/production/dye-recipes/:id/reject - 拒绝配方（待审核 → 已拒绝）
// 拒绝人身份唯一来源是服务端会话（AuthContext.user_id），请求体只承载拒绝理由 reason
// （trim 非空必填）。归属门沿用 approve 的 handler 前置口径：先按 created_by 校验
// 数据范围（越权 403），未通过绝不进入 service.reject 写路径。
pub async fn reject_recipe(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<RejectRecipeRequest>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let reason = req.reason.trim().to_string();
    if reason.is_empty() {
        return Err(AppError::validation_displayable("审批拒绝理由不能为空"));
    }
    let service = service(&state);
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;
    let updated = service.reject(id, reason, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "配方拒绝成功",
    )))
}

/// POST /api/v1/erp/production/dye-recipes/:id/new-version - 基于已审核配方创建新版本
// 新版本行的建版人同样取会话；备注 remarks 选填 ⇒ 体本身选填（OptionalJson 语义表），
// 缺体合法放行到服务层状态门，不得在解码层被吞成 400。
pub async fn create_new_version(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    OptionalJson(req): OptionalJson<CreateVersionRequest>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let service = service(&state);
    // 建版（复制）类端点归属门前置于新行落库：对复制源父配方按 created_by 校验数据范围，
    // 越权 403，不得凭他人机密配方派生新版本。缺体仍合法放行到 service 层状态门（OptionalJson）。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;
    let created = service
        .create_new_version(id, req.and_then(|r| r.remarks), auth.user_id)
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "配方新版本创建成功",
    )))
}

/// 按色号查询参考配方（共享工艺参考面）——handler 刻意不注入 AuthContext：
/// service `get_recipes_by_color` 仅返回 status=APPROVED 且未删除的配方，属跨部门
/// 共享工艺知识（与主列表机密面、"我的排程运行"owner 私有面属不同判据族），
/// 调用方传 color_code，数据存 dye_recipe 表。
pub async fn get_recipes_by_color(
    State(state): State<AppState>,
    Path(color_code): Path<String>,
) -> Result<Json<ApiResponse<Vec<dye_recipe::Model>>>, AppError> {
    let recipes = service(&state).get_recipes_by_color(&color_code).await?;
    Ok(Json(ApiResponse::success(recipes)))
}

pub async fn get_recipe_versions(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<dye_recipe::Model>>>, AppError> {
    // 版本列表归属门：以父配方 id 的 created_by 归属门把关（service.get_recipe_versions 内部
    // 对 get_by_id(id, ctx) 走 check_resource_owner_by_member_scope）——与主列表同源：
    // 用户对 id 有读权限即可查看其版本树，否则 403。
    let data_scope_ctx = auth.to_data_scope_context();
    let recipes = service(&state)
        .get_recipe_versions(id, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(recipes)))
}

/// POST /api/v1/erp/dye-recipes/:id/submit - 提交配方审核；批次 423B 状态机贯通：DRAFT → PENDING_APPROVAL，
/// 化验室主管在待审核态执行审批（DRAFT 态仍保留直审兼容路径）。
pub async fn submit_dye_recipe(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_recipe::Model>>, AppError> {
    let service = service(&state);
    // 提交（状态流转写端点）归属门前置于落库：按 created_by 校验数据范围，越权 403，
    // 不得把他人机密配方提交进审批流。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;
    let updated = service.submit(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "配方已提交审核",
    )))
}

/// GET /api/v1/erp/dye-recipes/export - 导出配方列表（xlsx）；注意：该接口返回 xlsx 二进制流（非 JSON），无法套用 `Result<Json<ApiResponse<T>>, AppError>`。 此处返回
/// `Result<axum::response::Response, AppError>`：成功时通过 build_xlsx_response 构造带 xlsx Content-Type 的 200 响应；失败时通过 `?` 将 `sea_orm::DbErr` 自动转换为 `AppError`。
pub async fn export_dye_recipes(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DyeRecipeListQuery>,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：校验审批令牌（在 query 被 move 前提取）
    let download_token = query.download_token.clone();
    let approval =
        crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
            .enforce_export_download(download_token.as_deref(), "dye_recipe")
            .await?;

    // 导出与主列表必须同一 scope：此处复用 handler 的 AuthContext 构造 DataScopeContext，
    // 下推到导出查询，避免列表已收窄而导出仍全库落盘（xlsx 一次性外带工艺机密）。
    // auth 此前仅用于审计，现同时作为行级数据权限来源；审计记录仍按原口径写。
    let data_scope_ctx = auth.to_data_scope_context();

    // 导出全量数据（不分页），保留 handler 直接查询以避免 service 暴露过多内部 select
    let recipes = query_dye_recipes_for_export(&state.db, &query, Some(&data_scope_ctx)).await?;
    let row_count = recipes.len();

    let recipes_json: Vec<serde_json::Value> = recipes
        .into_iter()
        .map(|r| serde_json::to_value(r).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;
    let table = build_dye_recipes_table(&recipes_json);

    record_dye_recipes_export_audit(&state, &auth, row_count, &query);

    // 敏感导出 fail-closed：记录令牌消费
    let _ = crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
        .record_download(
            approval.id,
            "dye_recipes_export".to_string(),
            row_count as i64,
            String::new(),
        )
        .await;

    // 规则 3：导出统一使用 xlsx 格式，错误用 AppError 表达，成功返回 200 + xlsx 响应体
    build_xlsx_response(&table, "dye_recipes_export")
}

/// 按查询条件构建导出配方查询（V15 P0 9-2 修复：添加上限限制防止 OOM）
const EXPORT_LIMIT: u64 = 10_000;

async fn query_dye_recipes_for_export(
    db: &std::sync::Arc<sea_orm::DatabaseConnection>,
    query: &DyeRecipeListQuery,
    data_scope: Option<&DataScopeContext>,
) -> Result<Vec<dye_recipe::Model>, AppError> {
    let mut q = dye_recipe::Entity::find().filter(dye_recipe::Column::IsDeleted.eq(false));
    // 行级数据权限下推：与 service.list 完全同款——同一 apply_data_scope 入口、同一归属列
    // created_by、同传 CreatedBy 作 owner 与 dept 列（表无 department_id，dept 退化为可见成员集合）。
    // 保证导出可见集与主列表逐行一致，越权用户无法经导出旁路拿到全库机密配方。
    if let Some(ctx) = data_scope {
        q = apply_data_scope(
            q,
            ctx,
            dye_recipe::Column::CreatedBy,
            dye_recipe::Column::CreatedBy,
        );
    }
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
    q = q
        .order_by_desc(dye_recipe::Column::CreatedAt)
        .limit(EXPORT_LIMIT);
    Ok(q.all(db.as_ref()).await?)
}

/// 染色配方导出表头（13 列）
fn dye_recipes_export_headers() -> Vec<String> {
    vec![
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
    ]
}

/// 从 serde_json::Value 提取染色配方行数据
fn build_dye_recipe_row(r: &serde_json::Value) -> Vec<String> {
    let get_str = |key: &str| -> String {
        r.get(key)
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else if v.is_string() {
                    v.as_str().unwrap_or("").to_string()
                } else {
                    v.to_string()
                }
            })
            .unwrap_or_default()
    };
    vec![
        get_str("id"),
        get_str("recipe_no"),
        get_str("recipe_name"),
        get_str("color_code"),
        get_str("color_name"),
        get_str("fabric_type"),
        get_str("dye_type"),
        get_str("temperature"),
        get_str("time_minutes"),
        get_str("ph_value"),
        get_str("liquor_ratio"),
        get_str("status"),
        get_str("version"),
    ]
}

/// 构造染色配方列表 xlsx 表格
fn build_dye_recipes_table(recipes_json: &[serde_json::Value]) -> XlsxTable {
    XlsxTable {
        sheet_name: "染色配方列表".to_string(),
        headers: dye_recipes_export_headers(),
        rows: recipes_json.iter().map(build_dye_recipe_row).collect(),
    }
}

/// 异步记录染色配方导出操作（审计自身，best-effort 不阻塞响应）
fn record_dye_recipes_export_audit(
    state: &AppState,
    auth: &AuthContext,
    row_count: usize,
    query: &DyeRecipeListQuery,
) {
    // V15 P0-S11：导出审计日志写入
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
            "recipe_no_filter": query.recipe_no.clone(),
            "color_code_filter": query.color_code.clone(),
            "color_name_filter": query.color_name.clone(),
            "dye_type_filter": query.dye_type.clone(),
            "status_filter": query.status.clone(),
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
}
