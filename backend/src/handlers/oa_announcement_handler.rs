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
    Json,
    extract::{Path, Query, State},
};

use crate::container::AppState;
use crate::define_tuple_crud_handlers;
use crate::middleware::auth_context::AuthContext;
use crate::services::oa_announcement_service::{
    CreateOaAnnouncementRequest, OaAnnouncementQuery, OaAnnouncementService,
    UpdateOaAnnouncementRequest,
};
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
///
/// 发布时联动站内通知：根据 visibility_scope 解析目标用户，
/// 调用 EventNotificationService::send_system_announcement 批量推送。
/// event_notification_service 未配置时仅更新状态，不阻断发布流程。
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

    // 联动站内通知：解析目标用户并推送
    let notified_count = match &state.event_notification_service {
        Some(event_svc) => {
            match resolve_audience(state.db.as_ref(), &announcement).await {
                Ok(user_ids) if !user_ids.is_empty() => {
                    match event_svc
                        .send_system_announcement(
                            user_ids,
                            &announcement.title,
                            &announcement.content,
                        )
                        .await
                    {
                        Ok(()) => {
                            tracing::info!(
                                "OA 公告 {} 已推送通知给 {} 位用户",
                                announcement.id,
                                user_ids.len()
                            );
                            user_ids.len()
                        }
                        Err(e) => {
                            tracing::warn!(
                                "OA 公告 {} 通知推送失败（不阻断发布）: {}",
                                announcement.id,
                                e
                            );
                            0
                        }
                    }
                }
                Ok(_) => {
                    tracing::warn!("OA 公告 {} 目标用户为空，跳过通知推送", announcement.id);
                    0
                }
                Err(e) => {
                    tracing::warn!(
                        "OA 公告 {} 目标用户解析失败（不阻断发布）: {}",
                        announcement.id,
                        e
                    );
                    0
                }
            }
        }
        None => {
            tracing::warn!("event_notification_service 未配置，OA 公告 {} 跳过通知推送", announcement.id);
            0
        }
    };

    let mut result = serde_json::to_value(&announcement)?;
    if let Some(obj) = result.as_object_mut() {
        obj.insert("notified_count".to_string(), serde_json::json!(notified_count));
    }

    Ok(Json(ApiResponse::success_with_message(
        result,
        "公告已发布",
    )))
}

/// 根据 OA 公告的 visibility_scope 解析目标用户 id 列表
async fn resolve_audience(
    db: &sea_orm::DatabaseConnection,
    announcement: &crate::models::oa_announcement::Model,
) -> Result<Vec<i32>, AppError> {
    use crate::models::user;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    match announcement.visibility_scope.as_str() {
        "ALL" => {
            let users = user::Entity::find().all(db).await?;
            Ok(users.into_iter().map(|u| u.id).collect())
        }
        "CUSTOM" => {
            let cfg = announcement.visible_scope_config.as_ref().ok_or_else(|| {
                AppError::internal("visibility_scope=CUSTOM 但 visible_scope_config 为空")
            })?;
            let ids: Vec<i32> = cfg
                .get("user_ids")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_i64().map(|n| n as i32))
                        .collect()
                })
                .unwrap_or_default();
            Ok(ids)
        }
        "DEPT" => {
            let cfg = announcement.visible_scope_config.as_ref().ok_or_else(|| {
                AppError::internal("visibility_scope=DEPT 但 visible_scope_config 为空")
            })?;
            let dept_ids: Vec<i32> = cfg
                .get("department_ids")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_i64().map(|n| n as i32))
                        .collect()
                })
                .unwrap_or_default();
            if dept_ids.is_empty() {
                return Ok(vec![]);
            }
            // 查这些部门下的用户（user.department_id IN dept_ids）
            let users = user::Entity::find()
                .filter(user::Column::DepartmentId.is_in(dept_ids))
                .all(db)
                .await?;
            Ok(users.into_iter().map(|u| u.id).collect())
        }
        "ROLE" => {
            let cfg = announcement.visible_scope_config.as_ref().ok_or_else(|| {
                AppError::internal("visibility_scope=ROLE 但 visible_scope_config 为空")
            })?;
            let role_ids: Vec<i32> = cfg
                .get("role_ids")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_i64().map(|n| n as i32))
                        .collect()
                })
                .unwrap_or_default();
            if role_ids.is_empty() {
                return Ok(vec![]);
            }
            let users = user::Entity::find()
                .filter(user::Column::RoleId.is_in(role_ids))
                .all(db)
                .await?;
            Ok(users.into_iter().map(|u| u.id).collect())
        }
        _ => Ok(vec![]),
    }
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
