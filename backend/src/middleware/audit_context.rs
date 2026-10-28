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
    http::{Request, header},
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
}

/// 无法解析客户端 IP 时的占位值（不是可用的客户端标识，禁止当作真实 IP 参与任何判定）
pub const CLIENT_IP_UNRESOLVED: &str = "unknown";

/// 客户端 IP 的来源档位。用途：CSRF Token 的 IP 漂移留痕需要标注这条 IP 是谁给的，
/// 因为 `X-Forwarded-For` 首段在客户端可自填（Nginx 用 `$proxy_add_x_forwarded_for` 追加，
/// 不清空客户端自带值），而 `X-Real-IP` 由边缘 Nginx 以 `$remote_addr` 覆写，客户端伪造不进去。
/// 见 deploy/nginx.conf 与 frontend/nginx.conf 的 `location /api/` 段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientIpSource {
    /// 边缘反向代理覆写的 `X-Real-IP`（本仓部署形态下的可信来源）
    RealIpHeader,
    /// `X-Forwarded-For` 首段（客户端可控，仅在无 `X-Real-IP` 时作为降级来源）
    ForwardedForHeader,
    /// TCP 连接对端地址（直连部署下就是客户端 IP；经反向代理部署下是代理自身 IP）
    ConnectPeer,
    /// 三级来源全部缺失，值为 [CLIENT_IP_UNRESOLVED] 占位
    Unresolved,
}

impl ClientIpSource {
    /// 落审计与日志用的稳定字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RealIpHeader => "x-real-ip",
            Self::ForwardedForHeader => "x-forwarded-for",
            Self::ConnectPeer => "connect-info",
            Self::Unresolved => "unresolved",
        }
    }
}

/// 客户端 IP 提取的唯一实现：按 HeaderMap 与可选的连接对端地址走同一条降级链。
/// 登录/刷新等只持有 HeaderMap 与 ConnectInfo 的 handler 也走这里，
/// 避免同一套优先级出现第二份实现（两份各自演进会让绑定 IP 与校验 IP 系统性错配）。
pub fn client_ip_from_parts(
    headers: &header::HeaderMap,
    peer_ip: Option<std::net::IpAddr>,
) -> (String, ClientIpSource) {
    if let Some(real_ip) = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        return (real_ip.to_string(), ClientIpSource::RealIpHeader);
    }
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return (trimmed.to_string(), ClientIpSource::ForwardedForHeader);
            }
        }
    }
    if let Some(ip) = peer_ip {
        return (ip.to_string(), ClientIpSource::ConnectPeer);
    }
    (CLIENT_IP_UNRESOLVED.to_string(), ClientIpSource::Unresolved)
}

/// 从完整请求提取客户端 IP 与来源档位（依次 X-Real-IP、X-Forwarded-For 首段、ConnectInfo、占位）
pub fn extract_client_ip_with_source(request: &Request<Body>) -> (String, ClientIpSource) {
    let peer_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip());
    client_ip_from_parts(request.headers(), peer_ip)
}

/// 提取客户端 IP（依次 X-Real-IP、X-Forwarded-For、ConnectInfo）
/// 公开为统一 helper，入参当前请求，传给 auth_middleware 与认证处理腿复用，返回 IP 文本
pub fn extract_client_ip(request: &Request<Body>) -> String {
    extract_client_ip_with_source(request).0
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

/// 审计上下文中间件；必须在 `trace_context` 之后挂载（注册顺序 = 执行顺序的逆序， 即先注册 trace_context，后注册 audit_context 才能保证 trace_id 已就绪）。
pub async fn audit_context_middleware(mut request: Request<Body>, next: Next) -> Response {
    // 复用 trace_context 注入的 trace_id；找不到则本地生成 UUID v4
    let request_id = request
        .extensions()
        .get::<TraceContext>()
        .map(|ctx| ctx.trace_id.clone())
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string());

    let ip_address = extract_client_ip(&request);
    let user_agent = extract_user_agent(&request);

    let audit_ctx = AuditContext {
        request_id,
        ip_address,
        user_agent,
    };

    request.extensions_mut().insert(audit_ctx);
    next.run(request).await
}
