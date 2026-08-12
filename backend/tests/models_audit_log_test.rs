#[cfg(test)]
mod tests {
    use super::*;

    /// 操作类型枚举序列化为稳定字符串
    #[test]
    fn test_op_type_as_str() {
        assert_eq!(OperationType::Create.as_str(), "CREATE");
        assert_eq!(OperationType::Login.as_str(), "LOGIN");
        assert_eq!(OperationType::Export.as_str(), "EXPORT");
        assert_eq!(OperationType::Print.as_str(), "PRINT");
        assert_eq!(OperationType::Download.as_str(), "DOWNLOAD");
        assert_eq!(OperationType::Other.as_str(), "OTHER");
    }

    /// 严重级别枚举序列化为稳定字符串
    #[test]
    fn test_severity_as_str() {
        assert_eq!(Severity::Info.as_str(), "INFO");
        assert_eq!(Severity::Warn.as_str(), "WARN");
        assert_eq!(Severity::Error.as_str(), "ERROR");
        assert_eq!(Severity::Critical.as_str(), "CRITICAL");
    }
}