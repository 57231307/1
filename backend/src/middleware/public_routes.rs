//! 公开路径白名单
//!
//! 安全原则：最小化公开端点，仅保留认证必需和基础设施健康检查。
//! 所有业务端点必须经过 JWT 验证。

pub const PUBLIC_PATHS: &[&str] = &[
    // 基础设施健康检查（负载均衡器 / 监控探针，无需认证）
    "/health",
    "/ready",
    "/live",
    "/api/v1/erp/health",
    "/api/v1/erp/ready",
    "/api/v1/erp/live",
    "/health/liveness",
    "/health/readiness",
    // 认证流程必需端点
    "/api/v1/erp/auth/login",
    "/api/v1/erp/auth/refresh",
    // 安全审计 P1-08-1：登录页 handleUsernameBlur 预检查账号锁定状态
    // （GET /security/lock-status），属登录前公开接口（未登录时无 cookie），
    // 必须放行否则 401 → axios refresh 拦截 → UI 被刷新流程阻塞（登录按钮 click 超时）
    "/api/v1/erp/lock-status",
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

/// 公开路径白名单（跳过 JWT 认证）；⚠️ **安全约束**： 1. 仅放行真正不需要身份认证的端点（健康检查、登录、静态资源、初始化） 2.
/// **业务端点（如 `/dashboard`、`/sales`、`/inventory` 等）必须经过 JWT 验证** 3. 任何新增条目都必须经过安全评审
pub fn is_public_path(path: &str) -> bool {
    // P1-03-2 修复：严格精确匹配，删除子路径前缀匹配
    // 原 starts_with + 子路径匹配会放行 /api/v1/erp/auth/login/anything 等子路径，
    // 若未来新增 /api/v1/erp/auth/login/{id} 等业务接口将绕过认证。
    // 改为仅精确匹配，如确需子路径公开，单独显式登记到 PUBLIC_PATHS。
    let clean_path = path.split(['?', '#']).next().unwrap_or(path);
    PUBLIC_PATHS.contains(&clean_path)
}
