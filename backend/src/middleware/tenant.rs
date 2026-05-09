//! 租户识别中间件
//!
//! 从请求头或子域名中提取租户标识，并注入租户上下文
#![allow(dead_code)]

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::utils::app_state::AppState;
use crate::middleware::auth_context::AuthContext;

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
    let tenant_id_from_auth = request.extensions()
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
