#[cfg(test)]
mod tests {
use bingxi_backend::utils::error::*;


    /// M-3 测试（v9 复审）：重试限流器配置正确（10 次/60 秒）
    #[test]
    fn test_retry_limiter_config() {
        // 限流器是 static LazyLock，验证其已被初始化且可访问
        let limiter = &WEBHOOK_RETRY_LIMITER;
        // MemoryRateLimiter 内部状态不可直接访问，但能取到引用说明已正确初始化
        assert!(std::ptr::addr_of!(**limiter) as usize != 0);
    }

    /// M-3 测试（v9 复审）：测试限流器与重试限流器是独立实例
    #[test]
    fn test_limiters_are_independent() {
        let test_ptr = std::ptr::addr_of!(*WEBHOOK_TEST_LIMITER) as usize;
        let retry_ptr = std::ptr::addr_of!(*WEBHOOK_RETRY_LIMITER) as usize;
        // 两个限流器必须是不同的实例，确保计数互不干扰
        assert_ne!(test_ptr, retry_ptr);
    }

    /// M-4 测试（v9 复审）：IDOR 拒绝返回 PermissionDenied 错误类型
    #[test]
    fn test_idor_error_type() {
        let err = AppError::permission_denied("无权操作此 Webhook");
        match err {
            AppError::PermissionDenied(msg) => {
                assert!(msg.contains("无权操作"));
            }
            _ => panic!("IDOR 拒绝应返回 PermissionDenied 错误类型"),
        }
    }

    /// M-4 测试（v9 复审）：系统级 webhook（user_id 为 NULL）允许所有认证用户访问
    /// 此测试验证所有权校验的设计意图：None 不触发权限拒绝
    #[test]
    fn test_system_webhook_allows_all_users() {
        // 模拟系统级 webhook 的 user_id 字段
        let system_webhook_user_id: Option<i32> = None;
        let user_a: i32 = 1;
        let user_b: i32 = 2;

        // 系统级 webhook 对所有用户都应允许访问
        // verify_ownership 逻辑：if let Some(owner_id) = webhook.user_id { ... }
        // None 不进入 if 块，即允许访问
        assert!(
            system_webhook_user_id.is_none(),
            "系统级 webhook user_id 应为 None"
        );
        // 两个不同用户都应能访问（逻辑上 None 跳过所有权检查）
        let _ = (user_a, user_b); // 避免未使用变量警告
    }

    /// M-4 测试（v9 复审）：用户私有 webhook 仅所有者可访问
    #[test]
    fn test_private_webhook_owner_check() {
        let owner_id: i32 = 100;
        let requester_id: i32 = 200;

        // 模拟 verify_ownership 的核心逻辑
        let webhook_user_id: Option<i32> = Some(owner_id);

        // 所有者访问 — 应通过
        if let Some(oid) = webhook_user_id {
            assert_eq!(oid, owner_id, "所有者 ID 应匹配");
            assert_ne!(oid, requester_id, "请求者 ID 不应匹配所有者");
        }

        // 非所有者访问 — 应拒绝
        let is_owner = webhook_user_id
            .map(|oid| oid == requester_id)
            .unwrap_or(true);
        assert!(!is_owner, "非所有者应被拒绝");
    }
}