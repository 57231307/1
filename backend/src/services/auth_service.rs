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
    cleanup_expired_jti, is_jti_revoked, is_user_token_revoked, revoke_jti, revoke_user_jtis,
    start_revoked_user_cleanup_task, unrevoke_user,
};

/// JWT 令牌声明（包含用户身份信息和令牌元数据）
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

/// 认证服务（处理用户认证、令牌生成和验证；字段声明为 `pub(crate)` 以便 `auth_service_ops::auth` 子模块的 `impl AuthService`；块直接访问（业务方法已迁移至该子模块）。）
#[derive(Clone)]
pub struct AuthService {
    pub(crate) db: Arc<DatabaseConnection>,
    pub(crate) encoding_key: EncodingKey,
}

impl AuthService {
    /// 创建新的认证服务实例（# 参数；`db`: 数据库连接；`secret`: JWT 密钥）
    pub fn new(db: Arc<DatabaseConnection>, secret: String) -> Self {
        Self {
            db,
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
        }
    }
}

/// 认证错误类型（定义认证过程中可能发生的所有错误）
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
