use axum::{extract::{State, Query, Path}, Json};
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;
use crate::models::dto::bpm_dto::{StartProcessRequest, ApproveTaskRequest, TaskQuery};
use crate::services::bpm_service::BpmService;
use crate::utils::error::AppError;
use serde::Deserialize;

pub async fn start_process(
    State(state): State<AppState>,
    Json(req): Json<StartProcessRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.start_process(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

pub async fn approve_task(
    State(state): State<AppState>,
    Json(req): Json<ApproveTaskRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.approve_task(req).await?;
    Ok(Json(ApiResponse::success("Task processed successfully".to_string())))
}

pub async fn query_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.query_user_tasks(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// BPM business relation query params
#[derive(Debug, Deserialize)]
pub struct BusinessRelationQuery {
    pub business_type: String,
    pub business_id: i32,
}

/// Get BPM business relation
pub async fn get_business_relation(
    State(state): State<AppState>,
    Query(params): Query<BusinessRelationQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let relation = service.get_business_relation(&params.business_type, params.business_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(relation)?)))
}

/// Get BPM process visualization data
pub async fn get_process_visualization(
    Path(instance_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, QueryOrder};
    use crate::models::{bpm_process_instance, bpm_task, bpm_process_definition};

    let instance = bpm_process_instance::Entity::find_by_id(instance_id)
        .one(state.db.as_ref()).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Process instance not found".to_string()))?;

    let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
        .one(state.db.as_ref()).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let tasks = bpm_task::Entity::find()
        .filter(bpm_task::Column::ProcessInstanceId.eq(instance_id))
        .order_by_asc(bpm_task::Column::CreatedAt)
        .all(state.db.as_ref()).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let task_nodes: Vec<serde_json::Value> = tasks.into_iter().map(|t| {
        serde_json::json!({
            "id": t.id,
            "task_no": t.task_no,
            "node_id": t.node_id,
            "node_name": t.node_name,
            "status": t.status,
            "assignee_id": t.assignee_id,
            "created_at": t.created_at,
            "completed_at": t.completed_at,
            "comment": t.comment,
        })
    }).collect();

    let visualization = serde_json::json!({
        "instance": {
            "id": instance.id,
            "instance_no": instance.instance_no,
            "business_type": instance.business_type,
            "business_id": instance.business_id,
            "status": instance.status,
            "start_time": instance.start_time,
            "end_time": instance.end_time,
        },
        "definition": definition.map(|d| serde_json::json!({
            "id": d.id,
            "code": d.code,
            "name": d.name,
            "config": d.config,
        })),
        "tasks": task_nodes,
        "timeline": task_nodes,
    });

    Ok(Json(ApiResponse::success(visualization)))
}
