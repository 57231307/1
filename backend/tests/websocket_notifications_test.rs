    use bingxi_backend::websocket::notifications::*;
#[cfg(test)]
mod tests {

    /// 测试票据签发与消费（正常流程）
    #[test]
    fn test_ticket_issue_and_consume() {
        let manager = WsTicketManager::new();
        let ticket = manager.issue_ticket(42);
        // 票据长度 = UUID v4 simple(32) × 2 = 64 字符
        assert_eq!(ticket.len(), 64);

        // 首次消费应成功
        let user_id = manager.validate_and_consume(&ticket);
        assert_eq!(user_id, Some(42));
    }

    /// 测试票据一次性消费：第二次消费应失败
    #[test]
    fn test_ticket_one_time_use() {
        let manager = WsTicketManager::new();
        let ticket = manager.issue_ticket(99);

        // 首次消费成功
        assert_eq!(manager.validate_and_consume(&ticket), Some(99));
        // 第二次消费失败（已消费）
        assert_eq!(manager.validate_and_consume(&ticket), None);
    }

    /// 测试无效票据：空、过短、不存在
    #[test]
    fn test_ticket_invalid() {
        let manager = WsTicketManager::new();
        // 空票据
        assert_eq!(manager.validate_and_consume(""), None);
        // 过短票据
        assert_eq!(manager.validate_and_consume("short"), None);
        // 不存在的票据
        assert_eq!(manager.validate_and_consume(&"a".repeat(64)), None);
    }

    #[test]
    fn test_ws_message_serialize() {
        let msg = WsMessage::Ping {
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("ping"));
        assert!(json.contains("1234567890"));
    }

    #[test]
    fn test_notification_broadcaster() {
        let broadcaster = NotificationBroadcaster::new();
        let payload = NotificationPayload {
            id: 1,
            title: "测试".to_string(),
            content: "内容".to_string(),
            category: "system".to_string(),
            priority: 5,
            created_at: "2026-06-17T10:30:00Z".to_string(),
        };
        // 广播给无订阅者的用户应不报错
        broadcaster.broadcast_notification(100, &payload);
    }
}