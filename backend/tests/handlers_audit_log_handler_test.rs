    use bingxi_backend::handlers::audit_log_handler::*;
#[cfg(test)]
mod tests {

    /// AuditLogListQuery 默认值：所有可选字段为 None
    #[test]
    fn test_list_query_default_values() {
        let q = AuditLogListQuery::default();
        assert!(q.start_time.is_none());
        assert!(q.end_time.is_none());
        assert!(q.user_id.is_none());
        assert!(q.operation_type.is_none());
        assert!(q.severity.is_none());
        assert!(q.resource_type.is_none());
        assert!(q.request_id.is_none());
        assert!(q.keyword.is_none());
        assert!(q.page.is_none());
        assert!(q.page_size.is_none());
    }

    /// V15 缺陷 10-4：hex_sha256 对空输入返回已知常量值
    #[test]
    fn test_hex_sha256_empty() {
        let hash = hex_sha256(b"");
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// V15 缺陷 10-4：hex_sha256 对相同输入产生相同指纹（确定性）
    #[test]
    fn test_hex_sha256_deterministic() {
        let a = hex_sha256(b"audit-log-export-test");
        let b = hex_sha256(b"audit-log-export-test");
        assert_eq!(a, b);
        assert_ne!(a, hex_sha256(b"audit-log-export-test-2"));
    }

    /// V15 缺陷 10-4：header_str 从 HeaderMap 提取首个值
    #[test]
    fn test_header_str_extract() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "abc-123".parse().unwrap());
        assert_eq!(
            header_str(&headers, "x-request-id"),
            Some("abc-123".to_string())
        );
        assert_eq!(header_str(&headers, "user-agent"), None);
    }

    /// V15 缺陷 10-4：ExportLogListQuery 默认分页参数为 None
    #[test]
    fn test_export_log_list_query_default() {
        let q = ExportLogListQuery {
            page: None,
            per_page: None,
            exporter_user_id: None,
        };
        assert!(q.page.is_none());
        assert!(q.per_page.is_none());
        assert!(q.exporter_user_id.is_none());
    }
}