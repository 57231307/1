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