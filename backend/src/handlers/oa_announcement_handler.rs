//! OA 公告 Handler（P0-D17 / Batch 488 + 缺陷 7.2 可见性过滤）
//!
//! 通过 `define_tuple_crud_handlers!` 宏生成 4 个基础 CRUD：
//! get/create/update/delete（list 因缺陷 7.2 需按 visibility_scope 过滤，手写覆盖）。
//!
//! 额外手写 2 个状态转换端点：
//! - POST /:id/publish  发布（DRAFT → PUBLISHED）
//! - POST /:id/archive  归档（PUBLISHED → ARCHIVED）
//!
//! 路由前缀：/api/v1/erp/oa-announcements
//! 权限码：oa-announcements（init_service.rs 已注册 + admin_assistant 角色映射）

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::define_tuple_crud_handlers;
use crate::middleware::auth_context::AuthContext;
use crate::services::oa_announcement_service::{
    CreateOaAnnouncementRequest, OaAnnouncementQuery, OaAnnouncementService,
    UpdateOaAnnouncementRequest,
};
use crate::container::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// 缺陷 7.2 修复：宏生成 CRUD 在私有模块内，避免 list 与本文件手写版本冲突
mod generated {
    use super::*;

    define_tuple_crud_handlers!(
        OaAnnouncementService,
        CreateOaAnnouncementRequest,
        UpdateOaAnnouncementRequest,
        OaAnnouncementQuery,
        i32,
        "公告不存在"
    );
}

// 重新导出 get/create/update/delete（list 由本文件手写覆盖）
// generated::list 重导出为 list_unfiltered 供内部使用，避免 dead_code 警告
#[allow(unused_imports)]
pub use generated::list as list_unfiltered;
pub use generated::{create, delete, get, update};

/// GET /api/v1/erp/oa-announcements - 列表（缺陷 7.2 修复：按 visibility_scope
/// 过滤）；调用 service.list_for_user 按 ALL/DEPT/ROLE/CUSTOM 范围过滤可见公告。
pub async fn list(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<OaAnnouncementQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OaAnnouncementService::new(state.db.clone());
    let (items, total) = service
        .list_for_user(params, auth.user_id, auth.department_id, auth.role_id)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
    }))))
}

/// POST /api/v1/erp/oa-announcements/:id/publish - 发布公告
pub async fn publish(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OaAnnouncementService::new(state.db.clone());
    let announcement = service.publish(id).await?;

    tracing::info!(
        "用户 {} 发布 OA 公告: id={}, title={}",
        auth.username,
        announcement.id,
        announcement.title
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(announcement)?,
        "公告已发布",
    )))
}

/// POST /api/v1/erp/oa-announcements/:id/archive - 归档公告
pub async fn archive(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = OaAnnouncementService::new(state.db.clone());
    let announcement = service.archive(id).await?;

    tracing::info!(
        "用户 {} 归档 OA 公告: id={}, title={}",
        auth.username,
        announcement.id,
        announcement.title
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(announcement)?,
        "公告已归档",
    )))
}
