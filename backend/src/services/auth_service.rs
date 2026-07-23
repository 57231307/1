//! 认证服务模块（facade）
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
//!
//! # 模块拆分说明
//! 本文件为 facade，仅保留：
//! - `AppClaims` / `AuthService` struct 定义与 `new` 构造函数
//! - `AuthError` enum 及其与 `AppError` 的 `From` 实现
//! - 测试模块
//! 业务方法（`impl AuthService` 的登录/验证/哈希等）迁移至 `auth_service_ops::auth`，
//! JTI 黑名单与用户级 Token 吊销的 free functions 迁移至 `auth_service_ops::jti`，
//! 下方通过 `pub use` 重新导出，保持外部调用路径不变。

use crate::utils::error::AppError;
use chrono::{DateTime, Utc};
use jsonwebtoken::EncodingKey;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// JTI 黑名单与用户级 Token 吊销的 free functions 在 auth_service_ops::jti 中实现，
// 此处重新导出以保持外部调用路径（如 crate::services::auth_service::revoke_jti）不变。
pub use crate::services::auth_service_ops::jti::{
    cleanup_expired_jti, cleanup_revoked_users, is_jti_revoked, is_user_token_revoked,
    revoke_jti, revoke_user_jtis, start_revoked_user_cleanup_task, unrevoke_user,
};

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
///
/// 字段声明为 `pub(crate)` 以便 `auth_service_ops::auth` 子模块的 `impl AuthService`
/// 块直接访问（业务方法已迁移至该子模块）。
#[derive(Clone)]
pub struct AuthService {
    pub(crate) db: Arc<DatabaseConnection>,
    pub(crate) encoding_key: EncodingKey,
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
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::auth_service_ops::jti::{REVOKED_USER_TTL_SECS, REVOKED_USERS};
    use chrono::Duration;
    use jsonwebtoken::{encode, Header};

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

    /// TS-S-3 安全加固（2026-06-26）：
    /// 测试 JWT 密钥改为运行时随机生成，避免硬编码密钥泄露后可伪造任意 JWT。
    /// 使用 OnceLock 保证同一测试进程中所有测试共享同一随机密钥。
    static TEST_JWT_SECRET_CELL: std::sync::OnceLock<String> = std::sync::OnceLock::new();

    /// 生成随机测试密钥
    fn generate_test_secret() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let seed = format!("{timestamp}{pid}");
        let mut hash = [0u8; 32];
        for (i, byte) in seed.as_bytes().iter().enumerate() {
            hash[i % 32] = hash[i % 32].wrapping_add(*byte).wrapping_mul(31);
        }
        hash.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// 获取测试 JWT 密钥
    fn test_jwt_secret() -> &'static str {
        TEST_JWT_SECRET_CELL.get_or_init(generate_test_secret)
    }

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

    /// 测试 JWT 令牌生成和验证（使用运行时随机密钥）
    #[test]
    fn test_token_generation_and_validation() {
        let secret = test_jwt_secret();

        // 使用静态方法直接测试令牌生成和验证
        // 先生成一个令牌（通过编码）
        let now = Utc::now();
        let claims = AppClaims {
            sub: 1,
            username: "testuser".to_string(),
            role_id: Some(1),
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
    }

    /// 测试无效令牌验证
    #[test]
    fn test_invalid_token_validation() {
        let secret = test_jwt_secret();
        let wrong_secret = "wrong-secret-key-for-jwt-tokens-32-byte";

        let now = Utc::now();
        let claims = AppClaims {
            sub: 1,
            username: "testuser".to_string(),
            role_id: Some(1),
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
        let secret = test_jwt_secret();

        let now = Utc::now();
        let claims = AppClaims {
            sub: 1,
            username: "testuser".to_string(),
            role_id: Some(1),
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
        let secret = test_jwt_secret();

        let now = Utc::now();
        let claims = AppClaims {
            sub: 42,
            username: "admin".to_string(),
            role_id: Some(1),
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

    /// v11 批次 145 P1-7：测试 cleanup_revoked_users 清理过期吊销记录
    ///
    /// 由于 REVOKED_USERS 是进程内全局表，测试通过手动注入"过期"记录验证清理逻辑。
    /// 过期记录（revoked_at < now - TTL）应被清理，未过期记录应保留。
    #[tokio::test]
    async fn test_cleanup_revoked_users_removes_expired() {
        let test_user_id: i32 = 9_999_004;

        // 清理前置状态
        unrevoke_user(test_user_id).await;

        // 注入一条"过期"吊销记录（revoked_at = now - TTL - 1 小时）
        {
            let mut table = REVOKED_USERS.write().await;
            let expired_ts = chrono::Utc::now().timestamp() - REVOKED_USER_TTL_SECS - 3600;
            table.insert(test_user_id, expired_ts);
        }

        // 验证过期记录存在
        let old_iat = chrono::Utc::now().timestamp() - 60;
        assert!(
            is_user_token_revoked(test_user_id, old_iat).await,
            "过期吊销记录在清理前应仍存在"
        );

        // 执行清理
        let removed = cleanup_revoked_users().await;
        assert!(
            removed >= 1,
            "应至少清理 1 条过期记录（实际清理 {} 条）",
            removed
        );

        // 验证过期记录已被清理
        assert!(
            !is_user_token_revoked(test_user_id, old_iat).await,
            "清理后过期吊销记录应被移除"
        );
    }

    /// v11 批次 145 P1-7：测试 cleanup_revoked_users 保留未过期记录
    #[tokio::test]
    async fn test_cleanup_revoked_users_keeps_valid() {
        let test_user_id: i32 = 9_999_005;

        // 清理前置状态
        unrevoke_user(test_user_id).await;

        // 标记吊销（revoked_at = now，未过期）
        revoke_user_jtis(test_user_id, "USER_DEACTIVATED")
            .await
            .expect("revoke_user_jtis 不应失败");

        // 执行清理
        let _removed = cleanup_revoked_users().await;

        // 验证未过期记录仍存在
        let old_iat = chrono::Utc::now().timestamp() - 60;
        assert!(
            is_user_token_revoked(test_user_id, old_iat).await,
            "未过期吊销记录应被保留"
        );

        // 清理
        unrevoke_user(test_user_id).await;
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

    // ---------- 异步密码函数（批次 392 补测） ----------

    /// 测试异步哈希密码返回有效哈希
    ///
    /// 验证 hash_password_async 能正确哈希密码且结果以 argon2id 前缀开头
    #[tokio::test]
    async fn test_hash_password_async_returns_valid_hash() {
        let password = "AsyncTestPassword456!".to_string();
        let hash = AuthService::hash_password_async(password.clone())
            .await
            .expect("异步密码哈希应成功");
        assert!(!hash.is_empty(), "哈希结果不应为空");
        assert!(hash.starts_with("$argon2"), "哈希结果应以 argon2 前缀开头");
    }

    /// 测试异步验证密码正确与错误
    ///
    /// 验证 verify_password_async 对正确密码返回 true，错误密码返回 false
    #[tokio::test]
    async fn test_verify_password_async_correct_and_wrong() {
        let password = "VerifyAsync789!".to_string();
        let hash = AuthService::hash_password_async(password.clone())
            .await
            .expect("测试夹具：异步哈希失败");

        // 正确密码
        let ok = AuthService::verify_password_async(password.clone(), hash.clone())
            .await
            .expect("异步验证不应返回 Err");
        assert!(ok, "正确密码应验证通过");

        // 错误密码
        let wrong = AuthService::verify_password_async("WrongAsyncPassword".to_string(), hash)
            .await
            .expect("异步验证不应返回 Err");
        assert!(!wrong, "错误密码应验证失败");
    }

    /// 测试异步哈希密码唯一性
    ///
    /// 验证相同密码两次异步哈希结果不同（随机盐）
    #[tokio::test]
    async fn test_hash_password_async_uniqueness() {
        let password = "UniqueAsync123!".to_string();
        let hash1 = AuthService::hash_password_async(password.clone())
            .await
            .expect("第一次异步哈希应成功");
        let hash2 = AuthService::hash_password_async(password)
            .await
            .expect("第二次异步哈希应成功");
        assert_ne!(hash1, hash2, "相同密码两次哈希结果应不同（随机盐）");
    }

    /// 测试异步验证密码对无效哈希返回 Err
    ///
    /// 验证 verify_password_async 对非 argon2 格式的哈希返回 Err
    #[tokio::test]
    async fn test_verify_password_async_invalid_hash_returns_err() {
        let result =
            AuthService::verify_password_async("any".to_string(), "not-a-valid-hash".to_string())
                .await;
        assert!(result.is_err(), "无效哈希应返回 Err 而非 false");
    }
}
