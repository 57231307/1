use crate::services::operation_log_service::OperationLogService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct ListLogsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub module: Option<String>,
    pub user_id: Option<i32>,
}

pub async fn list_logs(
    State(state): State<AppState>,
    Query(query): Query<ListLogsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OperationLogService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (logs, total) = if let Some(module) = query.module {
        service
            .list_logs_by_module(&module, page - 1, page_size)
            .await?
    } else if let Some(user_id) = query.user_id {
        service
            .list_logs_by_user(user_id, page - 1, page_size)
            .await?
    } else {
        service.list_logs(page - 1, page_size).await?
    };

    Ok(Json(ApiResponse::success(json!({
        "items": logs,
        "total": total,
        "page": page,
        "page_size": page_size
    }))))
}
