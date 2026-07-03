pub mod api_gateway;
pub mod audit_context;
pub mod auth;
pub mod auth_context;
pub mod csrf;
pub mod data_permission;
pub mod metrics;
pub mod omni_audit;
pub mod permission;
pub mod public_routes;
pub mod rate_limit;
pub mod request_validator;
// P4-1 性能优化 - 慢查询审计
pub mod slow_query;
// P4-2 安全加固 - CSP 中间件
pub mod csp;
pub mod sql_injection_audit;
pub mod timeout;
pub mod trace_context;
// P1 修复：init 端点 token 校验中间件（bug.md #3）
pub mod init_token;
// v3 P2-7~P2-10：删除 4 个未挂载的 dead middleware 文件
// operation_log / trace / logger_middleware / security_headers
// 功能已分别被 omni_audit_middleware / trace_context_middleware / TraceLayer / SetResponseHeaderLayer 替代
