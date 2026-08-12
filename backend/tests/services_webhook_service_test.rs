    use bingxi_backend::services::webhook_service::*;
#[cfg(test)]
mod tests {

    /// M8 测试：MAX_RETRY_COUNT 常量值为 5
    #[test]
    fn test_max_retry_count_value() {
        assert_eq!(MAX_RETRY_COUNT, 5);
    }

    /// M8 测试：WebhookPayload 序列化/反序列化正确
    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            event: "order.created".to_string(),
            timestamp: "2026-07-11T08:00:00Z".to_string(),
            data: serde_json::json!({"order_id": 12345}),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: WebhookPayload = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.event, "order.created");
        assert_eq!(deserialized.timestamp, "2026-07-11T08:00:00Z");
        assert_eq!(deserialized.data["order_id"], 12345);
    }

    /// M8 测试：WebhookDeliveryResult 默认状态（失败时）
    #[test]
    fn test_webhook_delivery_result_error_case() {
        let result = WebhookDeliveryResult {
            success: false,
            status_code: None,
            response_body: None,
            error: Some("连接超时".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: WebhookDeliveryResult = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.success);
        assert!(deserialized.status_code.is_none());
        assert!(deserialized.error.is_some());
        assert_eq!(deserialized.error.unwrap(), "连接超时");
    }
}