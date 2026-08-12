#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_like_pattern() {
        assert_eq!(escape_like_pattern("test"), "test");
        assert_eq!(escape_like_pattern("test%"), "test\\%");
        assert_eq!(escape_like_pattern("test_"), "test\\_");
        assert_eq!(escape_like_pattern("test\\"), "test\\\\");
        assert_eq!(escape_like_pattern("%_%"), "\\%\\_\\%");
        assert_eq!(escape_like_pattern("a\0b"), "ab");
    }

    #[test]
    fn test_safe_like_pattern() {
        assert_eq!(safe_like_pattern("hello"), "%hello%");
        assert_eq!(safe_like_pattern("he%llo"), "%he\\%llo%");
        assert_eq!(safe_like_pattern("he_llo"), "%he\\_llo%");
    }
}