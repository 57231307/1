//! 通知服务
//!
//! 提供通知消息的创建、查询、更新、删除等功能
//! 支持站内信、邮件、短信等多种通知渠道

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::sync::Arc;

use crate::models::notification::{
    self, Entity as NotificationEntity, NotificationPriority, NotificationStatus, NotificationType,
};
use crate::models::notification_setting::{self, Entity as NotificationSettingEntity};
use crate::utils::error::AppError;

/// 创建通知请求
#[derive(Debug, Clone)]
pub struct CreateNotificationRequest {
    pub user_id: i32,
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub priority: NotificationPriority,
    pub business_type: Option<String>,
    pub business_id: Option<i32>,
    pub action_url: Option<String>,
    pub sender_id: Option<i32>,
    pub sender_name: Option<String>,
}

/// 通知服务
pub struct NotificationService {
    db: Arc<DatabaseConnection>,
}

impl NotificationService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建通知消息
    pub async fn create_notification(
        &self,
        req: CreateNotificationRequest,
    ) -> Result<notification::Model, AppError> {
        let active_model = notification::ActiveModel {
            id: Default::default(),
            user_id: Set(req.user_id),
            notification_type: Set(req.notification_type),
            title: Set(req.title),
            content: Set(req.content),
            priority: Set(req.priority),
            status: Set(NotificationStatus::Unread),
            business_type: Set(req.business_type),
            business_id: Set(req.business_id),
            action_url: Set(req.action_url),
            sender_id: Set(req.sender_id),
            sender_name: Set(req.sender_name),
            read_at: Set(None),
            processed_at: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let notification = active_model.insert(&*self.db).await?;
        Ok(notification)
    }

    /// 批量创建通知（优化性能）
    pub async fn batch_create_notifications(
        &self,
        requests: Vec<CreateNotificationRequest>,
    ) -> Result<Vec<notification::Model>, AppError> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        let now = Utc::now();
        let mut notifications = Vec::with_capacity(requests.len());

        for req in requests {
            let active_model = notification::ActiveModel {
                id: Default::default(),
                user_id: Set(req.user_id),
                notification_type: Set(req.notification_type),
                title: Set(req.title),
                content: Set(req.content),
                priority: Set(req.priority),
                status: Set(NotificationStatus::Unread),
                business_type: Set(req.business_type),
                business_id: Set(req.business_id),
                action_url: Set(req.action_url),
                sender_id: Set(req.sender_id),
                sender_name: Set(req.sender_name),
                read_at: Set(None),
                processed_at: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let notification = active_model.insert(self.db.as_ref()).await?;
            notifications.push(notification);
        }

        Ok(notifications)
    }

    /// 获取用户的通知列表
    pub async fn list_user_notifications(
        &self,
        user_id: i32,
        status: Option<NotificationStatus>,
        notification_type: Option<NotificationType>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<notification::Model>, u64), AppError> {
        let mut query = NotificationEntity::find()
            .filter(notification::Column::UserId.eq(user_id))
            .filter(notification::Column::Status.ne(NotificationStatus::Deleted));

        if let Some(s) = status {
            query = query.filter(notification::Column::Status.eq(s));
        }

        if let Some(t) = notification_type {
            query = query.filter(notification::Column::NotificationType.eq(t));
        }

        let total = query.clone().count(&*self.db).await?;

        let notifications = query
            .order_by(notification::Column::CreatedAt, Order::Desc)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        Ok((notifications, total))
    }

    /// 获取用户未读通知数量
    pub async fn get_unread_count(&self, user_id: i32) -> Result<i64, AppError> {
        let count = NotificationEntity::find()
            .filter(notification::Column::UserId.eq(user_id))
            .filter(notification::Column::Status.eq(NotificationStatus::Unread))
            .count(&*self.db)
            .await?;

        Ok(count as i64)
    }

    /// 标记通知为已读
    pub async fn mark_as_read(&self, notification_id: i32, user_id: i32) -> Result<(), AppError> {
        let notification = NotificationEntity::find_by_id(notification_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "通知 {} 不存在",
                notification_id
            )))?;

        if notification.user_id != user_id {
            return Err(AppError::PermissionDenied(
                "无权操作此通知".to_string(),
            ));
        }

        let mut active_model: notification::ActiveModel = notification.into();
        active_model.status = Set(NotificationStatus::Read);
        active_model.read_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());
        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 批量标记通知为已读
    pub async fn batch_mark_as_read(
        &self,
        notification_ids: Vec<i32>,
        user_id: i32,
    ) -> Result<usize, AppError> {
        let mut count = 0;
        for id in notification_ids {
            if let Err(e) = self.mark_as_read(id, user_id).await {
                tracing::warn!("标记通知 {} 已读失败: {}", id, e);
            } else {
                count += 1;
            }
        }
        Ok(count)
    }

    /// 标记所有通知为已读（使用批量更新优化）
    pub async fn mark_all_as_read(&self, user_id: i32) -> Result<usize, AppError> {
        let now = Utc::now();
        let result = notification::Entity::update_many()
            .filter(notification::Column::UserId.eq(user_id))
            .filter(notification::Column::Status.eq(NotificationStatus::Unread))
            .set(notification::ActiveModel {
                status: Set(NotificationStatus::Read),
                read_at: Set(Some(now)),
                updated_at: Set(now),
                ..Default::default()
            })
            .exec(self.db.as_ref())
            .await?;

        Ok(result.rows_affected as usize)
    }

    /// 删除通知（软删除）
    pub async fn delete_notification(
        &self,
        notification_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let notification = NotificationEntity::find_by_id(notification_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "通知 {} 不存在",
                notification_id
            )))?;

        if notification.user_id != user_id {
            return Err(AppError::PermissionDenied(
                "无权删除此通知".to_string(),
            ));
        }

        let mut active_model: notification::ActiveModel = notification.into();
        active_model.status = Set(NotificationStatus::Deleted);
        active_model.updated_at = Set(Utc::now());
        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 获取通知详情
    pub async fn get_notification(
        &self,
        notification_id: i32,
        user_id: i32,
    ) -> Result<notification::Model, AppError> {
        let notification = NotificationEntity::find_by_id(notification_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "通知 {} 不存在",
                notification_id
            )))?;

        if notification.user_id != user_id {
            return Err(AppError::PermissionDenied(
                "无权查看此通知".to_string(),
            ));
        }

        Ok(notification)
    }

    // ========== 通知设置相关方法 ==========

    /// 获取用户的通知设置
    pub async fn get_user_settings(
        &self,
        user_id: i32,
    ) -> Result<Vec<notification_setting::Model>, AppError> {
        let settings = NotificationSettingEntity::find()
            .filter(notification_setting::Column::UserId.eq(user_id))
            .all(&*self.db)
            .await?;

        Ok(settings)
    }

    /// 更新通知设置
    pub async fn update_setting(
        &self,
        user_id: i32,
        business_type: String,
        enable_internal: bool,
        enable_email: bool,
        enable_sms: bool,
    ) -> Result<notification_setting::Model, AppError> {
        let existing = NotificationSettingEntity::find()
            .filter(notification_setting::Column::UserId.eq(user_id))
            .filter(notification_setting::Column::BusinessType.eq(&business_type))
            .one(&*self.db)
            .await?;

        let setting = if let Some(existing) = existing {
            let mut active_model: notification_setting::ActiveModel = existing.into();
            active_model.enable_internal = Set(enable_internal);
            active_model.enable_email = Set(enable_email);
            active_model.enable_sms = Set(enable_sms);
            active_model.updated_at = Set(Utc::now());
            active_model.update(&*self.db).await?
        } else {
            let active_model = notification_setting::ActiveModel {
                id: Default::default(),
                user_id: Set(user_id),
                business_type: Set(business_type),
                enable_internal: Set(enable_internal),
                enable_email: Set(enable_email),
                enable_sms: Set(enable_sms),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            active_model.insert(&*self.db).await?
        };

        Ok(setting)
    }

    /// 获取数据库连接（用于关联查询）
    pub fn db(&self) -> &Arc<DatabaseConnection> {
        &self.db
    }

    /// 检查用户是否启用了某类通知
    pub async fn is_notification_enabled(
        &self,
        user_id: i32,
        business_type: &str,
        notification_type: &NotificationType,
    ) -> Result<bool, AppError> {
        let setting = NotificationSettingEntity::find()
            .filter(notification_setting::Column::UserId.eq(user_id))
            .filter(notification_setting::Column::BusinessType.eq(business_type))
            .one(&*self.db)
            .await?;

        let enabled = match setting {
            Some(s) => match notification_type {
                NotificationType::Internal | NotificationType::System => s.enable_internal,
                NotificationType::Email => s.enable_email,
                NotificationType::Sms => s.enable_sms,
            },
            None => true, // 默认启用
        };

        Ok(enabled)
    }
}
