//! 用户通知偏好设置 Handler

use crate::middleware::auth_context::AuthContext;
use crate::services::user_notification_setting_service::{
    UpdateNotificationSettingParams, UserNotificationSettingService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{extract::State, Json};
use serde::Deserialize;

/// 获取当前用户的通知偏好设置
pub async fn get_setting(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = UserNotificationSettingService::new(state.db.clone());
    let setting = service.get_or_create_default(auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(setting)?)))
}

/// 更新当前用户的通知偏好设置
#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub email_enabled: Option<bool>,
    pub internal_enabled: Option<bool>,
    pub order_notification_type: Option<String>,
    pub approval_notification_type: Option<String>,
    pub inventory_notification_type: Option<String>,
    pub purchase_notification_type: Option<String>,
    pub finance_notification_type: Option<String>,
    pub system_notification_type: Option<String>,
}

pub async fn update_setting(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = UserNotificationSettingService::new(state.db.clone());
    // 批次 327 v10 复审 P3 修复：使用参数对象替代多参数
    let params = UpdateNotificationSettingParams {
        email_enabled: req.email_enabled,
        internal_enabled: req.internal_enabled,
        order_type: req.order_notification_type,
        approval_type: req.approval_notification_type,
        inventory_type: req.inventory_notification_type,
        purchase_type: req.purchase_notification_type,
        finance_type: req.finance_notification_type,
        system_type: req.system_notification_type,
    };
    let setting = service.update_setting(auth.user_id, params).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(setting)?,
        "通知偏好设置已更新",
    )))
}
