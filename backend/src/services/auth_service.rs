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

        if !self.verify_password(password, &user.password_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        if !user.is_active {
            return Err(AuthError::UserInactive);
        }

        let token = self.generate_token(user.id, &user.username, user.role_id)?;

        Ok((token, user))
    }

    pub fn generate_token(
        &self,
        user_id: i32,
        username: &str,
        role_id: Option<i32>,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let exp = now + Duration::hours(24);

        let claims = AppClaims {
            sub: user_id,
            username: username.to_string(),
            role_id,
            exp,
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::JwtError(e.to_string()))
    }

    /// 静态方法：验证JWT令牌（不依赖AuthService实例）
    pub fn validate_token_static(token: &str, secret: &str) -> Result<AppClaims, AuthError> {
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.leeway = 60;

        let token_data = decode::<AppClaims>(token, &decoding_key, &validation)
            .map_err(|e| AuthError::JwtError(e.to_string()))?;

        Ok(token_data.claims)
    }

    pub fn validate_token(&self, token: &str) -> Result<AppClaims, AuthError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.leeway = 60;

        let token_data = decode::<AppClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| AuthError::JwtError(e.to_string()))?;

        Ok(token_data.claims)
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }

    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| AuthError::HashError)?;

        Ok(hash.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AuthError {
    #[error("用户名或密码错误")]
    InvalidCredentials,
    #[error("用户未激活")]
    UserInactive,
    #[error("数据库错误：{0}")]
    DatabaseError(#[from] sea_orm::DbErr),
    #[error("JWT 错误：{0}")]
    JwtError(String),
    #[error("密码哈希错误")]
    HashError,
    #[error("用户不存在")]
    UserNotFound,
}
