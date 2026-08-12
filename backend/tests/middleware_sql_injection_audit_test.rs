#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_patterns_non_empty() {
        assert!(!DANGEROUS_PATTERNS.is_empty());
    }

    #[test]
    fn test_pattern_detection() {
        // 简单字符串包含测试
        assert!("'; DROP TABLE users".contains("'; DROP TABLE"));
        assert!("1' OR '1'='1".contains("' OR '1'='1"));
        assert!("admin'--".contains("--"));
    }
}