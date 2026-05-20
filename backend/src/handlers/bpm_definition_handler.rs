use axum::{extract::{State, Query, Path}, Json};
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;
use crate::models::dto::bpm_dto::{CreateProcessDefinitionRequest, UpdateProcessDefinitionRequest, ProcessDefinitionQuery, CreateVersionRequest, CreateTemplateRequest, TemplateQuery};
use crate::services::bpm_service::BpmService;
use crate::utils::error::AppError;
use serde::Deserialize;

/// 创建流程定义
pub async fn create_process_definition(
    State(state): State<AppState>,
    Json(req): Json<CreateProcessDefinitionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.create_process_definition(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 获取流程定义列表
pub async fn list_process_definitions(
    State(state): State<AppState>,
    Query(query): Query<ProcessDefinitionQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.list_process_definitions(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 获取单个流程定义
pub async fn get_process_definition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.get_process_definition(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 更新流程定义
pub async fn update_process_definition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProcessDefinitionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.update_process_definition(id, req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 删除流程定义
pub async fn delete_process_definition(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.delete_process_definition(id).await?;
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

/// 创建新版本
pub async fn create_version(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<CreateVersionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.create_process_version(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 获取版本列表
pub async fn list_versions(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.list_process_versions(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 激活指定版本
pub async fn activate_version(
    State(state): State<AppState>,
    Path((id, version)): Path<(i32, String)>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.activate_process_version(id).await?;
    Ok(Json(ApiResponse::success("版本激活成功".to_string())))
}

/// 保存为模板
pub async fn save_as_template(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.save_as_template(id, req.template_name).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({"message": "保存模板成功"}))))
}

/// 获取模板列表
pub async fn list_templates(
    State(state): State<AppState>,
    Query(query): Query<TemplateQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.list_templates(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 从模板创建流程定义
pub async fn create_from_template(
    State(state): State<AppState>,
    Path(template_id): Path<i32>,
    Json(req): Json<CreateProcessDefinitionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.create_from_template(template_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}