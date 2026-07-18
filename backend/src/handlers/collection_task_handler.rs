//! 催收任务 Handler（V15 P0-B03 Batch 481 创建）
//!
//! 实现 8 个 HTTP 端点：
//!   - POST   /auto-generate          自动生成催收任务（按账龄扫描）
//!   - POST   /                        手动创建催收任务
//!   - GET    /                        任务列表
//!   - GET    /:id                     任务详情
//!   - POST   /:id/contact             记录催收结果
//!   - POST   /:id/reassign            重新分配
//!   - POST   /:id/cancel              取消任务

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::collection_task;
use crate::models::collection_task_dto::{
    AutoGenerateTasksRequest, CancelTaskRequest, CreateTaskRequest, ListTaskQuery,
    RecordContactRequest, ReassignTaskRequest,
};
use crate::services::collection_task_service::{CollectionTaskError, CollectionTaskService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ==================== 响应 DTO ====================

/// 催收任务响应
#[derive(Debug, Serialize, Clone)]
pub struct TaskInfo {
    pub id: i64,
    pub task_no: String,
    pub customer_id: i64,
    pub ar_invoice_id: Option<i32>,
    pub overdue_amount: Decimal,
    pub overdue_days: i32,
    pub task_type: String,
    pub priority: String,
    pub due_date: NaiveDate,
    pub assigned_to: i32,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Option<i32>,
    pub status: String,
    pub contact_result: Option<String>,
    pub contact_at: Option<DateTime<Utc>>,
    pub next_action_date: Option<NaiveDate>,
    pub next_action_type: Option<String>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<collection_task::Model> for TaskInfo {
    fn from(m: collection_task::Model) -> Self {
        Self {
            id: m.id,
            task_no: m.task_no,
            customer_id: m.customer_id,
            ar_invoice_id: m.ar_invoice_id,
            overdue_amount: m.overdue_amount,
            overdue_days: m.overdue_days,
            task_type: m.task_type,
            priority: m.priority,
            due_date: m.due_date,
            assigned_to: m.assigned_to,
            assigned_at: m.assigned_at,
            assigned_by: m.assigned_by,
            status: m.status,
            contact_result: m.contact_result,
            contact_at: m.contact_at,
            next_action_date: m.next_action_date,
            next_action_type: m.next_action_type,
            remark: m.remark,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// 分页响应
#[derive(Debug, Serialize, Clone)]
pub struct PagedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 批量生成响应
#[derive(Debug, Serialize, Clone)]
pub struct AutoGenerateResponse {
    pub created: Vec<TaskInfo>,
    pub created_count: usize,
}

/// CollectionTaskError → AppError
pub fn collection_task_err(e: CollectionTaskError) -> AppError {
    match e {
        CollectionTaskError::NotFound => AppError::not_found("催收任务不存在"),
        CollectionTaskError::InvalidState { current, expected } => AppError::business(format!(
            "当前状态 {} 不允许此操作（期望 {}）",
            current, expected
        )),
        CollectionTaskError::Validation(msg) => AppError::validation(msg),
        CollectionTaskError::Database(e) => AppError::database(e.to_string()),
        // paginate_with_total 返回的 AppError 直接透传
        CollectionTaskError::App(e) => e,
    }
}

// ==================== Handler 端点 ====================

/// POST /api/v1/erp/collection-tasks/auto-generate - 自动生成催收任务
pub async fn auto_generate(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<AutoGenerateTasksRequest>,
) -> Result<Json<ApiResponse<AutoGenerateResponse>>, AppError> {
    let service = CollectionTaskService::from_state(&state);
    let created = service
        .auto_generate_tasks(req, auth.user_id)
        .await
        .map_err(collection_task_err)?;
    let count = created.len();
    let infos: Vec<TaskInfo> = created.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(AutoGenerateResponse {
        created: infos,
        created_count: count,
    })))
}

/// POST /api/v1/erp/collection-tasks - 手动创建催收任务
pub async fn create_task(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<ApiResponse<TaskInfo>>, AppError> {
    let service = CollectionTaskService::from_state(&state);
    let record = service
        .create_task(req, auth.user_id)
        .await
        .map_err(collection_task_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// GET /api/v1/erp/collection-tasks - 任务列表
pub async fn list_tasks(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListTaskQuery>,
) -> Result<Json<ApiResponse<PagedResponse<TaskInfo>>>, AppError> {
    let service = CollectionTaskService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let (items, total) = service
        .list_tasks(query)
        .await
        .map_err(collection_task_err)?;
    let infos: Vec<TaskInfo> = items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/collection-tasks/:id - 任务详情
pub async fn get_task(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<TaskInfo>>, AppError> {
    let service = CollectionTaskService::from_state(&state);
    let record = service.get_task(id).await.map_err(collection_task_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/collection-tasks/:id/contact - 记录催收结果
pub async fn record_contact(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<RecordContactRequest>,
) -> Result<Json<ApiResponse<TaskInfo>>, AppError> {
    let service = CollectionTaskService::from_state(&state);
    let record = service
        .record_contact(id, req)
        .await
        .map_err(collection_task_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/collection-tasks/:id/reassign - 重新分配
pub async fn reassign(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ReassignTaskRequest>,
) -> Result<Json<ApiResponse<TaskInfo>>, AppError> {
    let service = CollectionTaskService::from_state(&state);
    let record = service
        .reassign(id, req)
        .await
        .map_err(collection_task_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// POST /api/v1/erp/collection-tasks/:id/cancel - 取消任务
pub async fn cancel_task(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<CancelTaskRequest>,
) -> Result<Json<ApiResponse<TaskInfo>>, AppError> {
    let service = CollectionTaskService::from_state(&state);
    let record = service
        .cancel(id, req)
        .await
        .map_err(collection_task_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}
