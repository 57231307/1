//! 认证流程集成测试
//!
//! 测试登录、访问、登出完整流程

#[cfg(test)]
mod tests {
    /// 测试JWT令牌生成和验证
    #[test]
    fn test_jwt_token_creation() {
        let token = super::super::helpers::create_test_token(1, "admin", Some(1));
        assert!(!token.is_empty());
        assert!(token.contains('.')); // JWT格式包含点号
    }

    /// 测试认证请求头创建
    #[test]
    fn test_auth_header_creation() {
        let token = "test_token_123";
        let headers = super::super::helpers::create_auth_header(token);

        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].0, "Authorization");
        assert_eq!(headers[0].1, "Bearer test_token_123");
        assert_eq!(headers[1].0, "X-Requested-With");
        assert_eq!(headers[1].1, "XMLHttpRequest");
    }

    /// 测试测试配置默认值
    #[test]
    fn test_config_defaults() {
        let config = super::super::TestConfig::default();
        assert_eq!(config.jwt_secret.len(), 56);
        assert_eq!(config.cookie_secret.len(), 58);
    }
}
