use bingxi_backend::middleware::init_token::init_token_middleware;
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
    // 前端路由守卫 checkInitStatus 在登录前调用 GET /init/status 判断系统是否初始化
    // （公开只读接口，仅返回布尔值不泄露敏感信息），必须放行：
    // 未放行时 401 → 守卫判 initialized=false → 已初始化系统也被重定向 /setup → 页面内容不渲染
    assert!(is_public_path("/api/v1/erp/init/status"));
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
    // task-status 仍需 JWT 认证（handler 内 admin 二次校验）
    assert!(!is_public_path("/api/v1/erp/init/task-status"));
    assert!(!is_public_path("/api/tracking/page-view"));
    assert!(!is_public_path("/api/v1/erp/auth/logout"));
}

/// test-database 公开放行（Setup 向导匿名场景）的分层门禁契约：
/// 路由层公开，handler 层 OptionalAuthContext 门禁——未初始化时匿名放行
/// （Setup 向导无用户体系可登录），已初始化时必须有 admin 身份；
/// port/内网 IP 校验对匿名与登录请求一视同仁（SSRF 防护不因匿名放宽）。
/// 公开性由 init_handler::validate_admin_role / validate_not_initialized /
/// validate_internal_ip 三层函数保证，此处仅断言路由层放行 + test-status 反差。
#[test]
fn test_init_test_database_public_with_handler_guards() {
    // Setup 向导"测试数据库连接"公开（原"仍需 JWT"设计使向导必然 401）
    assert!(is_public_path("/api/v1/erp/init/test-database"));
    // 精确匹配：变体路径不放行
    assert!(!is_public_path("/api/v1/erp/init/test-database-async"));
    assert!(!is_public_path("/api/v1/erp/init/test-database/sub"));
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
