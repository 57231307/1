//! 公开路径白名单
//!
//! 安全原则：最小化公开端点，仅保留认证必需和基础设施健康检查。
//! 所有业务端点必须经过 JWT 验证。

pub const PUBLIC_PATHS: &[&str] = &[
    // 基础设施健康检查（K8s / 负载均衡器探针，无需认证）
    "/health",
    "/ready",
    "/live",
    "/api/v1/erp/health",
    "/api/v1/erp/ready",
    "/api/v1/erp/live",
    // 认证流程必需端点
    "/api/v1/erp/auth/login",
    "/api/v1/erp/auth/refresh",
];

/// 公开路径白名单（跳过 JWT 认证）
///
/// ⚠️ **安全约束**：
/// 1. 仅放行真正不需要身份认证的端点（健康检查、登录、静态资源、初始化）
/// 2. **业务端点（如 `/dashboard`、`/sales`、`/inventory` 等）必须经过 JWT 验证**
/// 3. 任何新增条目都必须经过安全评审
pub fn is_public_path(path: &str) -> bool {
    // 低危 #3 修复：精确匹配 + 子路径匹配，防止 starts_with 误匹配
    // 例：原 /api/v1/erp/auth/logout 会匹配 /api/v1/erp/auth/logout-bypass
    // 修复后只匹配：
    //   - 精确路径：/api/v1/erp/auth/logout
    //   - 子路径：  /api/v1/erp/auth/logout/callback
    let clean_path = path.split(['?', '#']).next().unwrap_or(path);
    PUBLIC_PATHS.iter().any(|p| {
        clean_path == *p
            || (clean_path.starts_with(p) && clean_path[p.len()..].starts_with('/'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_paths_public() {
        // 健康检查端点必须公开（K8s / 负载均衡探针）
        assert!(is_public_path("/health"));
        assert!(is_public_path("/ready"));
        assert!(is_public_path("/live"));
        assert!(is_public_path("/api/v1/erp/health"));
        assert!(is_public_path("/api/v1/erp/ready"));
        assert!(is_public_path("/api/v1/erp/live"));
        // 登录/刷新必须公开（认证流程）
        assert!(is_public_path("/api/v1/erp/auth/login"));
        assert!(is_public_path("/api/v1/erp/auth/refresh"));
    }

    #[test]
    fn test_business_paths_require_auth() {
        // 业务路径必须经过认证
        assert!(!is_public_path("/api/v1/erp/dashboard"));
        assert!(!is_public_path("/api/v1/erp/sales/orders"));
        assert!(!is_public_path("/api/v1/erp/inventory/stocks"));
        assert!(!is_public_path("/api/v1/erp/crm/customers"));
        // init / tracking / logout 均需认证
        assert!(!is_public_path("/init"));
        assert!(!is_public_path("/api/v1/erp/init"));
        assert!(!is_public_path("/api/tracking/page-view"));
        assert!(!is_public_path("/api/v1/erp/auth/logout"));
    }

    /// 低危 #3 修复：精确匹配防止 starts_with 误匹配
    #[test]
    fn test_public_paths_strict_prefix() {
        // 子路径应匹配（合法）
        assert!(is_public_path("/api/v1/erp/auth/login/sub"));
        // query string 后的子路径应匹配
        assert!(is_public_path("/api/v1/erp/auth/login?next=/dashboard"));
        // 路径变体（-xxx）不应匹配
        assert!(!is_public_path("/api/v1/erp/auth/login-bypass"));
        assert!(!is_public_path("/health-extra"));
        assert!(!is_public_path("/readyz"));
    }
}
