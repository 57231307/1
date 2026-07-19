//! OA 公告 Handler（P0-D17 / Batch 488）
//!
//! 通过 `define_tuple_crud_handlers!` 宏生成 5 个基础 CRUD：
//! list/get/create/update/delete。
//!
//! 额外手写 2 个状态转换端点：
//! - POST /:id/publish  发布（DRAFT → PUBLISHED）
//! - POST /:id/archive  归档（PUBLISHED → ARCHIVED）
//!
//! 路由前缀：/api/v1/erp/oa-announcements
//! 权限码：oa-announcements（init_service.rs 已注册 + admin_assistant 角色映射）

use axum::{
    extract::{Path, State},
    Json,
};

use crate::define_tuple_crud_handlers;
use crate::middleware::auth_context::AuthContext;
use crate::services::oa_announcement_service::{
    CreateOaAnnouncementRequest, OaAnnouncementQuery, OaAnnouncementService,
    UpdateOaAnnouncementRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// 5 个基础 CRUD handler（list/get/create/update/delete）
define_tuple_crud_handlers!(
    OaAnnouncementService,
    CreateOaAnnouncementRequest,
    UpdateOaAnnouncementRequest,
    OaAnnouncementQuery,
    i32,
    "公告不存在"
);

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
