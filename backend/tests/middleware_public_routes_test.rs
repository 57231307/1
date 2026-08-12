#[cfg(test)]
mod tests {
use bingxi_backend::middleware::public_routes::*;


    #[test]
    fn test_health_paths_public() {
        // 健康检查端点必须公开（负载均衡 / 监控探针）
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
        assert!(is_public_path("/api/v1/erp/webhooks/integrations/callback"));
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

    /// P1-03-2 修复：严格精确匹配，子路径不再放行
    #[test]
    fn test_public_paths_strict_exact() {
        // 精确路径匹配
        assert!(is_public_path("/api/v1/erp/auth/login"));
        // query string 后仍匹配（query 已 split 去除）
        assert!(is_public_path("/api/v1/erp/auth/login?next=/dashboard"));
        // 子路径不再匹配（P1-03-2 修复：删除子路径放行）
        assert!(!is_public_path("/api/v1/erp/auth/login/sub"));
        assert!(!is_public_path("/api/v1/erp/auth/login/callback"));
        // 路径变体（-xxx）不匹配
        assert!(!is_public_path("/api/v1/erp/auth/login-bypass"));
        assert!(!is_public_path("/health-extra"));
        assert!(!is_public_path("/readyz"));
    }
}