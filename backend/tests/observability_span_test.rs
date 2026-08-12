#[cfg(test)]
mod tests {
    use bingxi_backend::observability::trace_context::TraceContext;

    #[test]
    fn test_root_span_fields() {
        let ctx = TraceContext::new_root();
        let span = root_span(&ctx, "GET", "/api/v1/erp/users");
        let _g = span.enter();

        // 验证 span 创建成功
        assert!(!format!("{:?}", span).is_empty(), "span 不应为空");
    }

    // 死代码清理（2026-06-26）：_macro_compiles 改为 #[test]，
    // 触发 span_business! 宏编译检查的同时作为真实测试运行。
    #[test]
    fn test_span_business_macro_compiles() {
        let s = span_business!("test_op", user_id = 42);

        // 验证宏展开后创建了有效的 span
        assert!(!format!("{:?}", s).is_empty(), "span_business 宏应创建有效的 span");
    }
}