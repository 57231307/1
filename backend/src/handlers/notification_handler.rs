//! 通知消息 Handler
//!
//! 通知消息 HTTP 接口层，负责处理通知相关的 HTTP 请求

use crate::middleware::auth_context::AuthContext;
use crate::models::notification::{NotificationStatus, NotificationType};
use crate::services::notification_service::NotificationService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

/// 通知列表查询参数
#[derive(Debug, Deserialize)]
pub struct NotificationListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub notification_type: Option<String>,
}

/// 批量操作请求
#[derive(Debug, Deserialize)]
pub struct BatchOperationRequest {
    pub ids: Vec<i32>,
}

/// 通知设置请求
#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub business_type: String,
    pub enable_internal: bool,
    pub enable_email: bool,
    pub enable_sms: bool,
}

/// 获取用户通知列表
pub async fn list_notifications(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<NotificationListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = NotificationService::new(state.db.clone());

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let status = query.status.and_then(|s| match s.as_str() {
        "UNREAD" => Some(NotificationStatus::Unread),
        "READ" => Some(NotificationStatus::Read),
        "PROCESSED" => Some(NotificationStatus::Processed),
        _ => None,
    });

    let notification_type = query.notification_type.and_then(|t| match t.as_str() {
        "INTERNAL" => Some(NotificationType::Internal),
        "EMAIL" => Some(NotificationType::Email),
        "SMS" => Some(NotificationType::Sms),
        "SYSTEM" => Some(NotificationType::System),
        _ => None,
    });

    let (notifications, total) = service
        .list_user_notifications(auth.user_id, status, notification_type, page, page_size)
        .await?;

    let result = serde_json::json!({
        "list": notifications,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取用户未读通知数量
pub async fn get_unread_count(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<i64>>, AppError> {
    let service = NotificationService::new(state.db.clone());
    let count = service.get_unread_count(auth.user_id).await?;

    Ok(Json(ApiResponse::success(count)))
}

/// 标记通知为已读
pub async fn mark_as_read(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = NotificationService::new(state.db.clone());
    service.mark_as_read(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message((), "标记已读成功")))
}

/// 批量标记通知为已读
pub async fn batch_mark_as_read(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<BatchOperationRequest>,
) -> Result<Json<ApiResponse<usize>>, AppError> {
    let service = NotificationService::new(state.db.clone());
    let count = service.batch_mark_as_read(req.ids, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        count,
        &format!("成功标记 {} 条通知为已读", count),
    )))
}

/// 标记所有通知为已读
pub async fn mark_all_as_read(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<usize>>, AppError> {
    let service = NotificationService::new(state.db.clone());
    let count = service.mark_all_as_read(auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        count,
        &format!("成功标记 {} 条通知为已读", count),
    )))
}

/// 删除通知
pub async fn delete_notification(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = NotificationService::new(state.db.clone());
    service.delete_notification(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message((), "通知删除成功")))
}

/// 获取通知详情
pub async fn get_notification(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = NotificationService::new(state.db.clone());
    let notification = service.get_notification(id, auth.user_id).await?;

    // 自动标记为已读
    if notification.status == NotificationStatus::Unread {
        if let Err(e) = service.mark_as_read(id, auth.user_id).await {
            tracing::warn!("自动标记通知 {} 为已读失败: {}", id, e);
        }
    }

    Ok(Json(ApiResponse::success(serde_json::to_value(
        notification,
    )?)))
}

// ========== 通知设置接口 ==========

/// 获取用户通知设置
pub async fn get_settings(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let service = NotificationService::new(state.db.clone());
    let settings = service.get_user_settings(auth.user_id).await?;

    let settings_json: Vec<serde_json::Value> = settings
        .into_iter()
        .map(|s| serde_json::to_value(s).unwrap_or_default())
        .collect();

    Ok(Json(ApiResponse::success(settings_json)))
}

/// 更新通知设置
pub async fn update_setting(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = NotificationService::new(state.db.clone());
    let setting = service
        .update_setting(
            auth.user_id,
            req.business_type,
            req.enable_internal,
            req.enable_email,
            req.enable_sms,
        )
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(setting)?,
        "通知设置更新成功",
    )))
}
