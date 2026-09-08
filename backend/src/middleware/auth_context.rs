//! 认证上下文模块
//!
//! 提供从 JWT Token 提取用户信息的功能

use crate::services::auth_service::AppClaims;
use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    /// V15 P0-S01 新增：部门 ID（行级数据权限 dept 范围使用）
    /// 由权限中间件从 user 表查询注入，None 表示用户未分配部门
    pub department_id: Option<i32>,
    /// V15 P0-S01 新增：数据范围（行级数据权限） 由权限中间件从 role 表查询注入，"all"/"dept"/"self" None 表示未加载（此时 service 层应按 self 处理，最小权限原则）
    pub data_scope: Option<String>,
    /// RLS dept 语义（m_rls_dept_domain）：可见部门集合的逗号分隔串
    /// （主部门 + 兼职部门 + 子部门，由 auth 中间件调
    /// data_permission_service.get_user_dept_scope_ids_cached 解析）。
    /// 仅 data_scope=dept 用户加载；all/self 用户为 None。RLS 中间件据此
    /// 构造 RlsGuc.dept_ids 写入 task-local，连接池钩子设置 app.dept_ids GUC。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dept_ids: Option<Arc<String>>,
}

impl AuthContext {
    pub fn from_claims(claims: AppClaims) -> Self {
        Self {
            user_id: claims.sub,
            username: claims.username,
            role_id: claims.role_id,
            // V15 P0-S01：data_scope/department_id/dept_ids 由权限中间件从数据库加载后注入
            department_id: None,
            data_scope: None,
            dept_ids: None,
        }
    }

    /// V15 P0-S01 新增：构建数据范围上下文；从 AuthContext 提取 DataScopeContext，用于
    /// service 层调用 apply_data_scope。 若 data_scope 未加载，默认按 Self_ 处理（最小权限原则）。
    pub fn to_data_scope_context(&self) -> crate::utils::data_scope::DataScopeContext {
        use crate::utils::data_scope::{DataScope, DataScopeContext};

        let scope = self
            .data_scope
            .as_deref()
            .map(DataScope::parse_scope)
            .unwrap_or(DataScope::Self_);

        // dept_ids CSV 解析为 Vec<i32>：dept 用户的可见部门集合；self/all 为空
        let dept_ids = self
            .dept_ids
            .as_ref()
            .map(|csv| {
                csv.split(',')
                    .filter_map(|s| s.trim().parse::<i32>().ok())
                    .collect::<Vec<i32>>()
            })
            .unwrap_or_default();

        DataScopeContext {
            scope,
            user_id: self.user_id,
            department_id: self.department_id,
            dept_ids,
        }
    }
}

impl From<AppClaims> for AuthContext {
    fn from(claims: AppClaims) -> Self {
        Self::from_claims(claims)
    }
}

/// 为 OptionalAuthContext 实现 FromRequestParts：auth_middleware 注入了
/// AuthContext 则映射为有值上下文，未挂载/未认证（Setup 模式、公开路径）时
/// 得到 empty 上下文而非 401——供匿名可访问但需在 handler 内自检门禁的端点使用
/// （如 /init/test-database：未初始化时匿名放行，已初始化时 handler 内拒收）
impl<S> FromRequestParts<S> for OptionalAuthContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .map(|a| OptionalAuthContext {
                user_id: Some(a.user_id),
                username: Some(a.username.clone()),
                role_id: a.role_id,
            })
            .unwrap_or_else(OptionalAuthContext::empty))
    }
}

/// 为 AuthContext 实现 FromRequestParts，使其可以作为 axum 的提取器
/// 从请求扩展中获取认证信息（由中间件注入）
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
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

#[allow(dead_code)]
impl OptionalAuthContext {
    pub fn from_claims(claims: AppClaims) -> Self {
        Self {
            user_id: Some(claims.sub),
            username: Some(claims.username),
            role_id: claims.role_id,
        }
    }

    /// 创建空的 OptionalAuthContext
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
