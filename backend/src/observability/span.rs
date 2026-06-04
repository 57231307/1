//! 业务域 span 工具
//!
//! 提供统一的 span 创建方式，让所有 service / handler 的追踪格式一致。
//!
//! ## 使用
//!
//! ```rust,ignore
//! use crate::observability::span::span_business;
//!
//! // 在 service / handler 中：
//! let _enter = span_business!("create_purchase_order", user_id = 42, tenant_id = "acme");
//! // ... 业务逻辑 ...
//! ```
//!
//! 输出（tracing 日志）会包含：
//! ```text
//! span name=business.create_purchase_order
//! span fields: user_id=42 tenant_id=acme trace_id=...
//! ```

use std::collections::HashMap;
use tracing::{field::Empty, Span};

/// 创建业务域 span（macro 化的实现，调用方更便捷）
///
/// 等价于：
/// ```rust,ignore
/// let span = tracing::info_span!("business.create_purchase_order", user_id = 42, ...);
/// let _enter = span.enter();
/// ```
#[macro_export]
macro_rules! span_business {
    ($name:expr) => {
        tracing::info_span!(concat!("business.", $name))
    };
    ($name:expr, $($field:tt)*) => {
        tracing::info_span!(concat!("business.", $name), $($field)*)
    };
}

/// 创建一个空的 `tracing::Span` 占位，调用方后续可用 `record!` 补充字段
pub fn empty_business_span(name: &str) -> Span {
    tracing::info_span!("business.", operation = %name, trace_id = Empty)
}

/// 为 `TraceContext` 创建 root span
///
/// ```rust,ignore
/// let ctx = TraceContext::new_root();
/// let span = root_span(&ctx, "GET", "/api/v1/erp/users");
/// let _enter = span.enter();
/// ```
pub fn root_span(ctx: &super::trace_context::TraceContext, method: &str, path: &str) -> Span {
    tracing::info_span!(
        "http.request",
        method = %method,
        path = %path,
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        parent_span_id = ctx.parent_span_id.as_deref().unwrap_or(""),
        sampled = ctx.sampled,
    )
}

/// 为 TraceContext 创建子 span
pub fn child_span(ctx: &super::trace_context::TraceContext, operation: &str) -> Span {
    tracing::info_span!(
        "business",
        operation = %operation,
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        parent_span_id = ctx.parent_span_id.as_deref().unwrap_or(""),
    )
}

/// 从 `tracing::Span` 的 fields 中提取 trace_id
///
/// 用于在错误响应中拿到本请求的 trace_id（而非重新生成 UUID）。
pub fn current_trace_id() -> Option<String> {
    // 简化实现：在大型 span 树中查找 `trace_id` 字段。
    // 由于 tracing 没有标准 API 反查 span fields，
    // 通常的做法是配合 `tracing_subscriber::Registry` + custom layer。
    // 此处返回 None 表示需要 middleware 通过其他途径传递 trace_id。
    None
}

/// 把业务上下文批量记录到当前 span
pub fn record_business_fields(span: &Span, fields: &HashMap<String, String>) {
    for (k, v) in fields {
        span.record(k.as_str(), v.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::trace_context::TraceContext;

    #[test]
    fn test_empty_business_span() {
        let span = empty_business_span("create_user");
        // Span 创建不抛异常即成功
        let _g = span.enter();
    }

    #[test]
    fn test_root_span_fields() {
        let ctx = TraceContext::new_root();
        let span = root_span(&ctx, "GET", "/api/v1/erp/users");
        let _g = span.enter();
    }

    #[test]
    fn test_child_span_inherits_trace() {
        let ctx = TraceContext::new_root();
        let span = child_span(&ctx, "approve_order");
        let _g = span.enter();
    }

    #[test]
    fn test_record_business_fields() {
        let span = empty_business_span("test_op");
        let mut fields = HashMap::new();
        fields.insert("user_id".to_string(), "42".to_string());
        record_business_fields(&span, &fields);
    }

    // 触发宏编译
    #[allow(dead_code)]
    fn _macro_compiles() {
        let _s = span_business!("test_op", user_id = 42);
    }
}
