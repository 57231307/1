//! 集成测试模块
//!
//! 提供测试基础设施和辅助函数

/// 子测试模块 - 路由注册冒烟测试
pub mod api_routes;

use std::sync::Arc;

/// TS-S-3 安全加固（2026-06-26）：
/// 测试 JWT/Cookie 密钥改为运行时随机生成，避免硬编码密钥泄露后可伪造任意 JWT。
/// 使用 OnceLock 保证同一测试进程中所有测试共享同一随机密钥。
static TEST_JWT_SECRET_CELL: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static TEST_COOKIE_SECRET_CELL: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// 生成 64 字节随机十六进制字符串作为测试密钥
fn generate_random_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // 用系统时间 + 进程 ID + 线程 ID 作为熵源生成随机密钥
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let tid = format!("{:?}", std::thread::current().id());
    let seed = format!("{timestamp}{pid}{tid}");
    // 简单哈希生成 64 字节十六进制字符串
    let mut hash = [0u8; 32];
    let seed_bytes = seed.as_bytes();
    for (i, byte) in seed_bytes.iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(*byte).wrapping_mul(31);
    }
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

/// 获取测试 JWT 密钥（首次调用时随机生成，之后复用）
pub fn test_jwt_secret() -> &'static str {
    TEST_JWT_SECRET_CELL.get_or_init(generate_random_secret)
}

/// 获取测试 Cookie 密钥（首次调用时随机生成，之后复用）
pub fn test_cookie_secret() -> &'static str {
    TEST_COOKIE_SECRET_CELL.get_or_init(generate_random_secret)
}

/// 测试配置
pub struct TestConfig {
    pub db_url: String,
    pub jwt_secret: String,
    pub cookie_secret: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            db_url: std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/bingxi_test".to_string()
            }),
            jwt_secret: test_jwt_secret().to_string(),
            cookie_secret: test_cookie_secret().to_string(),
        }
    }
}

/// 测试辅助函数
pub mod helpers {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct TestClaims {
        sub: i32,
        username: String,
        role_id: Option<i32>,
        exp: i64,
    }

    /// 创建测试用的 JWT 令牌（使用运行时随机密钥）
    pub fn create_test_token(user_id: i32, username: &str, role_id: Option<i32>) -> String {
        let claims = TestClaims {
            sub: user_id,
            username: username.to_string(),
            role_id,
            exp: chrono::Utc::now().timestamp() + 3600,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(test_jwt_secret().as_bytes()),
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
