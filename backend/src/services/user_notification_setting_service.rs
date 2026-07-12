//! 用户通知偏好设置服务

use crate::models::user_notification_setting::{self, notification_type};
use crate::utils::error::AppError;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

/// 更新通知偏好设置的参数对象
///
/// 批次 327 v10 复审 P3 修复：引入参数对象消除 too_many_arguments 警告
#[derive(Debug, Clone, Default)]
pub struct UpdateNotificationSettingParams {
    /// 是否启用邮件通知
    pub email_enabled: Option<bool>,
    /// 是否启用站内通知
    pub internal_enabled: Option<bool>,
    /// 订单通知类型
    pub order_type: Option<String>,
    /// 审批通知类型
    pub approval_type: Option<String>,
    /// 库存通知类型
    pub inventory_type: Option<String>,
    /// 采购通知类型
    pub purchase_type: Option<String>,
    /// 财务通知类型
    pub finance_type: Option<String>,
    /// 系统通知类型
    pub system_type: Option<String>,
}

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
    ///
    /// 批次 327 v10 复审 P3 修复：使用 UpdateNotificationSettingParams 参数对象替代 8 个独立参数
    pub async fn update_setting(
        &self,
        user_id: i32,
        params: UpdateNotificationSettingParams,
    ) -> Result<user_notification_setting::Model, AppError> {
        let setting = self.get_or_create_default(user_id).await?;
        let mut active_model: user_notification_setting::ActiveModel = setting.into();

        if let Some(v) = params.email_enabled {
            active_model.email_enabled = Set(v);
        }
        if let Some(v) = params.internal_enabled {
            active_model.internal_enabled = Set(v);
        }
        if let Some(v) = params.order_type {
            active_model.order_notification_type = Set(v);
        }
        if let Some(v) = params.approval_type {
            active_model.approval_notification_type = Set(v);
        }
        if let Some(v) = params.inventory_type {
            active_model.inventory_notification_type = Set(v);
        }
        if let Some(v) = params.purchase_type {
            active_model.purchase_notification_type = Set(v);
        }
        if let Some(v) = params.finance_type {
            active_model.finance_notification_type = Set(v);
        }
        if let Some(v) = params.system_type {
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

        // 批次 342 v11 复审 P2 修复：显式检查 NONE 类型，通知类型为 none 时不发送任何通知
        if notification_type_str == notification_type::NONE {
            return Ok(false);
        }

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

        // 批次 342 v11 复审 P2 修复：显式检查 NONE 类型，通知类型为 none 时不发送任何通知
        if notification_type_str == notification_type::NONE {
            return Ok(false);
        }

        Ok(notification_type_str == notification_type::INTERNAL
            || notification_type_str == notification_type::BOTH)
    }

    /// 批量获取或创建默认设置（避免循环内逐个查询 N+1）
    ///
    /// v16 批次 45 新增：一次 IN 查询获取所有已存在的设置，对缺失的用户逐个创建默认记录。
    /// 查询部分批量化，创建部分仅对缺失项执行。
    pub async fn get_or_create_default_batch(
        &self,
        user_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, user_notification_setting::Model>, AppError> {
        if user_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        // 批量查询已存在的设置
        let existing = user_notification_setting::Entity::find()
            .filter(user_notification_setting::Column::UserId.is_in(user_ids.to_vec()))
            .all(&*self.db)
            .await?;
        let mut map: std::collections::HashMap<i32, user_notification_setting::Model> = existing
            .into_iter()
            .map(|s| (s.user_id, s))
            .collect();
        // 为缺失的用户创建默认设置
        use std::collections::hash_map::Entry;
        let now = chrono::Utc::now();
        for &user_id in user_ids {
            if let Entry::Vacant(e) = map.entry(user_id) {
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
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                };
                let model = active_model.insert(&*self.db).await?;
                e.insert(model);
            }
        }
        Ok(map)
    }

    /// 纯函数：基于已查询的设置判断是否发送站内通知（不查数据库）
    ///
    /// v16 批次 45 新增：与 should_send_internal 逻辑一致，但接收已查询的 setting model，
    /// 避免在循环内重复查询数据库。
    pub fn should_send_internal_from_setting(
        setting: &user_notification_setting::Model,
        notification_category: &str,
    ) -> bool {
        if !setting.internal_enabled {
            return false;
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
        notification_type_str == notification_type::INTERNAL
            || notification_type_str == notification_type::BOTH
    }

    /// 纯函数：基于已查询的设置判断是否发送邮件通知（不查数据库）
    ///
    /// v17 批次 47 新增：与 should_send_email 逻辑一致，但接收已查询的 setting model，
    /// 避免在循环内重复查询数据库。
    pub fn should_send_email_from_setting(
        setting: &user_notification_setting::Model,
        notification_category: &str,
    ) -> bool {
        if !setting.email_enabled {
            return false;
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
        notification_type_str == notification_type::EMAIL
            || notification_type_str == notification_type::BOTH
    }
}
