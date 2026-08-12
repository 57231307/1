    use super::*;
#[cfg(test)]
mod tests {

    /// 测试合法路径：通过
    #[test]
    fn test_sanitize_accepts_valid_paths() {
        assert!(sanitize_static_path("style.css").is_some());
        assert!(sanitize_static_path("css/main.css").is_some());
        assert!(sanitize_static_path("a/b/c/d.js").is_some());
    }

    /// 测试路径遍历攻击：应被拒绝
    #[test]
    fn test_sanitize_rejects_path_traversal() {
        assert!(sanitize_static_path("../../../etc/passwd").is_none());
        assert!(sanitize_static_path("..\\..\\windows\\system32").is_none());
        assert!(sanitize_static_path("a/../../etc/passwd").is_none());
    }

    /// 测试绝对路径：应被拒绝
    #[test]
    fn test_sanitize_rejects_absolute_paths() {
        assert!(sanitize_static_path("/etc/passwd").is_none());
        assert!(sanitize_static_path("/absolute/path").is_none());
    }

    /// 测试空路径与 Windows 风格反斜杠：应被拒绝
    #[test]
    fn test_sanitize_rejects_empty_and_windows_paths() {
        assert!(sanitize_static_path("").is_none());
        assert!(sanitize_static_path("..\\config").is_none());
    }
}