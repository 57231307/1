use crate::models::dto::bpm_dto::{ApproveTaskRequest, StartProcessRequest, TaskQuery};
use crate::services::bpm_service::BpmService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
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
    auth: crate::middleware::auth_context::AuthContext,
    Json(req): Json<ApproveTaskRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    // P0 8-4 修复：传入真实认证用户 user_id 用于审计追溯（防止代审追溯丢失）
    service.approve_task(req, Some(auth.user_id)).await?;
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
    let relation = service
        .get_business_relation(&params.business_type, params.business_id)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(relation)?)))
}

/// Get BPM process visualization data
pub async fn get_process_visualization(
    Path(instance_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::{bpm_process_definition, bpm_process_instance, bpm_task};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let instance = bpm_process_instance::Entity::find_by_id(instance_id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found("流程实例不存在"))?;

    let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
        .one(state.db.as_ref())
        .await?;

    let tasks = bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(instance_id))
        .order_by_asc(bpm_task::Column::CreatedAt)
        .all(state.db.as_ref())
        .await?;

    let task_nodes: Vec<serde_json::Value> = tasks
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "task_no": t.task_no,
                "node_id": t.node_id,
                "node_name": t.node_name,
                "status": t.status,
                "assignee_id": t.actual_handler_id,
                "created_at": t.created_at,
                "completed_at": t.handled_at,
                "comment": t.approval_opinion,
            })
        })
        .collect();

    let visualization = serde_json::json!({
        "instance": {
            "id": instance.id,
            "instance_no": instance.instance_no,
            "business_type": instance.business_type,
            "business_id": instance.business_id,
            "status": instance.status,
            "start_time": instance.started_at,
            "end_time": instance.completed_at,
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

// ========== 审批链和监控接口 ==========

/// 获取流程实例审批链
pub async fn get_approval_chain(
    Path(instance_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let chain = service.get_approval_chain(instance_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(chain)?)))
}

/// 获取流程实例详情
pub async fn get_instance_detail(
    Path(instance_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let detail = service.get_instance_detail(instance_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(detail)?)))
}

/// 流程监控统计查询参数
#[derive(Debug, Deserialize)]
pub struct MonitorQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
}

/// 获取流程监控统计
pub async fn get_monitor_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let stats = service.get_monitor_stats().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(stats)?)))
}

/// 获取待处理任务列表（监控）
pub async fn get_pending_tasks_for_monitor(
    State(state): State<AppState>,
    Query(query): Query<MonitorQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let tasks = service
        .get_pending_tasks_for_monitor(page, page_size)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(tasks)?)))
}

/// 获取流程实例列表（监控）
pub async fn list_instances_for_monitor(
    State(state): State<AppState>,
    Query(query): Query<MonitorQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let instances = service
        .list_instances_for_monitor(query.status, page, page_size)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(instances)?)))
}

/// 转办任务请求
#[derive(Debug, Deserialize)]
pub struct TransferTaskRequest {
    pub new_assignee_id: i32,
    pub transfer_reason: String,
}

/// 转办任务
pub async fn transfer_task(
    Path(task_id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<TransferTaskRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service
        .transfer_task(task_id, req.new_assignee_id, &req.transfer_reason)
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        "任务转办成功".to_string(),
        "任务转办成功",
    )))
}

/// 催办任务请求
#[derive(Debug, Deserialize)]
pub struct UrgeTaskRequest {
    pub urge_message: String,
}

/// 催办任务
pub async fn urge_task(
    Path(task_id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<UrgeTaskRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.urge_task(task_id, &req.urge_message).await?;
    Ok(Json(ApiResponse::success_with_message(
        "催办成功".to_string(),
        "催办通知已发送",
    )))
}

/// 获取待办任务列表
pub async fn get_pending_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let query = TaskQuery {
        status: Some("PENDING".to_string()),
        ..query
    };
    let res = service.query_user_tasks(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 获取已完成任务列表
pub async fn get_completed_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let query = TaskQuery {
        status: Some("COMPLETED".to_string()),
        ..query
    };
    let res = service.query_user_tasks(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

/// 执行审批请求
#[derive(Debug, Deserialize)]
pub struct ExecuteApprovalRequest {
    pub task_id: i32,
    pub handler_id: i32,
    pub handler_name: String,
    pub action: String,
    pub approval_opinion: Option<String>,
}

/// 执行审批
pub async fn execute_approval(
    State(state): State<AppState>,
    auth: crate::middleware::auth_context::AuthContext,
    Json(req): Json<ExecuteApprovalRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let approve_req = crate::models::dto::bpm_dto::ApproveTaskRequest {
        task_id: req.task_id,
        handler_id: req.handler_id,
        handler_name: req.handler_name,
        action: req.action,
        approval_opinion: req.approval_opinion,
        attachment_urls: None,
    };
    // P0 8-4 修复：传入真实认证用户 user_id 用于审计追溯（防止代审追溯丢失）
    service.approve_task(approve_req, Some(auth.user_id)).await?;
    Ok(Json(ApiResponse::success("审批执行成功".to_string())))
}
