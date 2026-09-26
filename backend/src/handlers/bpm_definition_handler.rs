//! BPM 流程定义/版本/模板管理 handler
//!
//! 批次 67（P1 1-2 修复）：原所有 handler 标注 `#[allow(dead_code)]` 因 stub 未实现。
//! 现 service 层已实现真实逻辑，移除 dead_code 标注并注册路由。
//!
//! 字段映射说明：
//! 后端 Model 字段（`code`/`name`/`config`）与前端 API 契约
//!（`process_key`/`process_name`/`nodes`）不一致，handler 层负责转换。
//! 列表承载键与全仓统一分页信封 `PaginatedResponse` 一致，为 `items`。

use crate::container::AppState;
use crate::models::bpm_process_definition;
use crate::models::dto::bpm_dto::{
    CreateBpmTemplateRequest, CreateProcessDefinitionRequest, CreateVersionRequest,
    ProcessDefinitionQuery, TemplateQuery, UpdateProcessDefinitionRequest,
};
use crate::services::bpm_service::BpmService;
use crate::utils::error::AppError;
use crate::utils::messages::biz_msg;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::{Value, json};

/// 将 Model 转换为前端期望的 JSON 格式；字段映射： - `code` → `process_key` - `name` →
/// `process_name` - `config` → 保留原字段，同时提取 `config.nodes` 为顶层 `nodes`
fn model_to_frontend_json(model: bpm_process_definition::Model) -> Value {
    let config = model.config.clone();
    let nodes = config.as_ref().and_then(|c| c.get("nodes")).cloned();
    json!({
        "id": model.id,
        "process_key": model.code,
        "process_name": model.name,
        "description": model.description,
        "category": model.category,
        "version": model.version,
        "status": model.status,
        "config": config,
        "nodes": nodes,
        "created_at": model.created_at,
        "updated_at": model.updated_at,
    })
}

/// 将 PaginatedResponse 转换为前端期望的分页格式：逐条经 `model_to_frontend_json`
/// 映射字段，列表承载键与统一分页信封一致为 `items`（total/page/page_size 原样透传）
fn page_to_frontend_json(page_resp: PaginatedResponse<bpm_process_definition::Model>) -> Value {
    let items: Vec<Value> = page_resp
        .items
        .into_iter()
        .map(model_to_frontend_json)
        .collect();
    json!({
        "items": items,
        "total": page_resp.total,
        "page": page_resp.page,
        "page_size": page_resp.page_size,
    })
}

/// 创建流程定义
pub async fn create_process_definition(
    State(state): State<AppState>,
    Json(req): Json<CreateProcessDefinitionRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.create_process_definition(req).await?;
    Ok(Json(ApiResponse::success(model_to_frontend_json(res))))
}

/// 获取流程定义列表
pub async fn list_process_definitions(
    State(state): State<AppState>,
    Query(query): Query<ProcessDefinitionQuery>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.list_process_definitions(query).await?;
    Ok(Json(ApiResponse::success(page_to_frontend_json(res))))
}

/// 获取单个流程定义
pub async fn get_process_definition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.get_process_definition(id).await?;
    match res {
        Some(model) => Ok(Json(ApiResponse::success(model_to_frontend_json(model)))),
        None => Err(AppError::not_found(format!("流程定义不存在: {}", id))),
    }
}

/// 更新流程定义
pub async fn update_process_definition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProcessDefinitionRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.update_process_definition(id, req).await?;
    Ok(Json(ApiResponse::success(model_to_frontend_json(res))))
}

/// 删除流程定义
pub async fn delete_process_definition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.delete_process_definition(id).await?;
    Ok(Json(ApiResponse::success(biz_msg::DELETE_OK.to_string())))
}

/// 创建新版本；按 definition_id 查询原定义，复制为新版本记录（同 code，新 version）
pub async fn create_version(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<CreateVersionRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    // 先查询原定义，获取 code 和 config 作为新版本基础
    let source = service.get_process_definition(id).await?;
    let source = source.ok_or_else(|| AppError::not_found(format!("流程定义不存在: {}", id)))?;

    // 构造新定义：同 code，新 version，若 req.config 为空则继承原 config
    let new_req = CreateProcessDefinitionRequest {
        name: format!("{}-v{}", source.name, req.version),
        code: source.code,
        description: req.description.or(source.description),
        category: source.category,
        version: Some(req.version),
        config: req.config.or(source.config),
        status: Some("DRAFT".to_string()),
    };
    let res = service.create_process_definition(new_req).await?;
    Ok(Json(ApiResponse::success(model_to_frontend_json(res))))
}

/// 获取版本列表
pub async fn list_versions(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.list_process_versions(id).await?;
    let list: Vec<Value> = res.into_iter().map(model_to_frontend_json).collect();
    Ok(Json(ApiResponse::success(json!(list))))
}

/// 激活指定版本
pub async fn activate_version(
    State(state): State<AppState>,
    Path((id, _version)): Path<(i32, String)>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.activate_process_version(id).await?;
    Ok(Json(ApiResponse::success("版本激活成功".to_string())))
}

/// 通过版本 ID 激活版本（简化路径别名）
/// 前端调用：`POST /bpm/versions/:version/activate`
pub async fn activate_version_by_id(
    State(state): State<AppState>,
    Path(version): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.activate_process_version(version).await?;
    Ok(Json(ApiResponse::success(model_to_frontend_json(res))))
}

/// 保存为模板
pub async fn save_as_template(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<CreateBpmTemplateRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.save_as_template(id, req.template_name).await?;
    Ok(Json(ApiResponse::success(
        json!({"message": "保存模板成功"}),
    )))
}

/// 获取模板列表
pub async fn list_templates(
    State(state): State<AppState>,
    Query(query): Query<TemplateQuery>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.list_templates(query).await?;
    Ok(Json(ApiResponse::success(page_to_frontend_json(res))))
}

/// 从模板创建流程定义；批次 199 P1-6 修复：原 handler 接收 `Json(_req)` 完全丢弃请求体， 客户端无法自定义新流程定义的
/// name/code/config 等字段。现真实接入 req， 由 service 层用 req 字段覆盖模板默认值（req 字段优先，未提供时回退模板值）。
pub async fn create_from_template(
    State(state): State<AppState>,
    Path(template_id): Path<i32>,
    Json(req): Json<CreateProcessDefinitionRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.create_from_template(template_id, req).await?;
    Ok(Json(ApiResponse::success(model_to_frontend_json(res))))
}

/// 模板分类标记（与 bpm_process_definition_service 的 TEMPLATE_CATEGORY 常量保持一致）
const BPM_TEMPLATE_CATEGORY: &str = "__TEMPLATE__";

/// GET /bpm/templates/{template_id} - 获取 BPM 模板详情
/// （对应前端 api/bpm-enhanced.ts getBpmTemplateById；仅返回 category=__TEMPLATE__ 的记录）
pub async fn get_template(
    State(state): State<AppState>,
    Path(template_id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.get_process_definition(template_id).await?;
    match res {
        Some(model) if model.category.as_deref() == Some(BPM_TEMPLATE_CATEGORY) => {
            Ok(Json(ApiResponse::success(model_to_frontend_json(model))))
        }
        _ => Err(AppError::not_found(format!(
            "流程模板不存在: {}",
            template_id
        ))),
    }
}

/// DELETE /bpm/templates/{template_id} - 删除 BPM 模板
/// （对应前端 api/bpm-enhanced.ts deleteBpmTemplate；校验记录为模板后复用流程定义删除逻辑）
pub async fn delete_template(
    State(state): State<AppState>,
    Path(template_id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.get_process_definition(template_id).await?;
    match res {
        Some(model) if model.category.as_deref() == Some(BPM_TEMPLATE_CATEGORY) => {
            service.delete_process_definition(template_id).await?;
            Ok(Json(ApiResponse::success(biz_msg::DELETE_OK.to_string())))
        }
        _ => Err(AppError::not_found(format!(
            "流程模板不存在: {}",
            template_id
        ))),
    }
}
