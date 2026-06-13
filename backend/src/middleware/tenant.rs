//! 租户识别中间件
//!
//! 从请求头或子域名中提取租户标识，并注入租户上下文
#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// 从认证上下文中安全提取租户 ID
///
/// 若 `tenant_id` 为 `None`，返回未授权错误，防止跨租户访问。
pub fn extract_tenant_id(auth: &AuthContext) -> Result<i32, AppError> {
    auth.tenant_id.ok_or_else(|| {
        AppError::unauthorized("缺少租户 ID，请重新登录")
    })
}

/// 租户上下文
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: i32,
    pub tenant_code: String,
    pub is_active: bool,
}

/// 租户识别中间件
/// 优先级：1. X-Tenant-ID Header  2. X-Tenant-Code Header  3. AuthContext中的tenant_id
pub async fn tenant_middleware(
    State(_state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 尝试从 Header 获取租户标识
    let tenant_id = request
        .headers()
        .get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i32>().ok());

    let tenant_code = request
        .headers()
        .get("X-Tenant-Code")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // 如果 Header 中没有，尝试从 AuthContext 获取
    let tenant_id_from_auth = request
        .extensions()
        .get::<AuthContext>()
        .and_then(|auth| auth.tenant_id);

    // 确定最终租户ID
    let final_tenant_id = tenant_id.or(tenant_id_from_auth);

    // 注入租户上下文
    if let Some(tid) = final_tenant_id {
        request.extensions_mut().insert(TenantContext {
            tenant_id: tid,
            tenant_code: tenant_code.unwrap_or_default(),
            is_active: true,
        });
    }

    Ok(next.run(request).await)
}
