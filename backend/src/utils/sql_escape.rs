/// SQL LIKE 特殊字符转义工具（PostgreSQL LIKE 用 \ 转义，需转义 % _ \ ）
pub fn escape_like_pattern(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                result.push('\\');
                result.push(ch);
            }
            // 防止空字节注入
            '\0' => {}
            other => result.push(other),
        }
    }
    result
}

/// 构建安全的 LIKE 查询模式，自动转义用户输入
/// 例如: safe_like_pattern("test%") => "%test\\%%"
pub fn safe_like_pattern(keyword: &str) -> String {
    let escaped = escape_like_pattern(keyword);
    format!("%{}%", escaped)
}

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
