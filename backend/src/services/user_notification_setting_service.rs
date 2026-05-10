//! 用户通知偏好设置服务

use crate::models::user_notification_setting::{self, notification_type};
use crate::utils::error::AppError;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

pub struct UserNotificationSettingService {
    db: Arc<DatabaseConnection>,
}

impl UserNotificationSettingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取或创建默认设置
    pub async fn get_or_create_default(
        &self,
        user_id: i32,
    ) -> Result<user_notification_setting::Model, AppError> {
        let setting = user_notification_setting::Entity::find()
            .filter(user_notification_setting::Column::UserId.eq(user_id))
            .one(&*self.db)
            .await?;

        if let Some(setting) = setting {
            Ok(setting)
        } else {
            // 创建默认设置
            let active_model = user_notification_setting::ActiveModel {
                user_id: Set(user_id),
                email_enabled: Set(true),
                internal_enabled: Set(true),
                order_notification_type: Set(notification_type::BOTH.to_string()),
                approval_notification_type: Set(notification_type::BOTH.to_string()),
                inventory_notification_type: Set(notification_type::BOTH.to_string()),
                purchase_notification_type: Set(notification_type::BOTH.to_string()),
                finance_notification_type: Set(notification_type::BOTH.to_string()),
                system_notification_type: Set(notification_type::INTERNAL.to_string()),
                created_at: Set(chrono::Utc::now()),
                updated_at: Set(chrono::Utc::now()),
                ..Default::default()
            };
            Ok(active_model.insert(&*self.db).await?)
        }
    }

    /// 更新设置
    #[allow(clippy::too_many_arguments)]
    pub async fn update_setting(
        &self,
        user_id: i32,
        email_enabled: Option<bool>,
        internal_enabled: Option<bool>,
        order_type: Option<String>,
        approval_type: Option<String>,
        inventory_type: Option<String>,
        purchase_type: Option<String>,
        finance_type: Option<String>,
        system_type: Option<String>,
    ) -> Result<user_notification_setting::Model, AppError> {
        let setting = self.get_or_create_default(user_id).await?;
        let mut active_model: user_notification_setting::ActiveModel = setting.into();

        if let Some(v) = email_enabled {
            active_model.email_enabled = Set(v);
        }
        if let Some(v) = internal_enabled {
            active_model.internal_enabled = Set(v);
        }
        if let Some(v) = order_type {
            active_model.order_notification_type = Set(v);
        }
        if let Some(v) = approval_type {
            active_model.approval_notification_type = Set(v);
        }
        if let Some(v) = inventory_type {
            active_model.inventory_notification_type = Set(v);
        }
        if let Some(v) = purchase_type {
            active_model.purchase_notification_type = Set(v);
        }
        if let Some(v) = finance_type {
            active_model.finance_notification_type = Set(v);
        }
        if let Some(v) = system_type {
            active_model.system_notification_type = Set(v);
        }
        active_model.updated_at = Set(chrono::Utc::now());

        Ok(active_model.update(&*self.db).await?)
    }

    /// 检查是否应该发送邮件通知
    pub async fn should_send_email(
        &self,
        user_id: i32,
        notification_category: &str,
    ) -> Result<bool, AppError> {
        let setting = self.get_or_create_default(user_id).await?;

        if !setting.email_enabled {
            return Ok(false);
        }

        let notification_type_str = match notification_category {
            "ORDER" => &setting.order_notification_type,
            "APPROVAL" => &setting.approval_notification_type,
            "INVENTORY" => &setting.inventory_notification_type,
            "PURCHASE" => &setting.purchase_notification_type,
            "FINANCE" => &setting.finance_notification_type,
            "SYSTEM" => &setting.system_notification_type,
            _ => notification_type::BOTH,
        };

        Ok(notification_type_str == notification_type::EMAIL
            || notification_type_str == notification_type::BOTH)
    }

    /// 检查是否应该发送站内通知
    pub async fn should_send_internal(
        &self,
        user_id: i32,
        notification_category: &str,
    ) -> Result<bool, AppError> {
        let setting = self.get_or_create_default(user_id).await?;

        if !setting.internal_enabled {
            return Ok(false);
        }

        let notification_type_str = match notification_category {
            "ORDER" => &setting.order_notification_type,
            "APPROVAL" => &setting.approval_notification_type,
            "INVENTORY" => &setting.inventory_notification_type,
            "PURCHASE" => &setting.purchase_notification_type,
            "FINANCE" => &setting.finance_notification_type,
            "SYSTEM" => &setting.system_notification_type,
            _ => notification_type::BOTH,
        };

        Ok(notification_type_str == notification_type::INTERNAL
            || notification_type_str == notification_type::BOTH)
    }
}
