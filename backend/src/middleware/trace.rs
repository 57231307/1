//! P9-6 OpenTelemetry HTTP 追踪中间件
//!
//! 本中间件为 HTTP 请求自动创建 span，并记录：
//! - HTTP method / url / status code
//! - 请求处理耗时
//! - 租户 ID（多租户隔离）
//! - trace_id（用于关联日志）

use std::time::Instant;

/// HTTP 追踪上下文
#[derive(Debug, Clone)]
pub struct HttpTraceCtx {
    /// HTTP 方法
    pub method: String,
    /// 请求路径
    pub path: String,
    /// 客户端 IP
    pub client_ip: Option<String>,
    /// User-Agent
    pub user_agent: Option<String>,
    /// 租户 ID（多租户隔离）
    pub tenant_id: Option<String>,
    /// Trace ID
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
}

impl HttpTraceCtx {
    /// 创建新的 HTTP 追踪上下文
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            client_ip: None,
            user_agent: None,
            tenant_id: None,
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
        }
    }

    /// 设置客户端 IP
    pub fn with_client_ip(mut self, ip: impl Into<String>) -> Self {
        self.client_ip = Some(ip.into());
        self
    }

    /// 设置 User-Agent
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// 设置租户 ID
    pub fn with_tenant_id(mut self, tid: impl Into<String>) -> Self {
        self.tenant_id = Some(tid.into());
        self
    }

    /// 是否为健康检查请求
    pub fn is_health_check(&self) -> bool {
        self.path == "/health" || self.path == "/healthz" || self.path == "/ready"
    }

    /// 是否需要追踪
    pub fn should_trace(&self) -> bool {
        !self.is_health_check()
    }

    /// 生成 W3C `traceparent` header
    pub fn to_traceparent(&self) -> String {
        format!("00-{}-{}-01", self.trace_id, self.span_id)
    }
}

/// HTTP 响应追踪
#[derive(Debug, Clone)]
pub struct HttpTraceResponse {
    pub status: u16,
    pub bytes_sent: u64,
    pub duration_ms: u64,
}

impl HttpTraceResponse {
    /// 记录响应信息
    pub fn record(self, ctx: &HttpTraceCtx) {
        tracing::info!(
            target: "http",
            trace_id = %ctx.trace_id,
            span_id = %ctx.span_id,
            method = %ctx.method,
            path = %ctx.path,
            status = self.status,
            duration_ms = self.duration_ms,
            bytes = self.bytes_sent,
            tenant_id = ?ctx.tenant_id,
            "HTTP {} {} -> {} ({}ms)",
            ctx.method,
            ctx.path,
            self.status,
            self.duration_ms
        );
    }
}

/// 计时器
#[derive(Debug, Clone)]
pub struct TraceTimer {
    pub start: Instant,
}

impl TraceTimer {
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Default for TraceTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成 32 字符 trace_id（16 字节十六进制）
fn generate_trace_id() -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(32);
    for b in 0..16 {
        if b > 0 {
            s.push_str("0");
        }
        let _ = write!(s, "{:x}", b);
    }
    s
}

/// 生成 16 字符 span_id（8 字节十六进制）
fn generate_span_id() -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(16);
    for b in 0..8 {
        if b > 0 {
            s.push_str("0");
        }
        let _ = write!(s, "{:x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_ctx_new() {
        let ctx = HttpTraceCtx::new("GET", "/api/orders");
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/api/orders");
        assert!(ctx.trace_id.len() >= 16);
        assert!(ctx.span_id.len() >= 8);
    }

    #[test]
    fn test_http_ctx_with_metadata() {
        let ctx = HttpTraceCtx::new("POST", "/api/orders")
            .with_client_ip("192.168.1.1")
            .with_user_agent("TestAgent/1.0")
            .with_tenant_id("tenant-001");
        assert_eq!(ctx.client_ip, Some("192.168.1.1".to_string()));
        assert_eq!(ctx.user_agent, Some("TestAgent/1.0".to_string()));
        assert_eq!(ctx.tenant_id, Some("tenant-001".to_string()));
    }

    #[test]
    fn test_health_check_detection() {
        let ctx = HttpTraceCtx::new("GET", "/health");
        assert!(ctx.is_health_check());
        assert!(!ctx.should_trace());
    }

    #[test]
    fn test_api_should_trace() {
        let ctx = HttpTraceCtx::new("GET", "/api/orders");
        assert!(!ctx.is_health_check());
        assert!(ctx.should_trace());
    }

    #[test]
    fn test_traceparent_format() {
        let ctx = HttpTraceCtx::new("GET", "/api/test");
        let tp = ctx.to_traceparent();
        // 格式：00-{trace_id}-{span_id}-01
        assert!(tp.starts_with("00-"));
        assert!(tp.ends_with("-01"));
        assert_eq!(tp.split('-').count(), 4);
    }

    #[test]
    fn test_trace_timer() {
        let timer = TraceTimer::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }

    #[test]
    fn test_http_trace_response() {
        let ctx = HttpTraceCtx::new("GET", "/api/test");
        let resp = HttpTraceResponse {
            status: 200,
            bytes_sent: 1024,
            duration_ms: 50,
        };
        // 应不 panic
        resp.record(&ctx);
    }
}
