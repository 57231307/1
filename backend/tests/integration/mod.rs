//! 集成测试模块
//!
//! 提供测试基础设施和辅助函数

/// 子测试模块 - 路由注册冒烟测试
pub mod api_routes;

use std::sync::Arc;

/// 测试配置
pub struct TestConfig {
    pub db_url: String,
    pub jwt_secret: String,
    pub cookie_secret: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            db_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/bingxi_test".to_string()),
            jwt_secret: "test-jwt-secret-key-for-integration-tests-only-32bytes".to_string(),
            cookie_secret: "test-cookie-secret-key-for-integration-tests-only-32bytes".to_string(),
        }
    }
}

/// 测试辅助函数
pub mod helpers {
    use super::*;

    /// 创建测试用的JWT令牌
    pub fn create_test_token(user_id: i32, username: &str, role_id: Option<i32>) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct TestClaims {
            sub: i32,
            username: String,
            role_id: Option<i32>,
            exp: i64,
        }

        let claims = TestClaims {
            sub: user_id,
            username: username.to_string(),
            role_id,
            exp: chrono::Utc::now().timestamp() + 3600,
        };

        let secret = "test-jwt-secret-key-for-integration-tests-only-32bytes";
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    /// 创建测试请求头
    pub fn create_auth_header(token: &str) -> Vec<(&'static str, String)> {
        vec![
            ("Authorization", format!("Bearer {}", token)),
            ("X-Requested-With", "XMLHttpRequest".to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TestConfig::default();
        assert!(!config.jwt_secret.is_empty());
        assert!(!config.cookie_secret.is_empty());
    }

    #[test]
    fn test_create_test_token() {
        let token = helpers::create_test_token(1, "test_user", Some(1));
        assert!(!token.is_empty());
    }
}
