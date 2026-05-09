//! 认证服务模块
//!
//! 提供用户认证、JWT令牌管理和密码安全处理功能。
//!
//! # 主要功能
//! - 用户登录认证（用户名+密码）
//! - JWT令牌生成与验证
//! - 密码哈希（Argon2id）
//! - 双因素认证（TOTP）支持
//!
//! # 安全特性
//! - 使用 Argon2id 进行密码哈希（64MB内存，3次迭代，4并发度）
//! - JWT 令牌有效期2小时，刷新令牌7天
//! - 支持令牌黑名单机制
//! - 支持密钥轮换（平滑过渡）

#![allow(dead_code)]

use crate::models::user;
use crate::services::user_service::UserService;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT 令牌声明
///
/// 包含用户身份信息和令牌元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppClaims {
    /// 用户 ID（Subject）
    pub sub: i32,
    /// 用户名
    pub username: String,
    /// 角色 ID
    pub role_id: Option<i32>,
    /// 租户 ID（多租户支持）
    pub tenant_id: Option<i32>,
    /// 令牌过期时间
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
    /// 令牌签发时间
    #[serde(with = "chrono::serde::ts_seconds")]
    pub iat: DateTime<Utc>,
    /// 刷新令牌过期时间（7天）
    #[serde(with = "chrono::serde::ts_seconds")]
    pub refresh_exp: DateTime<Utc>,
    /// 会话唯一标识
    pub session_id: String,
}

/// 认证服务
///
/// 处理用户认证、令牌生成和验证
#[derive(Clone)]
pub struct AuthService {
    db: Arc<DatabaseConnection>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    /// 创建新的认证服务实例
    ///
    /// # 参数
    /// - `db`: 数据库连接
    /// - `secret`: JWT 密钥
    pub fn new(db: Arc<DatabaseConnection>, secret: String) -> Self {
        Self {
            db,
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    /// 用户登录认证
    ///
    /// 验证用户名和密码，成功后返回 JWT 令牌和用户信息
    ///
    /// # 参数
    /// - `username`: 用户名
    /// - `password`: 明文密码
    ///
    /// # 返回
    /// - `Ok((token, user))`: 认证成功，返回令牌和用户信息
    /// - `Err(AuthError)`: 认证失败
    ///
    /// # 错误
    /// - `InvalidPassword`: 密码错误
    /// - `UserInactive`: 用户未激活
    /// - `UserNotFound`: 用户不存在
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(String, user::Model), AuthError> {
        let user = UserService::new(self.db.clone())
            .find_by_username(username)
            .await?;

        let is_valid = Self::verify_password(password, &user.password_hash)?;
        if !is_valid {
            return Err(AuthError::InvalidPassword);
        }

        if !user.is_active {
            return Err(AuthError::UserInactive);
        }

        let token = self.generate_token(user.id, &user.username, user.role_id, None)
            .map_err(|e| AuthError::TokenGenerationError(e.to_string()))?;

        Ok((token, user))
    }

    /// 生成 JWT 访问令牌
    ///
    /// 创建包含用户信息的 JWT 令牌，有效期2小时
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `username`: 用户名
    /// - `role_id`: 角色 ID（可选）
    /// - `tenant_id`: 租户 ID（可选）
    ///
    /// # 返回
    /// - `Ok(token)`: 生成的 JWT 令牌
    /// - `Err(AuthError::TokenGenerationError)`: 生成失败
    pub fn generate_token(
        &self,
        user_id: i32,
        username: &str,
        role_id: Option<i32>,
        tenant_id: Option<i32>,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        // Token expires in 2 hours (reduced from 24 hours for security)
        let exp = now + Duration::hours(2);
        // Refresh token expires in 7 days
        let refresh_exp = now + Duration::days(7);

        let claims = AppClaims {
            sub: user_id,
            username: username.to_string(),
            role_id,
            tenant_id,
            exp,
            iat: now,
            refresh_exp,
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGenerationError(e.to_string()))
    }

    /// 静态方法：验证 JWT 令牌
    ///
    /// 不依赖 AuthService 实例，使用提供的密钥验证令牌
    /// 用于密钥轮换场景（先尝试当前密钥，失败后再尝试旧密钥）
    ///
    /// # 参数
    /// - `token`: JWT 令牌字符串
    /// - `secret`: JWT 密钥
    ///
    /// # 返回
    /// - `Ok(claims)`: 验证成功，返回令牌声明
    /// - `Err(AuthError::InvalidToken)`: 令牌无效或已过期
    pub fn validate_token_static(token: &str, secret: &str) -> Result<AppClaims, AuthError> {
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.leeway = 60;

        let token_data = decode::<AppClaims>(token, &decoding_key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }

    /// 验证 JWT 令牌
    ///
    /// 使用 AuthService 实例的解码密钥验证令牌
    ///
    /// # 参数
    /// - `token`: JWT 令牌字符串
    ///
    /// # 返回
    /// - `Ok(claims)`: 验证成功，返回令牌声明
    /// - `Err(AuthError::InvalidToken)`: 令牌无效或已过期
    pub fn validate_token(&self, token: &str) -> Result<AppClaims, AuthError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.leeway = 60;

        let token_data = decode::<AppClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }

    /// 验证密码
    ///
    /// 使用 Argon2id 验证明文密码与哈希值是否匹配
    ///
    /// # 参数
    /// - `password`: 明文密码
    /// - `hash`: 密码哈希值
    ///
    /// # 返回
    /// - `Ok(true)`: 密码正确
    /// - `Ok(false)`: 密码错误
    /// - `Err(AuthError::HashingError)`: 哈希解析失败
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| AuthError::HashingError(e.to_string()))?;
        
        let argon2 = Argon2::default();
        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(AuthError::HashingError(e.to_string())),
        }
    }

    /// 哈希密码
    ///
    /// 使用 Argon2id 算法对明文密码进行哈希处理
    /// 配置参数：64MB内存，3次迭代，4并发度
    ///
    /// # 参数
    /// - `password`: 明文密码
    ///
    /// # 返回
    /// - `Ok(hash)`: 密码哈希值
    /// - `Err(AuthError::HashingError)`: 哈希失败
    ///
    /// # 示例
    /// ```
    /// let hash = AuthService::hash_password("my_password")?;
    /// ```
    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        // 使用更安全的Argon2参数配置: 64MB内存，3次迭代，4并发度
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(65536, 3, 4, None).map_err(|e| AuthError::HashingError(e.to_string()))?,
        );

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AuthError::HashingError(e.to_string()))
    }
}

/// 认证错误类型
///
/// 定义认证过程中可能发生的所有错误
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// 用户名或密码错误
    #[error("用户名或密码错误")]
    InvalidCredentials,
    /// 用户未激活
    #[error("用户未激活")]
    UserInactive,
    /// 数据库错误
    #[error("数据库错误：{0}")]
    DatabaseError(#[from] sea_orm::DbErr),
    /// JWT 相关错误
    #[error("JWT 错误：{0}")]
    JwtError(String),
    /// 密码哈希错误
    #[error("密码哈希错误: {0}")]
    HashingError(String),
    /// 用户不存在
    #[error("用户不存在")]
    UserNotFound,
    /// 无效的密码
    #[error("无效的密码")]
    InvalidPassword,
    /// 令牌生成失败
    #[error("Token 生成失败: {0}")]
    TokenGenerationError(String),
    /// 无效的令牌
    #[error("无效的 Token: {0}")]
    InvalidToken(String),
    /// 令牌已被撤销
    #[error("Token 已被撤销")]
    TokenRevoked,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试密码哈希和验证
    #[test]
    fn test_password_hash_and_verify() {
        let password = "TestPassword123!";
        let hash = AuthService::hash_password(password).expect("密码哈希失败");

        // 验证正确密码
        assert!(AuthService::verify_password(password, &hash).expect("验证失败"));

        // 验证错误密码
        assert!(!AuthService::verify_password("WrongPassword", &hash).expect("验证失败"));
    }

    /// 测试密码哈希唯一性（相同密码应产生不同哈希）
    #[test]
    fn test_password_hash_uniqueness() {
        let password = "TestPassword123!";
        let hash1 = AuthService::hash_password(password).expect("密码哈希失败");
        let hash2 = AuthService::hash_password(password).expect("密码哈希失败");

        // 两次哈希结果应不同（因为使用了随机盐）
        assert_ne!(hash1, hash2);

        // 但都能验证通过
        assert!(AuthService::verify_password(password, &hash1).expect("验证失败"));
        assert!(AuthService::verify_password(password, &hash2).expect("验证失败"));
    }

    /// 测试 JWT 令牌生成和验证（使用静态方法）
    #[test]
    fn test_token_generation_and_validation() {
        let secret = "test-secret-key-for-jwt-tokens-32-bytes";

        // 使用静态方法直接测试令牌生成和验证
        // 先生成一个令牌（通过编码）
        let now = Utc::now();
        let claims = AppClaims {
            sub: 1,
            username: "testuser".to_string(),
            role_id: Some(1),
            tenant_id: None,
            exp: now + Duration::hours(2),
            iat: now,
            refresh_exp: now + Duration::days(7),
            session_id: "test-session-123".to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("令牌编码失败");

        // 验证令牌
        let decoded = AuthService::validate_token_static(&token, secret).expect("令牌验证失败");
        assert_eq!(decoded.sub, 1);
        assert_eq!(decoded.username, "testuser");
        assert_eq!(decoded.role_id, Some(1));
        assert_eq!(decoded.tenant_id, None);
    }

    /// 测试无效令牌验证
    #[test]
    fn test_invalid_token_validation() {
        let secret = "test-secret-key-for-jwt-tokens-32-bytes";
        let wrong_secret = "wrong-secret-key-for-jwt-tokens-32-byte";

        let now = Utc::now();
        let claims = AppClaims {
            sub: 1,
            username: "testuser".to_string(),
            role_id: Some(1),
            tenant_id: None,
            exp: now + Duration::hours(2),
            iat: now,
            refresh_exp: now + Duration::days(7),
            session_id: "test-session-456".to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("令牌编码失败");

        // 使用错误的密钥验证应失败
        let result = AuthService::validate_token_static(&token, wrong_secret);
        assert!(result.is_err());
    }

    /// 测试过期令牌验证
    #[test]
    fn test_expired_token_validation() {
        let secret = "test-secret-key-for-jwt-tokens-32-bytes";

        let now = Utc::now();
        let claims = AppClaims {
            sub: 1,
            username: "testuser".to_string(),
            role_id: Some(1),
            tenant_id: None,
            exp: now - Duration::hours(1), // 已过期
            iat: now - Duration::hours(2),
            refresh_exp: now + Duration::days(7),
            session_id: "test-session-789".to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("令牌编码失败");

        // 过期令牌验证应失败
        let result = AuthService::validate_token_static(&token, secret);
        assert!(result.is_err());
    }

    /// 测试令牌声明字段完整性
    #[test]
    fn test_token_claims_fields() {
        let secret = "test-secret-key-for-jwt-tokens-32-bytes";

        let now = Utc::now();
        let claims = AppClaims {
            sub: 42,
            username: "admin".to_string(),
            role_id: Some(1),
            tenant_id: Some(100),
            exp: now + Duration::hours(2),
            iat: now,
            refresh_exp: now + Duration::days(7),
            session_id: "test-session-abc".to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("令牌编码失败");

        let decoded = AuthService::validate_token_static(&token, secret).expect("令牌验证失败");

        assert_eq!(decoded.sub, 42);
        assert_eq!(decoded.username, "admin");
        assert_eq!(decoded.role_id, Some(1));
        assert_eq!(decoded.tenant_id, Some(100));

        // 验证时间字段
        assert!(decoded.iat.timestamp() >= now.timestamp() - 1);
        assert!(decoded.exp > decoded.iat);
        assert!(decoded.refresh_exp > decoded.exp);

        // 验证会话ID不为空
        assert!(!decoded.session_id.is_empty());
    }
}
