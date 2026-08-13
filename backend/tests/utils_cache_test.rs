use bingxi_backend::utils::cache::*;
#[cfg(test)]
mod csrf_token_tests {

    /// 单元测试：set_csrf_token 写入 + consume_csrf_token 匹配 IP 成功
    #[test]
    fn test_set_csrf_token_then_consume_with_matching_ip() {
        let cache = AppCache::new();
        let token = "test-csrf-token-001".to_string();
        cache.set_csrf_token(
            token.clone(),
            "session-A".to_string(),
            "203.0.113.10".to_string(),
            42,
            None,
        );
        let result = cache.consume_csrf_token(&token, "203.0.113.10");
        assert_eq!(
            result,
            CsrfConsumeResult::Ok,
            "IP 匹配应返回 Ok，实际: {:?}",
            result
        );
    }

    /// 单元测试：consume_csrf_token IP 不匹配时返回 IpMismatch，且 token 仍保留
    #[test]
    fn test_consume_csrf_token_with_mismatched_ip_returns_ip_mismatch_and_keeps_token() {
        let cache = AppCache::new();
        let token = "test-csrf-token-002".to_string();
        cache.set_csrf_token(
            token.clone(),
            "session-B".to_string(),
            "203.0.113.20".to_string(),
            43,
            None,
        );

        // 第一次消费：IP 不匹配 → IpMismatch
        let r1 = cache.consume_csrf_token(&token, "198.51.100.99");
        assert_eq!(
            r1,
            CsrfConsumeResult::IpMismatch,
            "IP 不匹配应返回 IpMismatch，实际: {:?}",
            r1
        );

        // 第二次消费：使用正确 IP → 仍能消费成功（IP 不匹配不消费 token）
        let r2 = cache.consume_csrf_token(&token, "203.0.113.20");
        assert_eq!(
            r2,
            CsrfConsumeResult::Ok,
            "IP 不匹配不应消耗 token，原 IP 仍可消费，实际: {:?}",
            r2
        );
    }

    /// 单元测试：clear_old_csrf_token_for_user 清除用户旧 token
    #[test]
    fn test_clear_old_csrf_token_for_user_invalidates_old_token() {
        let cache = AppCache::new();
        let old_token = "old-csrf-token-003".to_string();
        cache.set_csrf_token(
            old_token.clone(),
            "session-C".to_string(),
            "203.0.113.30".to_string(),
            44,
            None,
        );

        // 强制轮换
        let cleared = cache.clear_old_csrf_token_for_user(44);
        assert!(cleared, "应返回 true（存在旧 token）");

        // 旧 token 已失效
        let r = cache.consume_csrf_token(&old_token, "203.0.113.30");
        assert_eq!(
            r,
            CsrfConsumeResult::NotFound,
            "清除后旧 token 应返回 NotFound，实际: {:?}",
            r
        );

        // 清除不存在的用户 → false
        let cleared_none = cache.clear_old_csrf_token_for_user(999);
        assert!(!cleared_none, "无活跃 token 的用户应返回 false");
    }

    /// 单元测试：IP 匹配消费后，反向索引同步清理（不再泄漏 user_id → token）
    #[test]
    fn test_consume_cleans_up_user_index() {
        let cache = AppCache::new();
        let token = "test-csrf-token-004".to_string();
        cache.set_csrf_token(
            token.clone(),
            "session-D".to_string(),
            "203.0.113.40".to_string(),
            45,
            None,
        );
        assert!(
            cache.csrf_user_index.contains_key(&45),
            "set 后反向索引应包含 user_id=45"
        );
        let _ = cache.consume_csrf_token(&token, "203.0.113.40");
        assert!(
            !cache.csrf_user_index.contains_key(&45),
            "consume 后反向索引应移除 user_id=45"
        );
    }
}
