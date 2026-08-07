use axum::{
    extract::{Query, State},
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::permission_change_audit::{self, Entity as PermissionChangeAuditEntity};
use crate::utils::admin_checker::is_admin_role;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct PermissionAuditQuery {
    pub change_type: Option<String>,
    pub operator_id: Option<i32>,
    pub role_id: Option<i32>,
    pub user_id: Option<i32>,
    pub resource_type: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// B12-P2-3：查询权限变更审计日志列表（仅 admin 可访问）
pub async fn list_permission_audits(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<PermissionAuditQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 权限校验：仅 admin
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("无角色"))?;
    if !is_admin_role(&state.db, role_id).await {
        return Err(AppError::permission_denied(
            "仅管理员可查询权限变更审计日志",
        ));
    }

    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut finder = PermissionChangeAuditEntity::find();

    if let Some(ct) = &query.change_type {
        finder = finder.filter(permission_change_audit::Column::ChangeType.eq(ct.as_str()));
    }
    if let Some(oid) = query.operator_id {
        finder = finder.filter(permission_change_audit::Column::OperatorId.eq(oid));
    }
    if let Some(rid) = query.role_id {
        finder = finder.filter(permission_change_audit::Column::RoleId.eq(rid));
    }
    if let Some(uid) = query.user_id {
        finder = finder.filter(permission_change_audit::Column::UserId.eq(uid));
    }
    if let Some(rt) = &query.resource_type {
        finder = finder.filter(permission_change_audit::Column::ResourceType.eq(rt.as_str()));
    }
    if let Some(start) = query.start_date {
        let start_dt = start
            .and_hms_opt(0, 0, 0)
            .unwrap_or_default()
            .and_utc();
        finder = finder.filter(permission_change_audit::Column::ChangedAt.gte(start_dt));
    }
    if let Some(end) = query.end_date {
        let end_dt = (end + chrono::Duration::days(1))
            .and_hms_opt(0, 0, 0)
            .unwrap_or_default()
            .and_utc();
        finder = finder.filter(permission_change_audit::Column::ChangedAt.lt(end_dt));
    }

    let paginator = finder
        .order_by_desc(permission_change_audit::Column::ChangedAt)
        .paginate(&*state.db, page_size);

    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(page - 1).await?;

    let result = serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    });
    Ok(Json(ApiResponse::success(result)))
}
