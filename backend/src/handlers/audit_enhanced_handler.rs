use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;

use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub table_name: Option<String>,
    pub action: Option<String>,
    pub user_id: Option<i32>,
    #[allow(dead_code)] // TODO(tech-debt): 审计模块接入业务后移除
    pub start_date: Option<String>,
    #[allow(dead_code)] // TODO(tech-debt): 审计模块接入业务后移除
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[allow(dead_code)] // TODO(tech-debt): 审计模块接入业务后移除
#[derive(Debug, Deserialize)]
pub struct OperationLogQuery {
    pub module: Option<String>,
    pub action: Option<String>,
    pub user_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogItem {
    pub id: i32,
    pub user_id: Option<i32>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub download_url: String,
    pub file_name: String,
    pub record_count: usize,
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    use crate::models::audit_log;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

    let mut query_builder = audit_log::Entity::find();

    if let Some(resource_type) = &query.table_name {
        query_builder =
            query_builder.filter(audit_log::Column::ResourceType.eq(resource_type.clone()));
    }
    if let Some(action) = &query.action {
        query_builder = query_builder.filter(audit_log::Column::Action.eq(action.clone()));
    }
    if let Some(user_id) = query.user_id {
        query_builder = query_builder.filter(audit_log::Column::UserId.eq(user_id));
    }

    let paginator = query_builder
        .order_by_desc(audit_log::Column::CreatedAt)
        .paginate(state.db.as_ref(), page_size);

    match paginator.num_items().await {
        Ok(total) => match paginator.fetch_page(page - 1).await {
            Ok(logs) => {
                let items: Vec<AuditLogItem> = logs
                    .into_iter()
                    .map(|m| AuditLogItem {
                        id: m.id,
                        user_id: m.user_id,
                        action: m.action,
                        resource_type: m.resource_type,
                        resource_id: m.resource_id,
                        ip_address: m.ip_address,
                        created_at: m.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                    })
                    .collect();

                Ok(Json(ApiResponse::success(serde_json::json!({
                    "list": items,
                    "total": total,
                    "page": page,
                    "page_size": page_size,
                }))))
            }
            Err(e) => {
                tracing::error!("查询审计日志失败: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Err(e) => {
            tracing::error!("统计审计日志失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn export_audit_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(_query): Query<AuditLogQuery>,
) -> Result<Json<ApiResponse<ExportResult>>, StatusCode> {
    use crate::models::audit_log;
    use sea_orm::{EntityTrait, QueryOrder};

    match audit_log::Entity::find()
        .order_by_desc(audit_log::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
    {
        Ok(logs) => {
            let count = logs.len();
            let file_name = format!(
                "audit_logs_{}.json",
                chrono::Utc::now().format("%Y%m%d%H%M%S")
            );

            Ok(Json(ApiResponse::success(ExportResult {
                download_url: format!("/api/v1/erp/downloads/{}", file_name),
                file_name,
                record_count: count,
            })))
        }
        Err(e) => {
            tracing::error!("导出审计日志失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
