//! 合同模板管理面 handler
//!
//! 功能 合同模板体系 HTTP 出口。读面 模板分页列表与模板详情（详情带条款项数组与
//!   条款库引用正文）；写面 新建草稿、表头更新（仅草稿）、改版（版本递增出新草稿）、
//!   法务复核通过、置启用、置停用、删除（仅未被快照引用的草稿），条款项的增改删与
//!   逐条复核。
//! 调用方 `routes/contract_template.rs` 挂载；权限键由权限中间件按路径派生为
//!   `contract-templates` 精确资源（read/create/update/delete/approve 五个动作），
//!   资源名单点登记于 `services/init_service.rs::PERMISSION_RESOURCES` 与
//!   `utils/path_utils.rs` 直接资源表。
//! 入参 下列各 DTO；出参 统一 `ApiResponse` 信封。
//! 传给谁 全部读写委托 `services/contract_template_service.rs`，本层只做参数整形、
//!   校验与出参装配，不写 SQL、不判状态机。
//! 存什么 不直接存；落库内容由服务层写 `contract_templates` 与 `contract_template_clauses`。
//! 存哪里 见服务层。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::contract_template_service::{
    ContractTemplateService, CreateTemplateRequest,
    ReviewClauseRequest as ServiceReviewClauseRequest, TemplateListQuery,
    UpdateTemplateHeaderRequest, UpsertClauseRequest,
};
use crate::utils::ApiResponse;
use crate::utils::error::AppError;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use validator::Validate;

/// 列表查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct TemplateListQueryDto {
    pub template_code: Option<String>,
    pub contract_type: Option<String>,
    pub party_side: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 新建模板请求 DTO。template_code 必填；改版走 /revise 不在创建口覆盖既有版本。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTemplateDto {
    #[validate(length(min = 1, max = 64, message = "模板编码长度须在1到64字符之间"))]
    pub template_code: String,
    #[validate(length(min = 1, max = 200, message = "模板标题长度须在1到200字符之间"))]
    pub title: String,
    pub contract_type: String,
    pub party_side: String,
    pub effective_from: Option<chrono::NaiveDate>,
    #[validate(length(max = 2000, message = "备注长度不能超过2000字符"))]
    pub remark: Option<String>,
    #[validate(nested)]
    pub clauses: Option<Vec<UpsertClauseDto>>,
}

/// 条款项写入请求 DTO（新增与更新共用一个形状）
#[derive(Debug, Deserialize, Validate)]
pub struct UpsertClauseDto {
    #[validate(length(min = 1, max = 64, message = "条款码长度须在1到64字符之间"))]
    pub clause_code: String,
    #[validate(length(min = 1, max = 200, message = "条款标题长度须在1到200字符之间"))]
    pub title: String,
    pub body: Option<String>,
    pub library_clause_id: Option<i32>,
    pub required: Option<bool>,
    pub sort: i16,
    pub applies_when: Option<String>,
}

/// 表头更新请求 DTO：三态语义（键缺席保持、显式 null 清空可空列、有值覆盖）。
/// title 是 NOT NULL 列，显式 null 由服务层拒绝。
#[derive(Debug, Deserialize)]
pub struct UpdateTemplateHeaderDto {
    #[serde(default, deserialize_with = "double_option")]
    pub title: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub effective_from: Option<Option<chrono::NaiveDate>>,
    #[serde(default, deserialize_with = "double_option")]
    pub remark: Option<Option<String>>,
}

/// 逐条复核请求 DTO
#[derive(Debug, Deserialize)]
pub struct ReviewClauseDto {
    pub legal_review_status: String,
    pub legal_review_remark: Option<String>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
/// 与其他域 handler 内同名私有适配器形状一致，跨域上收共享工具需动 utils，超出本域授权文件。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// GET /api/v1/erp/contract-templates - 模板分页列表
pub async fn list_templates(
    Query(params): Query<TemplateListQueryDto>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::info!("用户 {} 正在查询合同模板列表", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    let query = TemplateListQuery {
        template_code: params.template_code,
        contract_type: params.contract_type,
        party_side: params.party_side,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };
    let page = query.page;
    let page_size = query.page_size;
    let (rows, total) = service.list(query).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": rows,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// GET /api/v1/erp/contract-templates/{id} - 模板详情（带条款项数组）
pub async fn get_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::info!("用户 {} 正在查询合同模板详情 id={id}", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    let template = service.get_by_id(id).await?;
    let clauses = service.list_clause_views(id).await?;
    let mut value = serde_json::to_value(template).map_err(AppError::from)?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "clauses".to_string(),
            serde_json::to_value(clauses).map_err(AppError::from)?,
        );
    }
    Ok(Json(ApiResponse::success(value)))
}

/// POST /api/v1/erp/contract-templates - 新建模板草稿（v1）
pub async fn create_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateTemplateDto>,
) -> Result<Json<ApiResponse<crate::models::contract_template::Model>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;
    tracing::info!(
        "用户 {} 正在新建合同模板 {}",
        auth.user_id,
        req.template_code
    );
    let service = ContractTemplateService::new(state.db.clone());
    let create_req = CreateTemplateRequest {
        template_code: req.template_code,
        contract_type: req.contract_type,
        party_side: req.party_side,
        title: req.title,
        effective_from: req.effective_from,
        remark: req.remark,
        clauses: req.clauses.map(|items| {
            items
                .into_iter()
                .map(|c| UpsertClauseRequest {
                    clause_code: c.clause_code,
                    title: c.title,
                    body: c.body,
                    library_clause_id: c.library_clause_id,
                    required: c.required,
                    sort: c.sort,
                    applies_when: c.applies_when,
                })
                .collect()
        }),
    };
    let created = service.create(create_req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(created)))
}

/// PUT /api/v1/erp/contract-templates/{id} - 更新草稿模板表头
pub async fn update_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateTemplateHeaderDto>,
) -> Result<Json<ApiResponse<crate::models::contract_template::Model>>, AppError> {
    tracing::info!("用户 {} 正在更新合同模板 id={id}", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    let updated = service
        .update_header(
            id,
            UpdateTemplateHeaderRequest {
                title: req.title,
                effective_from: req.effective_from,
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// POST /api/v1/erp/contract-templates/{id}/revise - 改版，出新草稿版本
pub async fn revise_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::contract_template::Model>>, AppError> {
    tracing::info!("用户 {} 正在对合同模板 id={id} 改版", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    let created = service.revise(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(created)))
}

/// POST /api/v1/erp/contract-templates/{id}/approve - 记录法务复核通过
pub async fn approve_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::contract_template::Model>>, AppError> {
    tracing::info!(
        "用户 {} 正在为合同模板 id={id} 记录法务复核通过",
        auth.user_id
    );
    let service = ContractTemplateService::new(state.db.clone());
    let updated = service.approve(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// PUT /api/v1/erp/contract-templates/{id}/activate - 置启用（三道门见服务层）
pub async fn activate_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::contract_template::Model>>, AppError> {
    tracing::info!("用户 {} 正在启用合同模板 id={id}", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    let updated = service.activate(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// PUT /api/v1/erp/contract-templates/{id}/retire - 置停用（终态）
pub async fn retire_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::contract_template::Model>>, AppError> {
    tracing::info!("用户 {} 正在停用合同模板 id={id}", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    let updated = service.retire(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// DELETE /api/v1/erp/contract-templates/{id} - 删除草稿模板（被快照引用者拒绝）
pub async fn delete_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    tracing::info!("用户 {} 正在删除合同模板 id={id}", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    service.delete(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/erp/contract-templates/{id}/clauses - 模板条款项列表
pub async fn list_clauses(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<crate::services::contract_template_service::ClauseView>>>, AppError>
{
    tracing::info!("用户 {} 正在查询合同模板 {id} 的条款项", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    service.get_by_id(id).await?;
    let views = service.list_clause_views(id).await?;
    Ok(Json(ApiResponse::success(views)))
}

/// POST /api/v1/erp/contract-templates/{id}/clauses - 新增条款项（仅草稿模板）
pub async fn add_clause(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpsertClauseDto>,
) -> Result<Json<ApiResponse<crate::models::contract_template_clause::Model>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;
    tracing::info!("用户 {} 正在为合同模板 {id} 新增条款项", auth.user_id);
    let service = ContractTemplateService::new(state.db.clone());
    let created = service
        .add_clause(
            id,
            UpsertClauseRequest {
                clause_code: req.clause_code,
                title: req.title,
                body: req.body,
                library_clause_id: req.library_clause_id,
                required: req.required,
                sort: req.sort,
                applies_when: req.applies_when,
            },
            auth.user_id,
        )
        .await?;
    Ok(Json(ApiResponse::success(created)))
}

/// PUT /api/v1/erp/contract-templates/{id}/clauses/{clause_id} - 更新条款项（仅草稿模板）
pub async fn update_clause(
    Path((id, clause_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpsertClauseDto>,
) -> Result<Json<ApiResponse<crate::models::contract_template_clause::Model>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;
    tracing::info!(
        "用户 {} 正在更新合同模板 {id} 的条款项 {clause_id}",
        auth.user_id
    );
    let service = ContractTemplateService::new(state.db.clone());
    let updated = service
        .update_clause(
            id,
            clause_id,
            UpsertClauseRequest {
                clause_code: req.clause_code,
                title: req.title,
                body: req.body,
                library_clause_id: req.library_clause_id,
                required: req.required,
                sort: req.sort,
                applies_when: req.applies_when,
            },
            auth.user_id,
        )
        .await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// DELETE /api/v1/erp/contract-templates/{id}/clauses/{clause_id} - 删除条款项（仅草稿模板）
pub async fn delete_clause(
    Path((id, clause_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    tracing::info!(
        "用户 {} 正在删除合同模板 {id} 的条款项 {clause_id}",
        auth.user_id
    );
    let service = ContractTemplateService::new(state.db.clone());
    service.delete_clause(id, clause_id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// PUT /api/v1/erp/contract-templates/{id}/clauses/{clause_id}/review - 逐条法务复核
pub async fn review_clause(
    Path((id, clause_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ReviewClauseDto>,
) -> Result<Json<ApiResponse<crate::models::contract_template_clause::Model>>, AppError> {
    tracing::info!(
        "用户 {} 正在复核合同模板 {id} 的条款项 {clause_id}",
        auth.user_id
    );
    let service = ContractTemplateService::new(state.db.clone());
    let updated = service
        .review_clause(
            id,
            clause_id,
            ServiceReviewClauseRequest {
                legal_review_status: req.legal_review_status,
                legal_review_remark: req.legal_review_remark,
            },
            auth.user_id,
        )
        .await?;
    Ok(Json(ApiResponse::success(updated)))
}
