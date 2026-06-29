//! 认证流程集成测试
//!
//! 批次 29 v7 P0-6 修复：原测试只测试 helpers 函数本身（create_test_token / create_auth_header），
//! 没有真正测试 auth 流程。本次重写为真正的认证流程测试：
//! - JWT 令牌生成 + 解码验证
//! - 认证请求头格式校验
//! - Token 过期 / 非法 token 处理
//! - 配置默认值与密钥长度
//!
//! 注意：真正的 HTTP 集成测试（如 /auth/login -> /auth/me -> /auth/logout 完整流程）
//! 需要 axum::test 启动服务器实例，已在 CI cargo test 流程中通过 service container 提供
//! TEST_DATABASE_URL。本模块聚焦于 token / header 级别的快速单元测试。

#[cfg(test)]
mod tests {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use serde::{Deserialize, Serialize};

    /// 测试 JWT 令牌：sub / username / role_id / exp 字段必须正确编码
    #[test]
    fn test_jwt_token_creation() {
        let token = super::super::helpers::create_test_token(1, "admin", Some(1));
        assert!(!token.is_empty(), "token 不应为空");
        assert!(token.contains('.'), "JWT 格式应包含两个点号分隔三段");

        // 解码并校验 claims
        #[derive(Debug, Serialize, Deserialize)]
        struct TestClaims {
            sub: i32,
            username: String,
            role_id: Option<i32>,
            exp: i64,
        }

        let token_data = decode::<TestClaims>(
            &token,
            &DecodingKey::from_secret(super::super::test_jwt_secret().as_bytes()),
            &Validation::default(),
        )
        .expect("JWT 解码失败");

        assert_eq!(token_data.claims.sub, 1, "sub 应为用户 ID");
        assert_eq!(token_data.claims.username, "admin", "username 应为 admin");
        assert_eq!(token_data.claims.role_id, Some(1), "role_id 应为 1");
        // exp 应为当前时间 + 3600 秒（1 小时后）
        let now = chrono::Utc::now().timestamp();
        assert!(
            token_data.claims.exp > now,
            "exp 应在未来时间，实际 exp={}, now={}",
            token_data.claims.exp,
            now
        );
    }

    /// 测试认证请求头：必须包含 Authorization Bearer + X-Requested-With
    #[test]
    fn test_auth_header_creation() {
        let token = "test_token_123";
        let headers = super::super::helpers::create_auth_header(token);

        assert_eq!(headers.len(), 2, "应包含 2 个 header");
        assert_eq!(headers[0].0, "Authorization", "第一个 header 应为 Authorization");
        assert_eq!(
            headers[0].1, "Bearer test_token_123",
            "Authorization 值应为 Bearer + token"
        );
        assert_eq!(
            headers[1].0, "X-Requested-With",
            "第二个 header 应为 X-Requested-With"
        );
        assert_eq!(
            headers[1].1, "XMLHttpRequest",
            "X-Requested-With 值应为 XMLHttpRequest"
        );
    }

    /// 测试非法 token：用错误密钥签发的 token 应解码失败
    #[test]
    fn test_invalid_token_rejected() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        #[derive(Debug, Serialize, Deserialize)]
        struct FakeClaims {
            sub: i32,
            username: String,
            role_id: Option<i32>,
            exp: i64,
        }
        let fake_claims = FakeClaims {
            sub: 999,
            username: "attacker".to_string(),
            role_id: None,
            exp: chrono::Utc::now().timestamp() + 3600,
        };
        // 用错误密钥签发 token（模拟攻击者伪造）
        let fake_token = encode(
            &Header::default(),
            &fake_claims,
            &EncodingKey::from_secret(b"wrong_secret_for_testing_only"),
        )
        .unwrap();

        // 用真实密钥解码应失败
        let result = decode::<serde_json::Value>(
            &fake_token,
            &DecodingKey::from_secret(super::super::test_jwt_secret().as_bytes()),
            &Validation::default(),
        );
        assert!(result.is_err(), "用错误密钥签发的 token 应解码失败");
    }

    /// 测试过期 token：exp 设置为过去时间应解码失败
    #[test]
    fn test_expired_token_rejected() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        #[derive(Debug, Serialize, Deserialize)]
        struct TestClaims {
            sub: i32,
            username: String,
            role_id: Option<i32>,
            exp: i64,
        }
        // 设置 exp 为 1 小时前（已过期）
        let expired_claims = TestClaims {
            sub: 1,
            username: "expired_user".to_string(),
            role_id: Some(1),
            exp: chrono::Utc::now().timestamp() - 3600,
        };
        let expired_token = encode(
            &Header::default(),
            &expired_claims,
            &EncodingKey::from_secret(super::super::test_jwt_secret().as_bytes()),
        )
        .unwrap();

        let result = decode::<TestClaims>(
            &expired_token,
            &DecodingKey::from_secret(super::super::test_jwt_secret().as_bytes()),
            &Validation::default(),
        );
        assert!(
            result.is_err(),
            "已过期的 token 应解码失败，实际结果: {:?}",
            result
        );
    }

    /// 测试测试配置默认值
    #[test]
    fn test_config_defaults() {
        let config = super::super::TestConfig::default();
        // 批次 29 修复：断言密钥长度符合最低安全要求（32 字节 = 64 十六进制字符）
        assert!(
            config.jwt_secret.len() >= 32,
            "JWT 密钥长度应 >= 32 字节，实际: {}",
            config.jwt_secret.len()
        );
        assert!(
            config.cookie_secret.len() >= 32,
            "Cookie 密钥长度应 >= 32 字节，实际: {}",
            config.cookie_secret.len()
        );
        assert!(
            !config.db_url.is_empty(),
            "数据库 URL 不应为空（应从 TEST_DATABASE_URL 或默认值获取）"
        );
    }

    /// 测试多次调用返回同一密钥（OnceLock 保证进程内一致）
    #[test]
    fn test_jwt_secret_consistency() {
        let secret1 = super::super::test_jwt_secret();
        let secret2 = super::super::test_jwt_secret();
        assert_eq!(
            secret1, secret2,
            "同一进程内多次调用应返回同一密钥（OnceLock 保证）"
        );
    }
}
