//! 业务域 span 工具
//!
//! 提供统一的 span 创建方式，让所有 service / handler 的追踪格式一致。
//!
//! ## 使用
//!
//! 异步调用方（handler / service）把返回的 `Span` 交给 `Instrument` 绑定到 future，
//! 每次 poll 内成对进出该 span，让出时不在线程本地 span 栈上留守卫。
//!
//! ```rust,ignore
//! use tracing::Instrument;
//!
//! async fn create_purchase_order() {
//!     // ... 业务逻辑，全部日志与子 span 都归属 business.create_purchase_order ...
//! }
//!
//! create_purchase_order().instrument(span_business!("create_purchase_order", user_id = 42)).await;
//! ```
//!
//! 输出（tracing 日志）会包含：
//! ```text
//! span name=business.create_purchase_order
//! span fields: user_id=42 trace_id=...
//! ```

use tracing::Span;

/// 创建业务域 span（macro 化的实现，调用方更便捷）
///
/// 等价于：
/// ```rust,ignore
/// let span = tracing::info_span!("business.create_purchase_order", user_id = 42, ...);
/// ```
///
/// 返回的是 span 本体（不是守卫），本身不改变当前线程的 span 栈。异步调用方必须用
/// `Instrument` 绑定 future，或改用 `#[tracing::instrument]`；同步代码可在自己的栈帧内
/// 进出。持有跨 await 的同步 enter 守卫会让线程本地栈残留已关闭 span 的 id，导致后续
/// 请求在该线程上以该 id 为父级克隆 span 时 panic。
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
/// 调用方是 `middleware::trace_context`，传入本次请求的 trace 上下文、HTTP 方法与路径，
/// 返回的 `Span` 由该中间件用 `Instrument` 绑定到「下游执行加响应收尾」整段 future，
/// 作为本次请求全部日志与子 span 的父上下文。
///
/// ```rust,ignore
/// use tracing::Instrument;
///
/// let ctx = TraceContext::new_root();
/// let span = root_span(&ctx, "GET", "/api/v1/erp/users");
/// async { /* 下游请求处理 */ }.instrument(span).await;
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
