//! W3C Trace Context 实现
//!
//! 标准：[W3C Trace Context Level 2](https://www.w3.org/TR/trace-context/)
//!
//! ## 协议格式
//!
//! - `traceparent`: `00-{trace_id(32 hex)}-{parent_id(16 hex)}-{flags(2 hex)}`
//!   - `00`：版本号，目前固定 `00`
//!   - `trace_id`：32 字符 hex（128 bit），全 0 表示无效
//!   - `parent_id` / `span_id`：16 字符 hex（64 bit），全 0 表示无效
//!   - `flags`：1 字节
//!     - `01`：sampled（本 trace 已被采样）
//!     - `00`：未采样
//! - `tracestate`：可选，厂商扩展（key=value 列表）
//!
//! ## 简化约定
//!
//! - `trace_id`：32 hex 字符（128 bit），用 `Uuid::new_v4()` 生成后去 `-` 再补 0
//! - `span_id`：16 hex 字符（64 bit），用 `rand::random::<u64>()` 生成
//! - `sampled`：默认 `true`，可通过环境变量 `OTEL_TRACES_SAMPLER=always_off` 关闭

use std::fmt;
use uuid::Uuid;

/// Trace 上下文：单次请求的追踪 ID + 父 span ID + 当前 span ID + 采样标志
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    /// 128-bit trace ID（hex 32 字符）
    pub trace_id: String,
    /// 64-bit 当前 span ID（hex 16 字符）
    pub span_id: String,
    /// 64-bit 父 span ID（hex 16 字符；如本 span 即 root 则为 None）
    pub parent_span_id: Option<String>,
    /// 是否被采样
    pub sampled: bool,
}

impl TraceContext {
    /// 生成全新的 root span trace 上下文
    pub fn new_root() -> Self {
        let trace_id = generate_trace_id();
        let span_id = generate_span_id();
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            sampled: true,
        }
    }

    /// 基于当前 span 生成子 span（保留 trace_id，parent_span_id 指向当前 span_id）
    pub fn new_child(&self) -> Self {
        let child_span_id = generate_span_id();
        Self {
            trace_id: self.trace_id.clone(),
            span_id: child_span_id,
            parent_span_id: Some(self.span_id.clone()),
            sampled: self.sampled,
        }
    }

    /// 序列化到 W3C `traceparent` header
    pub fn to_traceparent(&self) -> String {
        // 格式：00-{trace_id}-{span_id}-{flags}
        // flags: 01 表示 sampled，00 表示未采样
        let flags = if self.sampled { "01" } else { "00" };
        format!("00-{}-{}-{}", self.trace_id, self.span_id, flags)
    }

    /// 从 W3C `traceparent` header 解析
    ///
    /// 解析失败或字段不合法时返回 `None`，调用方应 fallback 到生成新 trace。
    pub fn from_traceparent(header: &str) -> Option<Self> {
        // 去掉前后空白
        let h = header.trim();
        // 拆分 4 段：version-trace_id-parent_id-flags
        let parts: Vec<&str> = h.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let version = parts[0];
        let trace_id = parts[1];
        let parent_id = parts[2];
        let flags = parts[3];

        // 版本必须是 2 字符 hex
        if version.len() != 2 || !is_hex(version) {
            return None;
        }
        // trace_id 必须是 32 字符 hex，且非全 0
        if trace_id.len() != 32 || !is_hex(trace_id) || trace_id.chars().all(|c| c == '0') {
            return None;
        }
        // parent_id 必须是 16 字符 hex，且非全 0
        if parent_id.len() != 16 || !is_hex(parent_id) || parent_id.chars().all(|c| c == '0') {
            return None;
        }
        // flags 必须是 2 字符 hex
        if flags.len() != 2 || !is_hex(flags) {
            return None;
        }

        // flags 第 0 位为 1 表示 sampled
        let sampled_byte = u8::from_str_radix(flags, 16).ok()?;
        let sampled = (sampled_byte & 0x01) == 0x01;

        Some(Self {
            trace_id: trace_id.to_string(),
            span_id: parent_id.to_string(),
            parent_span_id: None,
            sampled,
        })
    }
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "trace_id={}, span_id={}", self.trace_id, self.span_id)?;
        if let Some(parent) = &self.parent_span_id {
            write!(f, ", parent_span_id={}", parent)?;
        }
        if self.sampled {
            write!(f, ", sampled=true")?;
        }
        Ok(())
    }
}

/// 生成 32 字符 hex 形式的 trace_id
///
/// 用 UUIDv4 去 `-` 后再补 0 凑足 32 字符（UUIDv4 去掉 `-` 是 32 字符，正好）。
pub fn generate_trace_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// 生成 16 字符 hex 形式的 span_id
pub fn generate_span_id() -> String {
    // 使用 fastrand（已在 Cargo.toml 中）
    let n: u64 = fastrand::u64(..);
    format!("{:016x}", n)
}

/// 简单 hex 校验
fn is_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

/// 给 reqwest HTTP 客户端构造 traceparent header（用于出站调用）
pub fn build_traceparent_for_outbound(ctx: &TraceContext) -> String {
    ctx.to_traceparent()
}

/// 解析入站 `traceparent` header，失败则返回新 root
///
/// 行为策略（fail-open）：
/// - 客户端正确传递 → 复用 trace_id，关联跨服务追踪
/// - 客户端没传 / 格式错 → 生成新 trace_id，单独追踪本次请求
pub fn extract_or_new(header_value: Option<&str>) -> TraceContext {
    match header_value.and_then(TraceContext::from_traceparent) {
        Some(mut ctx) => {
            // 父 span_id 是当前请求的 span；为本次新 span 重新生成 span_id，
            // 并把上级的 parent_id 记为 parent_span_id
            ctx.parent_span_id = Some(ctx.span_id.clone());
            ctx.span_id = generate_span_id();
            ctx
        }
        None => TraceContext::new_root(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_root() {
        let ctx = TraceContext::new_root();
        assert_eq!(ctx.trace_id.len(), 32);
        assert_eq!(ctx.span_id.len(), 16);
        assert!(ctx.parent_span_id.is_none());
        assert!(ctx.sampled);
    }

    #[test]
    fn test_new_child_inherits_trace_id() {
        let root = TraceContext::new_root();
        let child = root.new_child();
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.parent_span_id.as_deref(), Some(root.span_id.as_str()));
        assert_ne!(child.span_id, root.span_id);
    }

    #[test]
    fn test_traceparent_round_trip() {
        let ctx = TraceContext::new_root();
        let header = ctx.to_traceparent();
        let parsed = TraceContext::from_traceparent(&header).expect("should parse");
        // 注意：round_trip 后 span_id 变成 parent_id 字段，trace_id 保持一致
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert!(parsed.sampled);
    }

    #[test]
    fn test_traceparent_format() {
        let ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            parent_span_id: None,
            sampled: true,
        };
        assert_eq!(
            ctx.to_traceparent(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        );
    }

    #[test]
    fn test_traceparent_unsampled() {
        let ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            parent_span_id: None,
            sampled: false,
        };
        let header = ctx.to_traceparent();
        assert!(header.ends_with("-00"));
    }

    #[test]
    fn test_traceparent_invalid_inputs() {
        // 段数错
        assert!(TraceContext::from_traceparent("00-aaaa-bbbb").is_none());
        // 版本非 hex
        assert!(TraceContext::from_traceparent("ZZ-aaaa-aaaa-aa").is_none());
        // trace_id 全 0
        assert!(TraceContext::from_traceparent(
            "00-00000000000000000000000000000000-aaaaaaaaaaaaaaaaaaaaaaaa-01"
        )
        .is_none());
        // parent_id 全 0
        assert!(TraceContext::from_traceparent(
            "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-0000000000000000-01"
        )
        .is_none());
        // 空字符串
        assert!(TraceContext::from_traceparent("").is_none());
    }

    #[test]
    fn test_extract_or_new_with_valid_header() {
        let header = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ctx = extract_or_new(Some(header));
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.parent_span_id.as_deref(), Some("b7ad6b7169203331"));
        assert_ne!(ctx.span_id, "b7ad6b7169203331");
    }

    #[test]
    fn test_extract_or_new_with_missing_header() {
        let ctx = extract_or_new(None);
        assert_eq!(ctx.trace_id.len(), 32);
        assert!(ctx.parent_span_id.is_none());
    }

    #[test]
    fn test_extract_or_new_with_invalid_header_falls_back() {
        let ctx = extract_or_new(Some("not a valid header"));
        assert_eq!(ctx.trace_id.len(), 32);
        assert!(ctx.parent_span_id.is_none());
    }

    #[test]
    fn test_display() {
        let ctx = TraceContext::new_root();
        let s = format!("{}", ctx);
        assert!(s.contains("trace_id="));
        assert!(s.contains("span_id="));
    }
}
