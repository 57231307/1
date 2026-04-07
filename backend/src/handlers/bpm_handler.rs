use crate::models::dto::bpm_dto::{ApproveTaskRequest, StartProcessRequest, TaskQuery};
use crate::models::dto::ApiResponse;
use crate::services::bpm_service::BpmService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use axum::{
    extract::{Query, State},
    Json,
};

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
    Ok(Json(ApiResponse::success(
        "Task processed successfully".to_string(),
    )))
}

pub async fn query_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.query_user_tasks(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}
