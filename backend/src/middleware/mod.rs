pub mod api_gateway;
pub mod auth;
pub mod auth_context;
pub mod data_permission;
pub mod logger_middleware;
pub mod metrics;
pub mod omni_audit;
pub mod operation_log;
pub mod permission;
pub mod public_routes;
pub mod rate_limit;
pub mod request_validator;
pub mod security_headers;
// P4-1 性能优化 - 慢查询审计
pub mod slow_query;
// P4-2 安全加固 - CSP 中间件
pub mod csp;
pub mod sql_injection_audit;
pub mod tenant;
pub mod timeout;
pub mod trace_context;
// P9-6 OpenTelemetry HTTP 追踪中间件
pub mod trace;
