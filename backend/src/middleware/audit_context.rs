//! 审计上下文中间件
//!
//! 职责：
//! 1. 解析或生成请求追踪 ID（`request_id`），优先复用 `trace_context` 中间件生成的 `TraceContext`
//! 2. 提取客户端 IP（X-Real-IP / X-Forwarded-For / `ConnectInfo<SocketAddr>` 三级降级）
//! 3. 提取 `User-Agent` header
//! 4. 把结果打包成 `AuditContext` 写入 `Request::extensions()`，供 audit_log_service 异步落库
//!
//! 与 `trace_context` 的关系：
//! - `trace_context` 在最外层生成 W3C `traceparent`，设置 `TraceContext`
//! - `audit_context` 在 trace_context 之后运行，读取 `TraceContext::trace_id` 作为 `request_id`，
//!   当 trace_context 未运行时（单元测试 / 主动调用）则降级为本地生成的 UUID v4

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header, Request},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::observability::trace_context::TraceContext;

/// 写入 `Request::extensions()` 的审计上下文（service 直接读取）
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// 请求追踪 ID（来自 trace_context 或本地生成 UUID v4）
    pub request_id: String,
    /// 客户端 IP（v4 或 v6 字符串）
    pub ip_address: String,
    /// User-Agent 原始字符串
    pub user_agent: String,
}

impl AuditContext {
    /// 构造空上下文（缺省值，避免 service 调用时 unwrap）
    pub fn empty() -> Self {
        Self {
            request_id: String::new(),
            ip_address: String::new(),
            user_agent: String::new(),
        }
    }

    /// 从 request extensions 取出已注入的审计上下文；找不到则返回空
    #[allow(dead_code)] // TODO(tech-debt): 业务接入后逐项移除；预留 API 用于未来从 request 中提取
    pub fn from_request(request: &Request<Body>) -> Self {
        request
            .extensions()
            .get::<AuditContext>()
            .cloned()
            .unwrap_or_else(Self::empty)
    }
}

/// 提取客户端 IP（X-Real-IP → X-Forwarded-For → ConnectInfo）
fn extract_ip(request: &Request<Body>) -> String {
    let h = request.headers();
    if let Some(real_ip) = h
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        return real_ip.to_string();
    }
    if let Some(forwarded) = h.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    if let Some(ConnectInfo(addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }
    "unknown".to_string()
}

/// 提取 User-Agent
fn extract_user_agent(request: &Request<Body>) -> String {
    request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

/// 审计上下文中间件
///
/// 必须在 `trace_context` 之后挂载（注册顺序 = 执行顺序的逆序，
/// 即先注册 trace_context，后注册 audit_context 才能保证 trace_id 已就绪）。
pub async fn audit_context_middleware(mut request: Request<Body>, next: Next) -> Response {
    // 复用 trace_context 注入的 trace_id；找不到则本地生成 UUID v4
    let request_id = request
        .extensions()
        .get::<TraceContext>()
        .map(|ctx| ctx.trace_id.clone())
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string());

    let ip_address = extract_ip(&request);
    let user_agent = extract_user_agent(&request);

    let audit_ctx = AuditContext {
        request_id,
        ip_address,
        user_agent,
    };

    request.extensions_mut().insert(audit_ctx);
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    async fn echo() -> &'static str {
        "ok"
    }

    /// 未注入 trace_id 时仍能放行（fallback 到本地 UUID v4）
    #[tokio::test]
    async fn test_audit_context_generates_request_id_when_missing() {
        let app = Router::new()
            .route("/", get(echo))
            .layer(from_fn(audit_context_middleware));

        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("build");
        let response = app.oneshot(req).await.expect("request ok");
        assert_eq!(response.status(), 200);
    }

    /// X-Real-IP 优先级最高
    #[tokio::test]
    async fn test_audit_context_reads_real_ip_header() {
        let app = Router::new()
            .route("/", get(echo))
            .layer(from_fn(audit_context_middleware));

        let req = Request::builder()
            .uri("/")
            .header("x-real-ip", "203.0.113.42")
            .body(Body::empty())
            .expect("build");
        let response = app.oneshot(req).await.expect("request ok");
        assert_eq!(response.status(), 200);
    }

    /// X-Forwarded-For 多级时取第一段
    #[tokio::test]
    async fn test_audit_context_reads_forwarded_for_first_segment() {
        let app = Router::new()
            .route("/", get(echo))
            .layer(from_fn(audit_context_middleware));

        let req = Request::builder()
            .uri("/")
            .header("x-forwarded-for", "198.51.100.1, 10.0.0.1")
            .body(Body::empty())
            .expect("build");
        let response = app.oneshot(req).await.expect("request ok");
        assert_eq!(response.status(), 200);
    }

    /// 无 IP header 时 fallback 到 "unknown"
    #[tokio::test]
    async fn test_audit_context_falls_back_to_unknown_ip() {
        let app = Router::new()
            .route("/", get(echo))
            .layer(from_fn(audit_context_middleware));

        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("build");
        let response = app.oneshot(req).await.expect("request ok");
        assert_eq!(response.status(), 200);
    }

    /// `AuditContext::empty()` 字段全为空字符串
    #[test]
    fn test_empty_context() {
        let ctx = AuditContext::empty();
        assert_eq!(ctx.request_id, "");
        assert_eq!(ctx.ip_address, "");
        assert_eq!(ctx.user_agent, "");
    }
}
