//! 认证上下文模块
//!
//! 提供从 JWT Token 提取用户信息的功能

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::services::auth_service::AppClaims;

/// 认证错误响应
#[derive(Debug)]
pub struct AuthRejection {
    pub status: StatusCode,
    pub message: String,
}

impl AuthRejection {
    pub fn new(status: StatusCode, message: &str) -> Self {
        Self {
            status,
            message: message.to_string(),
        }
    }

    pub fn unauthorized(message: &str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "Unauthorized",
            "message": self.message
        });
        (self.status, Json(body)).into_response()
    }
}

/// 用户认证信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// 用户 ID
    pub user_id: i32,
    /// 用户名
    pub username: String,
    /// 角色 ID
    pub role_id: Option<i32>,
}

impl AuthContext {
    pub fn from_claims(claims: AppClaims) -> Self {
        Self {
            user_id: claims.sub,
            username: claims.username,
            role_id: claims.role_id,
        }
    }
}

impl From<AppClaims> for AuthContext {
    fn from(claims: AppClaims) -> Self {
        Self::from_claims(claims)
    }
}

/// 为 AuthContext 实现 FromRequestParts，使其可以作为 axum 的提取器
/// 从请求扩展中获取认证信息（由中间件注入）
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or(AuthRejection::unauthorized("未授权：缺少认证信息"))
    }
}

/// 可选的认证上下文（允许未认证的请求）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalAuthContext {
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub role_id: Option<i32>,
}

impl OptionalAuthContext {
    #[allow(dead_code)]
    pub fn from_claims(claims: AppClaims) -> Self {
        Self {
            user_id: Some(claims.sub),
            username: Some(claims.username),
            role_id: claims.role_id,
        }
    }

    /// 创建空的 OptionalAuthContext
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            user_id: None,
            username: None,
            role_id: None,
        }
    }
}

/// 认证上下文提取器类型别名（使用 Extension）
#[allow(dead_code)]
pub type Auth = axum::extract::Extension<AuthContext>;

/// 可选认证上下文提取器类型别名（使用 Extension）
#[allow(dead_code)]
pub type OptionalAuth = axum::extract::Extension<OptionalAuthContext>;
