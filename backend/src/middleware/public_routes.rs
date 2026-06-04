pub const PUBLIC_PATHS: &[&str] = &[
    "/health",
    "/ready",
    "/live",
    "/init",
    "/api/v1/erp/health",
    "/api/v1/erp/ready",
    "/api/v1/erp/live",
    "/api/v1/erp/init",
    "/api/v1/erp/auth/login",
    "/api/v1/erp/auth/refresh",
    "/api/v1/erp/auth/logout",
    "/api/tracking/page-view",
];

/// 公开路径白名单（跳过 JWT 认证）
///
/// ⚠️ **安全约束**：
/// 1. 仅放行真正不需要身份认证的端点（健康检查、登录、静态资源、初始化）
/// 2. **业务端点（如 `/dashboard`、`/sales`、`/inventory` 等）必须经过 JWT 验证**
/// 3. 任何新增条目都必须经过安全评审
pub fn is_public_path(path: &str) -> bool {
    PUBLIC_PATHS.iter().any(|p| path.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_paths_public() {
        assert!(is_public_path("/health"));
        assert!(is_public_path("/api/v1/erp/health"));
        assert!(is_public_path("/api/v1/erp/auth/login"));
    }

    #[test]
    fn test_business_paths_require_auth() {
        // 业务路径必须经过认证
        assert!(!is_public_path("/api/v1/erp/dashboard"));
        assert!(!is_public_path("/api/v1/erp/sales/orders"));
        assert!(!is_public_path("/api/v1/erp/inventory/stocks"));
        assert!(!is_public_path("/api/v1/erp/crm/customers"));
    }
}
