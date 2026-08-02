//! 通知服务
//!
//! 提供通知消息的创建、查询、更新、删除等功能
//! 支持站内信、邮件、短信等多种通知渠道

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;

use crate::models::notification::{
    self, Entity as NotificationEntity, NotificationPriority, NotificationStatus, NotificationType,
};
use crate::models::notification_setting::{self, Entity as NotificationSettingEntity};
use crate::utils::error::AppError;
use crate::websocket::notifications::{get_notification_broadcaster, NotificationPayload};

/// 将数据库 notification::Model 转为 WebSocket 推送载荷（批次 24 v6 P0-2 修复：通知创建后实时推送至在线 ws 客户端）
fn build_payload_from_notification(n: &notification::Model) -> NotificationPayload {
    let priority_value = match n.priority {
        NotificationPriority::Low => 1,
        NotificationPriority::Normal => 5,
        NotificationPriority::High => 8,
        NotificationPriority::Urgent => 10,
    };
    NotificationPayload {
        id: n.id as i64,
        title: n.title.clone(),
        content: n.content.clone(),
        category: format!("{:?}", n.notification_type).to_lowercase(),
        priority: priority_value,
        created_at: n.created_at.to_rfc3339(),
    }
}

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
    /// 缺陷 5.2 修复：去重键，Some 时 5 分钟窗口内相同 key 跳过创建
    pub dedup_key: Option<String>,
}

/// 缺陷 5.2 修复：去重窗口（5 分钟）
const DEDUP_WINDOW_SECS: i64 = 300;

/// 通知服务
pub struct NotificationService {
    db: Arc<DatabaseConnection>,
}

impl NotificationService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 缺陷 5.2 修复：检查 5 分钟窗口内是否已存在相同 dedup_key 的通知
    async fn check_dedup(&self, user_id: i32, dedup_key: &str) -> Result<bool, AppError> {
        let threshold = Utc::now() - chrono::Duration::seconds(DEDUP_WINDOW_SECS);
        let count = NotificationEntity::find()
            .filter(notification::Column::UserId.eq(user_id))
            .filter(notification::Column::DedupKey.eq(dedup_key))
            .filter(notification::Column::CreatedAt.gt(threshold))
            .count(&*self.db)
            .await?;
        Ok(count > 0)
    }

    /// 创建通知消息
    pub async fn create_notification(
        &self,
        req: CreateNotificationRequest,
    ) -> Result<notification::Model, AppError> {
        // 缺陷 5.2 修复：dedup_key 存在时先查 5 分钟窗口，命中则跳过创建
        if let Some(key) = req.dedup_key.as_deref() {
            if self.check_dedup(req.user_id, key).await? {
                return Err(AppError::validation(
                    "通知去重：5 分钟窗口内已存在相同 dedup_key",
                ));
            }
        }

        let active_model = notification::ActiveModel {
            id: Default::default(),
            user_id: Set(req.user_id),
            notification_type: Set(req.notification_type.clone()),
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
            dedup_key: Set(req.dedup_key.clone()),
        };

        let notification = active_model.insert(&*self.db).await?;

        // 批次 24 v6 P0-2 修复：通知创建后实时推送至在线 ws 客户端
        // 广播失败不影响通知创建本身（如所有订阅者已掉线，broadcast 返回 SendError，忽略即可）
        let payload = build_payload_from_notification(&notification);
        get_notification_broadcaster()
            .broadcast_notification(notification.user_id as i64, &payload);

        // 缺陷 5.1 修复：Webhook 类型通知触发外部系统推送
        if matches!(req.notification_type, NotificationType::Webhook) {
            self.dispatch_webhook_notification(&notification).await;
        }

        Ok(notification)
    }

    /// 缺陷 5.1 修复：分发 Webhook 通知到外部系统（企业微信/钉钉/Slack）
    /// 查询用户启用的 active webhook（含系统级 user_id IS NULL），逐个触发推送。
    /// best-effort 语义：单个 webhook 失败仅记录 warn 日志，不影响通知创建主流程。
    async fn dispatch_webhook_notification(&self, notification: &notification::Model) {
        use crate::models::webhook::{self, Entity as WebhookEntity};
        use crate::services::webhook_service::WebhookService;
        use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter};

        // 查询当前用户可用的 active webhook：系统级（user_id IS NULL）+ 用户私有（user_id = notification.user_id）
        let ownership_condition = Condition::any()
            .add(webhook::Column::UserId.is_null())
            .add(webhook::Column::UserId.eq(notification.user_id));

        let webhooks = match WebhookEntity::find()
            .filter(webhook::Column::IsActive.eq(true))
            .filter(ownership_condition)
            .all(&*self.db)
            .await
        {
            Ok(ws) => ws,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    notification_id = notification.id,
                    "Webhook 通知分发：查询用户 webhook 列表失败，跳过分发"
                );
                return;
            }
        };

        if webhooks.is_empty() {
            tracing::debug!(
                notification_id = notification.id,
                user_id = notification.user_id,
                "Webhook 通知分发：用户未配置 active webhook，跳过分发"
            );
            return;
        }

        // 构造通知载荷 JSON（推送给外部系统的数据结构）
        let payload = serde_json::json!({
            "notification_id": notification.id,
            "user_id": notification.user_id,
            "title": notification.title,
            "content": notification.content,
            "business_type": notification.business_type,
            "business_id": notification.business_id,
            "action_url": notification.action_url,
            "priority": format!("{:?}", notification.priority),
            "created_at": notification.created_at.to_rfc3339(),
        });
        let payload_str = payload.to_string();

        let webhook_service = WebhookService::new(self.db.clone());
        let event_name = notification
            .business_type
            .as_deref()
            .unwrap_or("notification")
            .to_lowercase();

        let mut success_count = 0u32;
        let mut fail_count = 0u32;

        for wh in &webhooks {
            // 仅触发订阅了 "*" 或当前事件类型的 webhook，避免向无关 webhook 推送
            let subscribed_events: Vec<&str> = wh.events.split(',').map(|s| s.trim()).collect();
            if !subscribed_events.contains(&"*")
                && !subscribed_events.contains(&event_name.as_str())
            {
                continue;
            }

            match webhook_service
                .trigger_webhook(notification.user_id, wh.id, &event_name, &payload_str)
                .await
            {
                Ok(result) => {
                    if result.success {
                        success_count += 1;
                        tracing::info!(
                            notification_id = notification.id,
                            webhook_id = wh.id,
                            webhook_name = %wh.name,
                            "Webhook 通知分发成功"
                        );
                    } else {
                        fail_count += 1;
                        tracing::warn!(
                            notification_id = notification.id,
                            webhook_id = wh.id,
                            webhook_name = %wh.name,
                            error = ?result.error,
                            status_code = ?result.status_code,
                            "Webhook 通知分发失败（best-effort，不影响主流程）"
                        );
                    }
                }
                Err(e) => {
                    fail_count += 1;
                    tracing::warn!(
                        error = %e,
                        notification_id = notification.id,
                        webhook_id = wh.id,
                        webhook_name = %wh.name,
                        "Webhook 通知分发异常（best-effort，不影响主流程）"
                    );
                }
            }
        }

        tracing::info!(
            notification_id = notification.id,
            user_id = notification.user_id,
            total_webhooks = webhooks.len(),
            success_count,
            fail_count,
            "Webhook 通知分发完成"
        );
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
            // 缺陷 5.2 修复：dedup_key 存在时先查 5 分钟窗口，命中则跳过
            if let Some(key) = req.dedup_key.as_deref() {
                if self.check_dedup(req.user_id, key).await? {
                    continue;
                }
            }

            let active_model = notification::ActiveModel {
                id: Default::default(),
                user_id: Set(req.user_id),
                notification_type: Set(req.notification_type.clone()),
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
                dedup_key: Set(req.dedup_key.clone()),
            };
            let notification = active_model.insert(self.db.as_ref()).await?;

            // 批次 24 v6 P0-2 修复：每条通知创建后实时推送至在线 ws 客户端
            let payload = build_payload_from_notification(&notification);
            get_notification_broadcaster()
                .broadcast_notification(notification.user_id as i64, &payload);

            // 缺陷 5.1 修复：Webhook 类型通知触发外部系统推送（与单条创建逻辑一致）
            if matches!(notification.notification_type, NotificationType::Webhook) {
                self.dispatch_webhook_notification(&notification).await;
            }

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
            .offset(page.saturating_sub(1) * page_size)
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
            .ok_or_else(|| AppError::not_found(format!("通知 {} 不存在", notification_id)))?;

        if notification.user_id != user_id {
            return Err(AppError::permission_denied("无权操作此通知"));
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
        if notification_ids.is_empty() {
            return Ok(0);
        }
        // v11 批次 37 修复：用 update_many 批量更新，避免循环内逐个 find_by_id + update（N+1）
        // 参考 mark_all_as_read 的批量模式，增加 Id.is_in 过滤指定通知
        let now = Utc::now();
        let result = notification::Entity::update_many()
            .filter(notification::Column::Id.is_in(notification_ids))
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
            .ok_or_else(|| AppError::not_found(format!("通知 {} 不存在", notification_id)))?;

        if notification.user_id != user_id {
            return Err(AppError::permission_denied("无权删除此通知"));
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
            .ok_or_else(|| AppError::not_found(format!("通知 {} 不存在", notification_id)))?;

        if notification.user_id != user_id {
            return Err(AppError::permission_denied("无权查看此通知"));
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
                enable_webhook: Set(false),
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
                NotificationType::Webhook => s.enable_webhook,
            },
            None => true, // 默认启用
        };

        Ok(enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::notification::{
        Model as NotificationModel, NotificationPriority, NotificationStatus, NotificationType,
    };
    use chrono::Utc;

    /// 构造测试用 notification::Model（避免每个测试重复字段）
    fn make_test_notification(
        ntype: NotificationType,
        priority: NotificationPriority,
        business_type: Option<&str>,
    ) -> NotificationModel {
        NotificationModel {
            id: 1001,
            user_id: 5001,
            notification_type: ntype,
            title: "测试通知标题".to_string(),
            content: "测试通知内容".to_string(),
            priority,
            status: NotificationStatus::Unread,
            business_type: business_type.map(|s| s.to_string()),
            business_id: Some(2002),
            action_url: Some("/test/path".to_string()),
            sender_id: Some(3003),
            sender_name: Some("测试发送者".to_string()),
            read_at: None,
            processed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            dedup_key: Some("test_dedup_key".to_string()),
        }
    }

    // ========== 缺陷 5.1 补充：Webhook 通知类型与载荷测试 ==========

    /// Webhook 类型通知的 build_payload_from_notification 应生成正确载荷
    #[test]
    fn test_build_payload_webhook_leha_zh() {
        let n = make_test_notification(
            NotificationType::Webhook,
            NotificationPriority::Urgent,
            Some("INVENTORY"),
        );
        let payload = build_payload_from_notification(&n);

        assert_eq!(payload.id, 1001_i64);
        assert_eq!(payload.title, "测试通知标题");
        assert_eq!(payload.content, "测试通知内容");
        assert_eq!(payload.category, "webhook");
        assert_eq!(payload.priority, 10); // Urgent → 10
    }

    /// 不同优先级映射到正确的数值
    #[test]
    fn test_build_payload_priority_ys() {
        let cases = vec![
            (NotificationPriority::Low, 1),
            (NotificationPriority::Normal, 5),
            (NotificationPriority::High, 8),
            (NotificationPriority::Urgent, 10),
        ];
        for (prio, expected) in cases {
            let n = make_test_notification(NotificationType::Webhook, prio.clone(), None);
            let payload = build_payload_from_notification(&n);
            assert_eq!(
                payload.priority, expected,
                "优先级 {:?} 应映射为 {}",
                prio, expected
            );
        }
    }

    /// NotificationType::Webhook 变体应正确匹配
    #[test]
    fn test_notification_type_webhook_match() {
        let n = make_test_notification(
            NotificationType::Webhook,
            NotificationPriority::Normal,
            None,
        );
        assert!(matches!(n.notification_type, NotificationType::Webhook));
    }

    /// 非 Webhook 类型不应匹配 Webhook 分支
    #[test]
    fn test_notification_type_non_webhook_no_match() {
        let n = make_test_notification(
            NotificationType::Internal,
            NotificationPriority::Normal,
            None,
        );
        assert!(!matches!(n.notification_type, NotificationType::Webhook));

        let n2 =
            make_test_notification(NotificationType::Email, NotificationPriority::Normal, None);
        assert!(!matches!(n2.notification_type, NotificationType::Webhook));
    }

    /// CreateNotificationRequest 应支持 Webhook 类型和 dedup_key
    #[test]
    fn test_create_request_webhook_with_dedup() {
        let req = CreateNotificationRequest {
            user_id: 5001,
            notification_type: NotificationType::Webhook,
            title: "库存预警".to_string(),
            content: "产品 A 库存不足".to_string(),
            priority: NotificationPriority::Urgent,
            business_type: Some("INVENTORY".to_string()),
            business_id: Some(100),
            action_url: Some("/inventory/stock/100".to_string()),
            sender_id: None,
            sender_name: Some("系统".to_string()),
            dedup_key: Some("inventory_alert:100".to_string()),
        };

        assert!(matches!(req.notification_type, NotificationType::Webhook));
        assert_eq!(req.dedup_key.as_deref(), Some("inventory_alert:100"));
        assert_eq!(req.business_type.as_deref(), Some("INVENTORY"));
    }

    // ========== 缺陷 5.2 补充：去重窗口常量测试 ==========

    /// DEDUP_WINDOW_SECS 应为 300（5 分钟）
    #[test]
    fn test_dedup_window_secs_val() {
        assert_eq!(DEDUP_WINDOW_SECS, 300);
    }

    /// 去重窗口为 5 分钟 = 300 秒
    #[test]
    fn test_dedup_window_is_5min() {
        let five_mins = 5 * 60;
        assert_eq!(DEDUP_WINDOW_SECS, five_mins);
    }
}
