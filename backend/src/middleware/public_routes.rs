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
    // 批次 110 P0-1：Webhook 回调端点（第三方平台调用，无法持有 JWT）
    // 安全等价：handle_generic_callback 内部通过 HMAC-SHA256 签名验证替代认证
    // （X-Webhook-Signature 头 + webhook_secret 密钥校验）
    "/api/v1/erp/webhooks/integrations/callback",
    // 批次 261 修复：初始化高危接口放行 JWT 认证，由 init_token_middleware
    // 用 X-Init-Token（恒定时间比较）替代认证。
    // 设计意图：系统首次部署时数据库无 users 表，无法登录获取 JWT，
    // 需用 X-Init-Token 替代。只放行 initialize 系列（高危接口已受
    // init_token_middleware 保护），只读接口（status/test-database/
    // task-status）仍需 JWT 认证。
    "/api/v1/erp/init/initialize",
    "/api/v1/erp/init/initialize-with-db",
    "/api/v1/erp/init/initialize-with-db-async",
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
        // 批次 110 P0-1：Webhook 回调端点公开（HMAC 签名验证替代认证）
        assert!(is_public_path(
            "/api/v1/erp/webhooks/integrations/callback"
        ));
        // 批次 261 修复：initialize 系列高危接口放行 JWT 认证（由 init_token_middleware 认证）
        assert!(is_public_path("/api/v1/erp/init/initialize"));
        assert!(is_public_path("/api/v1/erp/init/initialize-with-db"));
        assert!(is_public_path("/api/v1/erp/init/initialize-with-db-async"));
    }

    #[test]
    fn test_business_paths_require_auth() {
        // 业务路径必须经过认证
        assert!(!is_public_path("/api/v1/erp/dashboard"));
        assert!(!is_public_path("/api/v1/erp/sales/orders"));
        assert!(!is_public_path("/api/v1/erp/inventory/stocks"));
        assert!(!is_public_path("/api/v1/erp/crm/customers"));
        // init 根路径 / tracking / logout 均需认证（initialize 系列除外，由 init_token_middleware 认证）
        assert!(!is_public_path("/init"));
        assert!(!is_public_path("/api/v1/erp/init"));
        // 只读 init 接口仍需 JWT 认证（test-database/task-status 有 admin 二次校验）
        assert!(!is_public_path("/api/v1/erp/init/status"));
        assert!(!is_public_path("/api/v1/erp/init/test-database"));
        assert!(!is_public_path("/api/v1/erp/init/task-status"));
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
