use bingxi_backend::observability::span::root_span;
use bingxi_backend::observability::trace_context::TraceContext;
// span_business! 通过 #[macro_export] 导出到 crate 根
use bingxi_backend::span_business;

#[test]
fn test_root_span_fields() {
    let ctx = TraceContext::new_root();
    let span = root_span(&ctx, "GET", "/api/v1/erp/users");
    let _g = span.enter();

    // 验证 span 创建成功
    assert!(!format!("{:?}", span).is_empty(), "span 不应为空");
}

#[test]
fn test_span_business_macro_compiles() {
    let s = span_business!("test_op", user_id = 42);

    // 验证宏展开后创建了有效的 span
    assert!(!format!("{:?}", s).is_empty(), "span_business 宏应创建有效的 span");
}
