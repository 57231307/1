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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppClaims {
    pub sub: i32,
    pub username: String,
    pub role_id: Option<i32>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub iat: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AuthService {
    db: Arc<DatabaseConnection>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    pub fn new(db: Arc<DatabaseConnection>, secret: String) -> Self {
        Self {
            db,
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

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

        let token = self.generate_token(user.id, &user.username, user.role_id)
            .map_err(|e| AuthError::TokenGenerationError(e.to_string()))?;

        Ok((token, user))
    }

    pub fn generate_token(
        &self,
        user_id: i32,
        username: &str,
        role_id: Option<i32>,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        // Token expires in 24 hours
        let exp = now + Duration::hours(24);

        let claims = AppClaims {
            sub: user_id,
            username: username.to_string(),
            role_id,
            exp,
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGenerationError(e.to_string()))
    }

    /// 静态方法：验证JWT令牌（不依赖AuthService实例）
    pub fn validate_token_static(token: &str, secret: &str) -> Result<AppClaims, AuthError> {
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.leeway = 60;

        let token_data = decode::<AppClaims>(token, &decoding_key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }

    pub fn validate_token(&self, token: &str) -> Result<AppClaims, AuthError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.leeway = 60;

        let token_data = decode::<AppClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| AuthError::HashingError(e.to_string()))?;
        
        let argon2 = Argon2::default();
        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(AuthError::HashingError(e.to_string())),
        }
    }

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

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("用户名或密码错误")]
    InvalidCredentials,
    #[error("用户未激活")]
    UserInactive,
    #[error("数据库错误：{0}")]
    DatabaseError(#[from] sea_orm::DbErr),
    #[error("JWT 错误：{0}")]
    JwtError(String),
    #[error("密码哈希错误: {0}")]
    HashingError(String),
    #[error("用户不存在")]
    UserNotFound,
    #[error("无效的密码")]
    InvalidPassword,
    #[error("Token 生成失败: {0}")]
    TokenGenerationError(String),
    #[error("无效的 Token: {0}")]
    InvalidToken(String),
    #[error("Token 已被撤销")]
    TokenRevoked,
}
