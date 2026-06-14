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

use tracing::Span;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::trace_context::TraceContext;

    #[test]
    fn test_root_span_fields() {
        let ctx = TraceContext::new_root();
        let span = root_span(&ctx, "GET", "/api/v1/erp/users");
        let _g = span.enter();
    }

    // 触发宏编译
    #[allow(dead_code)]
    fn _macro_compiles() {
        let _s = span_business!("test_op", user_id = 42);
    }
}
