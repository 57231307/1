//! 路径白名单（单一真相源）
//!
//! 安全原则：最小化公开端点，仅保留认证必需和基础设施健康检查。
//! 所有业务端点必须经过 JWT 验证。
//!
//! 两类清单语义不同，均集中在此模块维护，避免各中间件各存一份副本而产生豁免漂移：
//! - [`PUBLIC_PATHS`]：完全匿名，跳过 JWT 认证
//! - [`AUTH_ONLY_PATHS`]：仍需 JWT 认证，但跳过 CSRF 一次性 token 消费与 RBAC 权限码校验

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
    // 路由守卫 checkInitStatus 在每次导航调 GET /init/status 判断系统是否初始化，
    // 属公开只读接口（仅返回布尔值，不泄露敏感信息），未登录/登录后均需访问。
    // 未放行时 401 → 守卫判 initialized=false → 重定向 /setup → 页面内容不渲染
    "/api/v1/erp/init/status",
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
    // Setup 向导"测试数据库连接"：未初始化时无用户体系可登录获取 JWT，
    // 原"仍需 JWT"设计使向导必然 401（Cookie=false/false/Header=false）。
    // 安全边界由 handler 门禁保证：validate_not_initialized（初始化后拒绝）
    // + 已初始化时 validate_admin_role（仅限管理员）。handler 内认证上下文
    // 改为 OptionalAuthContext（未初始化时匿名通过，已初始化必须有 admin 身份）。
    "/api/v1/erp/init/test-database",
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

/// 认证豁免清单：已通过 JWT 认证后，额外跳过 CSRF 一次性 token 消费与 RBAC 权限码校验。
///
/// ⚠️ **安全约束**：新增条目必须经安全评审，且必须逐条论证"无业务写副作用"——
/// 豁免 CSRF 意味着该端点接受跨站携带 cookie 的请求，任何真实的写操作都可能被伪造。
///
/// - `/audit-logs/record-print`：前端打印审计埋点，仅追加一条 PRINT 审计记录，
///   不改动任何业务数据；任何已认证用户均可上报。
/// - `/ws/ticket`：WebSocket 一次性票据签发。票据本身即属"鉴权材料"而非业务写操作，
///   鉴权强依赖 JWT。若不豁免，WS 重连风暴（1~30s 退避）会与页面其它 POST 争抢
///   一次性 CSRF token，把并发请求全部打成 CSRF_TOKEN_INVALID。
pub const AUTH_ONLY_PATHS: &[&str] = &[
    "/api/v1/erp/audit-logs/record-print",
    "/api/v1/erp/ws/ticket",
];

/// 路径是否仅需认证（严格精确匹配，语义与 [`is_public_path`] 一致，不做子路径前缀放行）
pub fn is_auth_only_path(path: &str) -> bool {
    let clean_path = path.split(['?', '#']).next().unwrap_or(path);
    AUTH_ONLY_PATHS.contains(&clean_path)
}
