use crate::models::user;
use crate::services::user_service::UserService;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

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

#[derive(Debug, Clone)]
pub struct AuthService {
    db: Arc<DatabaseConnection>,
    secret: Vec<u8>,
}

impl AuthService {
    pub fn new(db: Arc<DatabaseConnection>, secret: String) -> Self {
        Self {
            db,
            secret: secret.into_bytes(),
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

        let json = serde_json::to_string(&claims)
            .map_err(|e| AuthError::JwtError(e.to_string()))?;

        let encoded = BASE64.encode(json.as_bytes());
        let signature = self.sign(&encoded);
        let token = format!("{}.{}", encoded, signature);

        Ok(token)
    }

    pub fn validate_token(&self, token: &str) -> Result<AppClaims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 2 {
            return Err(AuthError::JwtError("Invalid token format".to_string()));
        }

        let (encoded, signature) = (parts[0], parts[1]);
        if !self.verify_signature(encoded, signature) {
            return Err(AuthError::JwtError("Invalid signature".to_string()));
        }

        let json = BASE64.decode(encoded)
            .map_err(|e| AuthError::JwtError(e.to_string()))?;

        let claims: AppClaims = serde_json::from_slice(&json)
            .map_err(|e| AuthError::JwtError(e.to_string()))?;

        if claims.exp < Utc::now() {
            return Err(AuthError::JwtError("Token expired".to_string()));
        }

        Ok(claims)
    }

    fn sign(&self, data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        self.secret.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn verify_signature(&self, data: &str, signature: &str) -> bool {
        self.sign(data) == signature
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

    #[allow(dead_code)]
    pub fn get_secret(&self) -> &str {
        std::str::from_utf8(&self.secret).unwrap_or("")
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
