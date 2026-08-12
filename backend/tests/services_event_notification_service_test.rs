#[cfg(test)]
mod tests {
use bingxi_backend::models::notification::*;
    use bingxi_backend::services::event_notification_service::*;


    // ========== build_inventory_alert_notification 字段完整性测试 ==========

    /// 验证库存预警通知请求体的全部字段被正确构造（场景一：常规参数）
    #[test]
    fn test_build_inventory_alert_notification_zdwzx_cgcs() {
        let req = EventNotificationService::build_inventory_alert_notification(
            1001,
            "纯棉面料 T001",
            5001,
            "120.5",
            "200",
        );

        // 用户 ID 应正确设置
        assert_eq!(req.user_id, 1001);
        // 通知类型应为站内信
        assert_eq!(req.notification_type, NotificationType::Internal);
        // 标题应为库存预警
        assert_eq!(req.title, "库存预警");
        // 内容应包含产品名、当前库存、阈值
        assert!(req.content.contains("纯棉面料 T001"));
        assert!(req.content.contains("120.5"));
        assert!(req.content.contains("200"));
        // 优先级应为紧急
        assert_eq!(req.priority, NotificationPriority::Urgent);
        // 业务类型应为库存
        assert_eq!(req.business_type.as_deref(), Some("INVENTORY"));
        // 业务 ID 应为产品 ID
        assert_eq!(req.business_id, Some(5001));
        // 跳转 URL 应包含产品 ID
        assert_eq!(req.action_url.as_deref(), Some("/inventory/stock/5001"));
        // 发送者 ID 应为 None（系统发送）
        assert_eq!(req.sender_id, None);
        // 发送者名称应为系统
        assert_eq!(req.sender_name.as_deref(), Some("系统"));
    }

    /// 验证库存预警通知请求体的全部字段被正确构造（场景二：含中文特殊字符的产品名）
    #[test]
    fn test_build_inventory_alert_notification_zdwzx_zwtszf() {
        let req = EventNotificationService::build_inventory_alert_notification(
            2002,
            "涤纶【防泼水】面料（蓝色）",
            8008,
            "0",
            "50.5",
        );

        assert_eq!(req.user_id, 2002);
        assert_eq!(req.notification_type, NotificationType::Internal);
        assert_eq!(req.title, "库存预警");
        // 内容应完整保留包含特殊字符的产品名
        assert!(req.content.contains("涤纶【防泼水】面料（蓝色）"));
        assert!(req.content.contains("0"));
        assert!(req.content.contains("50.5"));
        assert_eq!(req.priority, NotificationPriority::Urgent);
        assert_eq!(req.business_type.as_deref(), Some("INVENTORY"));
        assert_eq!(req.business_id, Some(8008));
        assert_eq!(req.action_url.as_deref(), Some("/inventory/stock/8008"));
        assert_eq!(req.sender_id, None);
        assert_eq!(req.sender_name.as_deref(), Some("系统"));
    }

    /// 验证库存预警通知请求体的全部字段被正确构造（场景三：库存为零且阈值极低）
    #[test]
    fn test_build_inventory_alert_notification_zdwzx_lkccj() {
        let req = EventNotificationService::build_inventory_alert_notification(
            3003,
            "羊毛混纺面料",
            10000,
            "0",
            "10",
        );

        assert_eq!(req.user_id, 3003);
        assert_eq!(req.notification_type, NotificationType::Internal);
        assert_eq!(req.title, "库存预警");
        assert!(req.content.contains("羊毛混纺面料"));
        assert!(req.content.contains("0"));
        assert!(req.content.contains("10"));
        assert_eq!(req.priority, NotificationPriority::Urgent);
        assert_eq!(req.business_type.as_deref(), Some("INVENTORY"));
        assert_eq!(req.business_id, Some(10000));
        // 验证 action_url 中 product_id 正确格式化（无前导零）
        assert_eq!(req.action_url.as_deref(), Some("/inventory/stock/10000"));
        assert_eq!(req.sender_id, None);
        assert_eq!(req.sender_name.as_deref(), Some("系统"));
    }

    /// 验证不同参数组合下 action_url 与 business_id 的一致性
    #[test]
    fn test_build_inventory_alert_notification_action_url_y_business_id_yz() {
        // 测试多组 product_id，确认 action_url 始终包含相同 product_id
        let product_ids = vec![1, 99, 12345, 999999];
        for pid in product_ids {
            let req = EventNotificationService::build_inventory_alert_notification(
                1,
                "测试面料",
                pid,
                "10",
                "20",
            );
            // business_id 与传入 product_id 一致
            assert_eq!(req.business_id, Some(pid));
            // action_url 中的 ID 字符串与 product_id 一致
            let expected_url = format!("/inventory/stock/{}", pid);
            assert_eq!(req.action_url.as_deref(), Some(expected_url.as_str()));
        }
    }

    /// 验证内容模板格式与原始内联构造逻辑一致
    #[test]
    fn test_build_inventory_alert_notification_nrmbgs() {
        let req = EventNotificationService::build_inventory_alert_notification(
            1,
            "亚麻面料",
            1,
            "88.8",
            "150.0",
        );
        // 内容应严格匹配模板：产品 '{}' 当前库存 {}，已低于预警阈值 {}
        assert_eq!(
            req.content,
            "产品 '亚麻面料' 当前库存 88.8，已低于预警阈值 150.0"
        );
    }
}