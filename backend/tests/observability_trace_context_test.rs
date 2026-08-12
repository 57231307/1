    use bingxi_backend::observability::trace_context::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_new_root() {
        let ctx = TraceContext::new_root();
        assert_eq!(ctx.trace_id.len(), 32);
        assert_eq!(ctx.span_id.len(), 16);
        assert!(ctx.parent_span_id.is_none());
        assert!(ctx.sampled);
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