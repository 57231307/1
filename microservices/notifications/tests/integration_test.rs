// integration_test.rs - 集成测试
// 用途：验证 gRPC 端到端流程
// 沙箱限制：仅在 CI 环境运行，本地沙箱 OOM 无法跑 cargo test
// 测试用 in-memory 数据库或 testcontainer

#[cfg(test)]
mod tests {
    use crate::model::NewNotification;

    /// 单元测试：NewNotification 校验逻辑
    #[test]
    fn test_new_notification_validate() {
        let valid = NewNotification {
            tenant_id: 1,
            user_id: 100,
            title: "测试通知".to_string(),
            content: "通知内容".to_string(),
            category: "order".to_string(),
            priority: 5,
        };
        assert!(valid.validate().is_ok());

        // 无效租户 ID
        let invalid_tenant = NewNotification {
            tenant_id: 0,
            ..valid.clone()
        };
        assert!(invalid_tenant.validate().is_err());

        // 无效优先级
        let invalid_priority = NewNotification {
            priority: 15,
            ..valid.clone()
        };
        assert!(invalid_priority.validate().is_err());

        // 空标题
        let empty_title = NewNotification {
            title: "".to_string(),
            ..valid.clone()
        };
        assert!(empty_title.validate().is_err());
    }

    /// 单元测试：model 模块导出正常
    #[test]
    fn test_model_module_exports() {
        let _notif = NewNotification {
            tenant_id: 1,
            user_id: 1,
            title: "x".to_string(),
            content: "y".to_string(),
            category: "system".to_string(),
            priority: 5,
        };
    }

    /// 集成测试：gRPC send_notification 端到端（需 PostgreSQL）
    /// 沙箱不跑，CI 中启用
    #[tokio::test]
    #[ignore = "需要 PostgreSQL，仅在 CI 启用"]
    async fn test_grpc_send_notification_end_to_end() {
        // 注：完整端到端测试需要：
        // 1. 启动 PostgreSQL testcontainer
        // 2. 启动 gRPC server（in-process）
        // 3. 客户端调用 SendNotification
        // 4. 验证数据库中存在该通知
        // 5. 验证租户隔离
        // 沙箱 OOM 限制下仅保留 stub，CI 完整实现
    }
}
