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
    /// 操作终端类型（PC / H5 / MINIPROGRAM / UNKNOWN），中间件按请求头与 UA 判定，取不到记 UNKNOWN
    pub terminal_type: String,
    /// 操作页面标识（优先 X-Page-Path，回退 Referer 的片段或路径），取不到为 None
    pub operation_page: Option<String>,
    /// 单据编号（业务单号），业务侧经 set_audit_doc_no 登记，未登记为 None
    pub doc_no: Option<String>,
    /// 变更原因备注，业务侧经 set_audit_change_reason 登记，未登记为 None
    pub change_reason: Option<String>,
    /// 高风险操作标记，业务侧经 mark_audit_high_risk 登记；删除由写入单点自动补 DELETE
    pub risk_flag: Option<String>,
}

/// 终端类型取值（读取侧过滤与写入判定共用同一词表，避免三端漂移）
pub const TERMINAL_PC: &str = "PC";
/// 移动端浏览器与 App WebView 内的 H5 页面
pub const TERMINAL_H5: &str = "H5";
/// 微信小程序端（带 terminalSource 或 terminalSourceV2 或 App-User-Agent 自定义头）
pub const TERMINAL_MINIPROGRAM: &str = "MINIPROGRAM";
/// 三档都判不出时的如实取值，禁止用它兜底成 PC
pub const TERMINAL_UNKNOWN: &str = "UNKNOWN";

/// 高风险操作标记取值（单独可筛维度，不只靠 operation_type 文案匹配）
pub const RISK_DELETE: &str = "DELETE";
/// 红冲
pub const RISK_REVERSE: &str = "REVERSE";
/// 作废
pub const RISK_VOID: &str = "VOID";

impl AuditContext {
    /// 构造空上下文（缺省值，避免 service 调用时 unwrap）；终端记 UNKNOWN（不默认 PC）
    pub fn empty() -> Self {
        Self {
            request_id: String::new(),
            ip_address: String::new(),
            user_agent: String::new(),
            terminal_type: TERMINAL_UNKNOWN.to_string(),
            operation_page: None,
            doc_no: None,
            change_reason: None,
            risk_flag: None,
        }
    }
}

/// 请求级审计上下文（task-local，同步写入单点与异步发送点共用）。
///
/// 生命周期：由 `audit_context_middleware` 在解析出终端类型与页面之后用
/// `AUDIT_REQUEST_CTX.scope(RefCell::new(ctx), next.run(req)).await` 绑定，覆盖整个请求
/// future。业务侧在受理请求的 handler 内可按需把本张单的单据编号与变更原因与风险标记
/// 写进这枚活对象（RefCell），同步写入点与异步发送点都能读到最新值。
///
/// 形态：`RefCell` 只在同步临界区借用（borrow 不跨 await），随请求 task 移动要求
/// `Send`，AuditContext 各字段皆 String 与 Option<String>，满足 Send。
tokio::task_local! {
    pub static AUDIT_REQUEST_CTX: std::cell::RefCell<AuditContext>;
}

/// 读取当前请求的审计上下文快照（含业务侧已登记的单号与原因与风险）。
/// 未绑定（后台任务 / spawn 旁路 / 位于 audit_context 中间件外层）返回 None，
/// 由调用方按 UNKNOWN 终端如实落库，绝不臆造。
pub fn current_audit_request_ctx() -> Option<AuditContext> {
    AUDIT_REQUEST_CTX
        .try_with(|cell| cell.borrow().clone())
        .ok()
}

/// 业务侧登记本次操作的单据编号（同一请求多次调用以最后一次为准）；未在请求作用域内调用为无害空操作。
pub fn set_audit_doc_no(doc_no: Option<String>) {
    let _ = AUDIT_REQUEST_CTX.try_with(|cell| cell.borrow_mut().doc_no = doc_no);
}

/// 业务侧登记本次操作的变更原因；调用方没给就不调用，落库保持 NULL（禁止默认文案）。
pub fn set_audit_change_reason(reason: Option<String>) {
    let _ = AUDIT_REQUEST_CTX.try_with(|cell| cell.borrow_mut().change_reason = reason);
}

/// 业务侧标记本次为高风险操作（红冲 REVERSE 或作废 VOID 等）；删除由写入单点自动补，不在此重复。
pub fn mark_audit_high_risk(flag: &str) {
    let _ = AUDIT_REQUEST_CTX.try_with(|cell| {
        cell.borrow_mut().risk_flag = Some(flag.to_string());
    });
}

/// 判定终端类型：小程序自定义头优先（Web 与 H5 不带这三枚头），其次 User-Agent。
/// 三档都判不出记 UNKNOWN，绝不默认 PC。
pub fn detect_terminal_type(headers: &header::HeaderMap) -> &'static str {
    // 微信小程序网关固定附带 terminalSource 与 terminalSourceV2 与 App-User-Agent（见小程序 request.js）
    let from_miniprogram = headers
        .get("terminalsource")
        .or_else(|| headers.get("terminalsourcev2"))
        .or_else(|| headers.get("app-user-agent"))
        .is_some();
    if from_miniprogram {
        return TERMINAL_MINIPROGRAM;
    }

    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ua.is_empty() || ua == "unknown" {
        return TERMINAL_UNKNOWN;
    }

    // 移动端浏览器与 Android 与 iOS 设备与 App 内 WebView（UA 含 android 或 mobile）归 H5
    if ua.contains("android")
        || ua.contains("mobile")
        || ua.contains("iphone")
        || ua.contains("ipad")
        || ua.contains("ipod")
    {
        return TERMINAL_H5;
    }

    // 桌面浏览器特征归 PC
    if ua.contains("windows nt")
        || ua.contains("macintosh")
        || ua.contains("cros")
        || ua.contains("x11")
        || ua.contains("linux x86_64")
    {
        return TERMINAL_PC;
    }

    TERMINAL_UNKNOWN
}

/// 判定操作页面：优先请求头 X-Page-Path（前端待接显式页面标识），回退 Referer。
/// Referer 里 hash 路由的片段（# 之后）才是 SPA 真实页面，无片段取路径；两者都取不到记 None。
pub fn detect_operation_page(headers: &header::HeaderMap) -> Option<String> {
    if let Some(page) = headers
        .get("x-page-path")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        return Some(page.to_string());
    }
    let referer = headers
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())?;
    // hash 路由优先
    if let Some(hash_pos) = referer.find('#') {
        let fragment = &referer[hash_pos + 1..];
        if !fragment.is_empty() {
            return Some(fragment.to_string());
        }
    }
    // 无片段则取 URL 的路径部分（scheme://host 之后、?query 之前）
    let after_scheme = referer
        .find("://")
        .map(|i| &referer[i + 3..])
        .unwrap_or(referer);
    let path = after_scheme
        .split('/')
        .nth(1)
        .map(|rest| format!("/{}", rest))
        .or_else(|| {
            if after_scheme.contains('/') {
                Some(format!("/{}", &after_scheme[after_scheme.find('/')? + 1..]))
            } else {
                None
            }
        })
        .unwrap_or_default();
    let path = path.split('?').next().unwrap_or(&path).to_string();
    if path.is_empty() || path == "/" {
        None
    } else {
        Some(path)
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
    let terminal_type = detect_terminal_type(request.headers()).to_string();
    let operation_page = detect_operation_page(request.headers());

    let audit_ctx = AuditContext {
        request_id,
        ip_address,
        user_agent,
        terminal_type,
        operation_page,
        doc_no: None,
        change_reason: None,
        risk_flag: None,
    };

    // 活上下文交给 task-local，供同步写入单点（update_with_audit 等不接收 ctx 的调用）与
    // 业务侧 set_* 登记单号与原因与风险共享同一份；extensions 里的快照供 handler 显式取用。
    request.extensions_mut().insert(audit_ctx.clone());
    AUDIT_REQUEST_CTX
        .scope(std::cell::RefCell::new(audit_ctx), next.run(request))
        .await
}
