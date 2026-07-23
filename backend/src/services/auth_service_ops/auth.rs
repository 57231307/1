//! 认证业务实现子模块（auth_service_ops/auth）
//!
//! 从原 `auth_service.rs` 迁移 `impl AuthService` 的业务方法：
//! - authenticate：用户登录认证（用户名+密码）
//! - generate_token / generate_refresh_token：JWT 令牌生成
//! - validate_token_static：JWT 令牌验证（密钥轮换场景）
//! - verify_password / verify_password_async：密码验证（Argon2id）
//! - hash_password / hash_password_async：密码哈希（Argon2id）

use crate::models::user;
use crate::services::auth_service::{AppClaims, AuthError, AuthService};
use crate::services::user_service::UserService;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, Header, Validation};

impl AuthService {
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

        // v14 P0-1 修复：使用 spawn_blocking 包装 Argon2id 哈希计算，避免阻塞 tokio worker
        let is_valid =
            Self::verify_password_async(password.to_string(), user.password_hash.clone()).await?;
        if !is_valid {
            return Err(AuthError::InvalidPassword("密码错误".to_string()));
        }

        if !user.is_active {
            return Err(AuthError::UserInactive);
        }

        let token = self
            .generate_token(user.id, &user.username, user.role_id)
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
    ///
    /// # 返回
    /// - `Ok(token)`: 生成的 JWT 令牌
    /// - `Err(AuthError::TokenGenerationError)`: 生成失败
    pub fn generate_token(
        &self,
        user_id: i32,
        username: &str,
        role_id: Option<i32>,
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
            exp,
            iat: now,
            refresh_exp,
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGenerationError(e.to_string()))
    }

    /// 生成 JWT 形式的刷新令牌（P1 7-1 修复）
    ///
    /// 修复背景：原 login 生成 `uuid::Uuid::new_v4().to_string()` 纯 UUID 作为 refresh_token，
    /// 但 refresh_token 接口用 `validate_token_static`（JWT 验证）校验，纯 UUID 必然验证失败，
    /// 导致 access_token 30 分钟过期后用户永远无法刷新。
    ///
    /// 修复方案：refresh_token 改用 JWT 形式，exp = refresh_exp = 7 天，
    /// session_id 与 access_token 共享，便于 refresh 时统一吊销旧会话。
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `username`: 用户名
    /// - `role_id`: 角色 ID（可选）
    /// - `session_id`: 会话 ID（与 access_token 共享，便于统一吊销）
    ///
    /// # 返回
    /// - `Ok(token)`: 生成的 JWT 刷新令牌（exp=7d, refresh_exp=7d）
    /// - `Err(AuthError::TokenGenerationError)`: 生成失败
    pub fn generate_refresh_token(
        &self,
        user_id: i32,
        username: &str,
        role_id: Option<i32>,
        session_id: &str,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        // refresh_token 的 exp = refresh_exp = 2 天后
        // P2 7-9 修复：原 7 天有效期缩短至 2 天，降低 refresh_token 被盗用后的有效窗口
        // 使 validate_token_static（验证 exp）能通过，同时 refresh_exp 检查也通过
        let refresh_exp = now + Duration::days(2);

        let claims = AppClaims {
            sub: user_id,
            username: username.to_string(),
            role_id,
            exp: refresh_exp,
            iat: now,
            refresh_exp,
            session_id: session_id.to_string(),
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
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
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        // P2 7-10 修复：leeway 从 60 秒降至 5 秒，避免 Token 过期后仍有 60 秒有效窗口
        validation.leeway = 5;

        let token_data = decode::<AppClaims>(token, &decoding_key, &validation)
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
        // 验证哈希长度，防止异常长的哈希导致性能问题或安全风险
        if hash.len() > 512 {
            return Err(AuthError::InvalidPassword("密码哈希长度异常".to_string()));
        }

        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| AuthError::HashingError(e.to_string()))?;

        let argon2 = Argon2::default();
        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(AuthError::HashingError(e.to_string())),
        }
    }

    /// 异步验证密码（v14 P0-1 修复：用 spawn_blocking 包装 Argon2id 哈希计算，避免阻塞 tokio worker）
    ///
    /// Argon2id（m=64MB, t=3, p=4）单次哈希耗时 50-100ms，同步调用会阻塞 async runtime。
    /// 生产 async 上下文必须使用此异步版本；测试夹具可继续使用同步版本。
    pub async fn verify_password_async(
        password: String,
        hash: String,
    ) -> Result<bool, AuthError> {
        tokio::task::spawn_blocking(move || Self::verify_password(&password, &hash))
            .await
            .map_err(|e| AuthError::HashingError(format!("spawn_blocking join 失败: {}", e)))?
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
    /// use bingxi_backend::AuthService;
    /// let hash = AuthService::hash_password("my_password").unwrap();
    /// ```
    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        // 使用更安全的Argon2参数配置: 64MB内存，3次迭代，4并发度
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(65536, 3, 4, None)
                .map_err(|e| AuthError::HashingError(e.to_string()))?,
        );

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AuthError::HashingError(e.to_string()))
    }

    /// 异步哈希密码（v14 P0-1 修复：用 spawn_blocking 包装 Argon2id 哈希计算，避免阻塞 tokio worker）
    ///
    /// Argon2id（m=64MB, t=3, p=4）单次哈希耗时 50-100ms，同步调用会阻塞 async runtime。
    /// 生产 async 上下文必须使用此异步版本；测试夹具可继续使用同步版本。
    pub async fn hash_password_async(password: String) -> Result<String, AuthError> {
        tokio::task::spawn_blocking(move || Self::hash_password(&password))
            .await
            .map_err(|e| AuthError::HashingError(format!("spawn_blocking join 失败: {}", e)))?
    }
}
