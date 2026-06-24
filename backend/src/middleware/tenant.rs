//! 租户识别中间件
//!
//! 从请求头或子域名中提取租户标识，并注入租户上下文

use crate::middleware::auth_context::AuthContext;
use crate::utils::error::AppError;

/// 从认证上下文中安全提取租户 ID
///
/// 若 `tenant_id` 为 `None`，返回未授权错误，防止跨租户访问。
pub fn extract_tenant_id(auth: &AuthContext) -> Result<i32, AppError> {
    auth.tenant_id
        .ok_or_else(|| AppError::unauthorized("缺少租户 ID，请重新登录"))
}

/// 租户上下文
#[allow(dead_code)] // TODO(tech-debt): 中间件注入租户上下文功能接入后移除
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: i32,
    pub tenant_code: String,
    pub is_active: bool,
}

