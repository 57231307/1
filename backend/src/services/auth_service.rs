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


use crate::models::user;
use crate::services::user_service::UserService;
use crate::utils::error::AppError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::{OnceCell, RwLock};

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
            return Err(AuthError::InvalidPassword("密码错误".to_string()));
        }

        if !user.is_active {
            return Err(AuthError::UserInactive);
        }

        let token = self
            .generate_token(user.id, &user.username, user.role_id, None)
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
    #[allow(dead_code)] // TODO(tech-debt): 业务接入后移除
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
    #[error("无效的密码: {0}")]
    InvalidPassword(String),
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

impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InvalidCredentials => AppError::unauthorized("用户名或密码错误"),
            AuthError::UserInactive => AppError::unauthorized("用户未激活"),
            AuthError::DatabaseError(e) => AppError::database(e.to_string()),
            AuthError::JwtError(e) => AppError::internal(format!("JWT 错误: {}", e)),
            AuthError::HashingError(e) => AppError::internal(format!("密码哈希错误: {}", e)),
            AuthError::UserNotFound => AppError::not_found("用户不存在"),
            AuthError::InvalidPassword(msg) => {
                AppError::unauthorized(format!("无效的密码: {}", msg))
            }
            AuthError::TokenGenerationError(e) => {
                AppError::internal(format!("Token 生成失败: {}", e))
            }
            AuthError::InvalidToken(e) => AppError::unauthorized(format!("无效的 Token: {}", e)),
            AuthError::TokenRevoked => AppError::unauthorized("Token 已被撤销"),
        }
    }
}

impl From<AppError> for AuthError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::DatabaseError(e) => AuthError::DatabaseError(sea_orm::DbErr::Custom(e)),
            AppError::NotFound(_) => AuthError::UserNotFound,
            _ => AuthError::DatabaseError(sea_orm::DbErr::Custom(err.to_string())),
        }
    }
}

// =====================================================================
// JTI 黑名单（已吊销的 JWT ID）
// =====================================================================
//
// 用于实现 Refresh Token 轮换场景下的旧 Token 立即失效：
// - 登出时调用 `revoke_jti` 吊销当前 Token 的 JTI（session_id）
// - Refresh Token 旋转时调用 `revoke_jti` 吊销旧 Token 的 JTI
// - 每次受保护请求在 middleware 中调用 `is_jti_revoked` 检查
//
// 低危 #1 修复：JTI 黑名单从进程内 HashMap 迁移到 Redis（SETEX + TTL）。
// 进程内存储在多实例部署时不共享，撤销后的旧 JWT 在其他实例最多可继续使用
// 2 小时（JWT 过期时间）。Redis 后端保证所有实例共享同一黑名单视图。
// Redis 不可用时自动回退到内存（graceful degradation）。

/// JTI 黑名单 Redis key 前缀
const JTI_KEY_PREFIX: &str = "jwt:jti:revoked:";

/// JWT JTI 黑名单（进程内降级回退表：jti -> 过期时间戳）
///
/// 仅在 Redis 不可用时使用，避免阻塞业务。生产环境应配置 Redis。
static JTI_BLACKLIST: LazyLock<RwLock<HashMap<String, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 分布式 JTI 黑名单 Redis 客户端（懒初始化）
///
/// 通过环境变量 `JTI_REDIS_URL` 或回退 `REDIS_URL` 启用。
static REDIS_JTI_BLACKLIST: OnceCell<Option<Arc<tokio::sync::Mutex<ConnectionManager>>>> =
    OnceCell::const_new();

/// 初始化 Redis JTI 黑名单客户端
async fn init_redis_jti_blacklist() -> Option<Arc<tokio::sync::Mutex<ConnectionManager>>> {
    let url = std::env::var("JTI_REDIS_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .ok()
        .filter(|s| !s.is_empty());

    let url = match url {
        Some(u) => u,
        None => {
            tracing::debug!(
                "JTI_REDIS_URL/REDIS_URL 未配置，JTI 黑名单使用进程内存储（多实例部署不安全）"
            );
            return None;
        }
    };

    match redis::Client::open(url.as_str()) {
        Ok(client) => match ConnectionManager::new(client).await {
            Ok(conn) => {
                tracing::info!("JTI 黑名单已启用 Redis 分布式后端 (URL 已配置)");
                Some(Arc::new(tokio::sync::Mutex::new(conn)))
            }
            Err(e) => {
                tracing::warn!(
                    "JTI 黑名单 Redis 连接失败 ({:?})，回退到进程内存储",
                    e
                );
                None
            }
        },
        Err(e) => {
            tracing::warn!("JTI 黑名单 Redis URL 解析失败 ({:?})，回退到进程内存储", e);
            None
        }
    }
}

/// 获取或初始化 Redis JTI 黑名单客户端
async fn get_redis_jti_blacklist() -> Option<Arc<tokio::sync::Mutex<ConnectionManager>>> {
    REDIS_JTI_BLACKLIST
        .get_or_init(init_redis_jti_blacklist)
        .await
        .clone()
}

/// 吊销指定 JTI
///
/// 将给定 JTI 加入黑名单（优先写 Redis；Redis 不可用时回退到进程内 HashMap）。
/// 后续请求将拒绝持有该 JTI 的 Token。
///
/// # 参数
/// - `jti`: 待吊销的 Token 唯一标识（当前实现取自 `AppClaims::session_id`）
/// - `expires_at`: Token 的过期时间戳（Unix 秒）
pub async fn revoke_jti(jti: &str, expires_at: i64) {
    // 主路径：写入 Redis（SETEX 自动设置 TTL，过期自动清理，零维护成本）
    if let Some(conn_arc) = get_redis_jti_blacklist().await {
        let now = chrono::Utc::now().timestamp();
        let ttl_secs = (expires_at - now).max(1) as u64;
        let key = format!("{}{}", JTI_KEY_PREFIX, jti);

        let write_result: Result<(), redis::RedisError> = async {
            let mut conn = conn_arc.lock().await;
            let _: () = conn.set_ex(&key, expires_at.to_string(), ttl_secs).await?;
            Ok(())
        }
        .await;

        if let Err(e) = write_result {
            tracing::warn!(
                "JTI 写入 Redis 失败 ({:?})，回退到进程内存储；jti={}",
                e,
                jti
            );
            // 降级：写入内存
            let mut blacklist = JTI_BLACKLIST.write().await;
            blacklist.insert(jti.to_string(), expires_at);
        } else {
            tracing::info!("JTI 已吊销（Redis）：{}，TTL {} 秒", jti, ttl_secs);
            return;
        }
    } else {
        // 未配置 Redis：直接写内存
        let mut blacklist = JTI_BLACKLIST.write().await;
        blacklist.insert(jti.to_string(), expires_at);
        tracing::info!("JTI 已吊销（内存）：{}，过期时间：{}", jti, expires_at);
    }
}

/// 检查 JTI 是否在黑名单
///
/// # 参数
/// - `jti`: 待检查的 Token 唯一标识
///
/// # 返回
/// - `true`: 该 JTI 已被吊销
/// - `false`: 该 JTI 仍然有效
pub async fn is_jti_revoked(jti: &str) -> bool {
    // 主路径：查 Redis
    if let Some(conn_arc) = get_redis_jti_blacklist().await {
        let key = format!("{}{}", JTI_KEY_PREFIX, jti);
        let check_result: Result<bool, redis::RedisError> = async {
            let mut conn = conn_arc.lock().await;
            let exists: bool = conn.exists(&key).await?;
            Ok(exists)
        }
        .await;

        match check_result {
            Ok(exists) => return exists,
            Err(e) => {
                tracing::warn!(
                    "JTI 查 Redis 失败 ({:?})，回退到进程内检查；jti={}",
                    e,
                    jti
                );
                // 降级：查内存
            }
        }
    }

    // 降级：查内存
    let blacklist = JTI_BLACKLIST.read().await;
    blacklist.contains_key(jti)
}

/// 清理过期 JTI（建议定期调用，如每小时）
///
/// 当使用 Redis 后端时，TTL 自动清理过期条目，此函数为 noop。
/// 当回退到进程内存储时，主动清理已超过过期时间的记录，避免内存泄漏。
///
/// # 参数
/// - `_max_age_secs`: 允许的最大存活时间（秒），当前实现忽略该参数
pub async fn cleanup_expired_jti(_max_age_secs: i64) {
    // Redis 后端下，SETEX TTL 自动清理，无需手动操作
    if get_redis_jti_blacklist().await.is_some() {
        tracing::debug!("JTI 黑名单使用 Redis 后端，过期条目由 TTL 自动清理");
        return;
    }

    // 进程内存储降级路径：手动清理过期记录
    let mut blacklist = JTI_BLACKLIST.write().await;
    let now = chrono::Utc::now().timestamp();
    let before = blacklist.len();
    blacklist.retain(|_, expires_at| *expires_at > now);
    let removed = before - blacklist.len();
    tracing::info!(
        "清理 JTI 黑名单（内存）：移除 {} 条过期记录，剩余 {} 条",
        removed,
        blacklist.len()
    );
}

// =====================================================================
// 用户级 Token 吊销表（修复安全漏洞 #9：删除/封禁用户后即时撤销其所有活跃 JWT）
// =====================================================================
//
// 设计动机：现有 JTI 黑名单按 session_id（UUID）维度存储，
// 但应用层在删除/封禁用户时无法枚举该用户历史上颁发的全部 session_id。
// 为此新增 user_id -> revoked_at 的全局表，middleware 在校验完 Claims 后
// 再检查 `claims.iat < user_revoke_ts` 以决定是否放行。
//
// 语义：
// - `revoke_user_jtis(user_id, reason)`：将 user_id 标记为已吊销，记录当前时间戳。
//   后续所有 iat < 该时间戳的 Token 一律拒绝；iat >= 该时间戳的 Token 仍然有效。
// - `is_user_token_revoked(user_id, token_iat)`：供 middleware 调用的快速判定。
// - 该表为进程内内存表，进程重启后失效。生产环境如需持久化，
//   应迁移到 Redis/DB（按 user_id 维度持久化 revoked_at），此实现仅做 MVP。

/// 用户级 Token 吊销表（user_id -> 吊销时间戳，Unix 秒）
static REVOKED_USERS: LazyLock<RwLock<HashMap<i32, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 吊销指定用户的所有活跃 JWT
///
/// 将 user_id 加入内存吊销表，记录当前时间戳为吊销点。
/// 后续 middleware 收到该用户 Token 时，若 `iat < revoked_at` 则拒绝。
///
/// # 参数
/// - `user_id`: 被吊销用户的 ID
/// - `reason`: 吊销原因（如 `"USER_DELETED"`、`"USER_DEACTIVATED"`），仅用于日志
///
/// # 返回
/// - `Ok(())`: 成功加入吊销表
/// - `Err(AuthError::InternalError)`: 当前实现下不会失败，保留 Result 供后续扩展
pub async fn revoke_user_jtis(
    user_id: i32,
    reason: &str,
) -> Result<(), crate::utils::error::AppError> {
    let now = chrono::Utc::now().timestamp();
    let mut table = REVOKED_USERS.write().await;
    table.insert(user_id, now);
    tracing::warn!(
        target: "security_audit",
        event = "USER_TOKENS_REVOKED",
        user_id = user_id,
        reason = reason,
        revoked_at = now,
        "[SECURITY] 用户级 Token 吊销：user_id={} reason={} revoked_at={}",
        user_id,
        reason,
        now
    );
    Ok(())
}

/// 检查某用户 Token 是否已被吊销
///
/// 判定规则：
/// - 若 user_id 不在吊销表中，返回 `false`（未吊销）
/// - 若 token_iat >= revoked_at，返回 `false`（Token 在吊销后签发，仍有效）
/// - 若 token_iat < revoked_at，返回 `true`（Token 在吊销前签发，必须拒绝）
///
/// # 参数
/// - `user_id`: Token 所属用户 ID
/// - `token_iat`: Token 签发时间戳（Unix 秒）
pub async fn is_user_token_revoked(user_id: i32, token_iat: i64) -> bool {
    let table = REVOKED_USERS.read().await;
    if let Some(&revoked_at) = table.get(&user_id) {
        token_iat < revoked_at
    } else {
        false
    }
}

/// 清理过期的用户吊销记录（建议定期调用）
///
/// 当前实现为占位：因 revoked_at 永不过期（仅当用户重新激活时调用方应主动删除），
/// 此函数保留接口以备后续策略调整（例如引入"吊销 TTL"）。
#[allow(dead_code)] // TODO(tech-debt): 业务接入后移除
pub async fn cleanup_revoked_users() {
    // 当前策略：吊销记录永久保留，直至进程重启或显式 unregister。
    // 保留此函数以备后续引入"自动解除封禁"等业务策略。
    let table = REVOKED_USERS.read().await;
    tracing::info!("当前用户吊销表条目数：{}", table.len());
}

/// 显式注销用户吊销标记（用于用户重新激活场景）
///
/// # 参数
/// - `user_id`: 需注销的用户 ID
#[allow(dead_code)] // TODO(tech-debt): 业务接入后移除
pub async fn unrevoke_user(user_id: i32) {
    let mut table = REVOKED_USERS.write().await;
    if table.remove(&user_id).is_some() {
        tracing::info!("用户吊销标记已清除：user_id={}", user_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // P9-1: 测试夹具 helper，封装 AuthService 的常见操作
    fn hash_pwd(p: &str) -> String {
        AuthService::hash_password(p).expect("P9-1: 测试夹具 密码哈希失败")
    }

    fn verify_pwd_ok(plain: &str, hash: &str) -> bool {
        AuthService::verify_password(plain, hash).expect("P9-1: 测试夹具 密码验证失败")
    }

    // P9-1: 集中 encode 调用，避免 4 处重复 .expect
    fn encode_test_token(claims: &AppClaims, secret: &str) -> String {
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("P9-1: 测试夹具 令牌编码失败")
    }

    fn validate_test_token(token: &str, secret: &str) -> AppClaims {
        AuthService::validate_token_static(token, secret).expect("P9-1: 令牌验证失败")
    }

    /// 统一测试 JWT 密钥（与 tests/integration/mod.rs::TEST_JWT_SECRET 保持一致）
    const TEST_JWT_SECRET: &str = "test-jwt-secret-key-for-integration-tests-only-32bytes";

    /// 测试密码哈希和验证
    #[test]
    fn test_password_hash_and_verify() {
        let password = "TestPassword123!";
        let hash = hash_pwd(password);

        // 验证正确密码
        assert!(verify_pwd_ok(password, &hash));

        // 验证错误密码
        assert!(!verify_pwd_ok("WrongPassword", &hash));
    }

    /// 测试密码哈希唯一性（相同密码应产生不同哈希）
    #[test]
    fn test_password_hash_uniqueness() {
        let password = "TestPassword123!";
        let hash1 = hash_pwd(password);
        let hash2 = hash_pwd(password);

        // 两次哈希结果应不同（因为使用了随机盐）
        assert_ne!(hash1, hash2);

        // 但都能验证通过
        assert!(verify_pwd_ok(password, &hash1));
        assert!(verify_pwd_ok(password, &hash2));
    }

    /// 测试 JWT 令牌生成和验证（使用集中测试密钥常量）
    #[test]
    fn test_token_generation_and_validation() {
        let secret = TEST_JWT_SECRET;

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

        let token = encode_test_token(&claims, secret);

        // 验证令牌
        let decoded = validate_test_token(&token, secret);
        assert_eq!(decoded.sub, 1);
        assert_eq!(decoded.username, "testuser");
        assert_eq!(decoded.role_id, Some(1));
        assert_eq!(decoded.tenant_id, None);
    }

    /// 测试无效令牌验证
    #[test]
    fn test_invalid_token_validation() {
        let secret = TEST_JWT_SECRET;
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

        let token = encode_test_token(&claims, secret);

        // 使用错误的密钥验证应失败
        let result = AuthService::validate_token_static(&token, wrong_secret);
        assert!(result.is_err());
    }

    /// 测试过期令牌验证
    #[test]
    fn test_expired_token_validation() {
        let secret = TEST_JWT_SECRET;

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

        let token = encode_test_token(&claims, secret);

        // 过期令牌验证应失败
        let result = AuthService::validate_token_static(&token, secret);
        assert!(result.is_err());
    }

    /// 测试令牌声明字段完整性
    #[test]
    fn test_token_claims_fields() {
        let secret = TEST_JWT_SECRET;

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

        let token = encode_test_token(&claims, secret);

        let decoded = validate_test_token(&token, secret);

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

    // =================================================================
    // 安全漏洞 #9 修复：用户级 Token 吊销函数单元测试
    // =================================================================

    /// 测试 `revoke_user_jtis` 后旧 iat Token 被判定为已吊销
    #[tokio::test]
    async fn test_revoke_user_jtis_blocks_old_iat_token() {
        // 选一个不与其他测试冲突的高位 user_id
        let test_user_id: i32 = 9_999_001;

        // 清理：确保测试前该用户未被吊销
        unrevoke_user(test_user_id).await;

        // 模拟"删除前签发"的 Token：iat 在 revoke 之前 1 小时
        let old_iat = chrono::Utc::now().timestamp() - 3600;
        assert!(
            !is_user_token_revoked(test_user_id, old_iat).await,
            "未吊销时旧 Token 应判定为有效"
        );

        // 标记用户吊销
        revoke_user_jtis(test_user_id, "USER_DELETED")
            .await
            .expect("revoke_user_jtis 不应失败");

        // 删除前的 Token (iat < revoked_at) 应被拒绝
        assert!(
            is_user_token_revoked(test_user_id, old_iat).await,
            "revoke 之前的 Token 必须被判定为已吊销"
        );

        // 清理
        unrevoke_user(test_user_id).await;
    }

    /// 测试吊销后新签发 Token 不受影响（iat >= revoked_at）
    #[tokio::test]
    async fn test_revoke_user_jtis_does_not_block_new_iat_token() {
        let test_user_id: i32 = 9_999_002;
        unrevoke_user(test_user_id).await;

        // 标记吊销
        revoke_user_jtis(test_user_id, "USER_DEACTIVATED")
            .await
            .expect("revoke_user_jtis 不应失败");

        // 等 10ms 模拟"新 Token 在吊销后签发"
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let new_iat = chrono::Utc::now().timestamp();

        assert!(
            !is_user_token_revoked(test_user_id, new_iat).await,
            "吊销后签发的新 Token 不应被误判为已吊销"
        );

        unrevoke_user(test_user_id).await;
    }

    /// 测试 `unrevoke_user` 解除吊销标记
    #[tokio::test]
    async fn test_unrevoke_user_clears_revocation() {
        let test_user_id: i32 = 9_999_003;
        unrevoke_user(test_user_id).await;

        revoke_user_jtis(test_user_id, "USER_DELETED")
            .await
            .expect("revoke_user_jtis 不应失败");

        let old_iat = chrono::Utc::now().timestamp() - 60;
        assert!(is_user_token_revoked(test_user_id, old_iat).await);

        unrevoke_user(test_user_id).await;
        assert!(
            !is_user_token_revoked(test_user_id, old_iat).await,
            "unrevoke 后旧 Token 应判定为有效"
        );
    }

    // =================================================================
    // 安全漏洞 #1 修复：JTI 黑名单→Redis 单元测试
    // =================================================================
    //
    // 注：测试运行环境未配置 JTI_REDIS_URL/REDIS_URL，自动回退到进程内 HashMap。
    // 此处覆盖回退路径的核心行为：revoke → is_revoked 双向一致性。

    /// 测试 JTI revoke 后立即被 is_jti_revoked 判定为已吊销
    #[tokio::test]
    async fn test_revoke_jti_marks_as_revoked() {
        let test_jti = format!("test-jti-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let expires_at = chrono::Utc::now().timestamp() + 3600;

        // revoke 前应未吊销
        assert!(!is_jti_revoked(&test_jti).await, "新 JTI 默认应为有效");

        // revoke 后应判定为已吊销
        revoke_jti(&test_jti, expires_at).await;
        assert!(is_jti_revoked(&test_jti).await, "revoke 后应被判定为已吊销");
    }

    /// 测试不同 JTI 之间互不干扰
    #[tokio::test]
    async fn test_revoke_jti_isolation() {
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let jti_a = format!("test-jti-a-{}", ts);
        let jti_b = format!("test-jti-b-{}", ts);
        let expires_at = chrono::Utc::now().timestamp() + 3600;

        revoke_jti(&jti_a, expires_at).await;

        assert!(is_jti_revoked(&jti_a).await, "jti_a 应被吊销");
        assert!(!is_jti_revoked(&jti_b).await, "jti_b 不应被吊销（互不干扰）");
    }

    /// 测试 cleanup_expired_jti 在内存模式下移除过期项
    #[tokio::test]
    async fn test_cleanup_expired_jti_removes_expired_entries() {
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let expired_jti = format!("test-jti-expired-{}", ts);
        let fresh_jti = format!("test-jti-fresh-{}", ts);
        let now = chrono::Utc::now().timestamp();

        // 插入一条已过期（1 小时前到期）和一条未过期（1 小时后到期）
        revoke_jti(&expired_jti, now - 3600).await;
        revoke_jti(&fresh_jti, now + 3600).await;

        assert!(is_jti_revoked(&expired_jti).await);
        assert!(is_jti_revoked(&fresh_jti).await);

        // 触发清理
        cleanup_expired_jti(0).await;

        // 过期项应被清除，未过期项保留
        assert!(
            !is_jti_revoked(&expired_jti).await,
            "过期 JTI 应被清理"
        );
        assert!(
            is_jti_revoked(&fresh_jti).await,
            "未过期 JTI 应保留"
        );
    }
}
