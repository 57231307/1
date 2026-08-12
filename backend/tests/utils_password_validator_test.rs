#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strong_password() {
        let result = validate_password("MyP@ssw0rd123!");
        assert!(result.is_valid);
        assert!(result.strength.score() >= 60);
    }

    #[test]
    fn test_weak_password_too_short() {
        let result = validate_password("Ab1!");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("长度")));
    }

    #[test]
    fn test_weak_password_common() {
        let result = validate_password("password123!");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("常见")));
    }

    #[test]
    fn test_consecutive_chars() {
        assert!(has_consecutive_chars("abc123"));
        assert!(has_consecutive_chars("321cba"));
        assert!(!has_consecutive_chars("a1b2c3"));
    }

    #[test]
    fn test_repeated_chars() {
        assert!(has_repeated_chars("aaabbb"));
        assert!(has_repeated_chars("111222"));
        assert!(!has_repeated_chars("abcdef"));
    }

    // === 漏洞 #7 修复单元测试 ===

    /// #7 验证：l33t 变体 "P@ssw0rd1!" 应被拒绝（历史问题：原"contains"模糊匹配无法识别 l33t 变体）
    #[test]
    fn test_l33t_variant_rejected() {
        let result = validate_password("P@ssw0rd1!");
        assert!(
            !result.is_valid,
            "P@ssw0rd1! 应被拒绝（l33t 变体），实际 errors: {:?}",
            result.errors
        );
        assert!(
            result.errors.iter().any(|e| e.contains("常见")),
            "应包含'常见'错误，实际 errors: {:?}",
            result.errors
        );
    }

    /// #7 验证：完全相等黑名单 "password" 应被拒绝
    #[test]
    fn test_exact_blacklist_match_rejected() {
        let result = validate_password("password");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("常见")));
    }

    /// #7 验证：扩展黑名单 Top 50（"12345"、"iloveyou"）应被拒绝
    #[test]
    fn test_extended_blacklist_rejected() {
        for weak in &[
            "12345", "iloveyou", "monkey", "dragon", "sunshine", "princess",
        ] {
            let result = validate_password(weak);
            assert!(
                !result.is_valid,
                "常见密码 '{}' 应被拒绝，实际 errors: {:?}",
                weak, result.errors
            );
        }
    }

    /// #7 验证：归一化后相等 "P@ssword" 应被拒绝
    #[test]
    fn test_normalized_match_rejected() {
        let result = validate_password("P@ssword");
        assert!(!result.is_valid, "P@ssword（归一化后=password）应被拒绝");
    }

    /// #7 验证：截尾黑名单 "admin1!" 应被拒绝（历史问题：原"contains"匹配"admin123"命中，但仅去末尾数字/特殊字符的简化密码"admin"需要新逻辑）
    #[test]
    fn test_trimmed_blacklist_rejected() {
        let result = validate_password("admin1!");
        assert!(!result.is_valid, "admin1!（去掉尾部后=admin）应被拒绝");
    }

    /// #7 验证：键盘序列 "Qwerty123" 应被拒绝
    #[test]
    fn test_keyboard_sequence_rejected() {
        let result = validate_password("Qwerty123!");
        assert!(!result.is_valid, "Qwerty123!（含键盘序列）应被拒绝");
        assert!(
            result.errors.iter().any(|e| e.contains("键盘序列")),
            "应包含'键盘序列'错误，实际 errors: {:?}",
            result.errors
        );
    }

    /// #7 验证：键盘序列 "Asdf" / "Zxcv" / "Qwer" 都应被检测
    #[test]
    fn test_keyboard_sequence_various() {
        for kb in &["asdf", "Asdf1234!", "zxcvbnm", "Qwerasdf"] {
            let result = validate_password(kb);
            assert!(
                !result.is_valid,
                "键盘序列 '{}' 应被拒绝，实际 errors: {:?}",
                kb, result.errors
            );
        }
    }

    /// #7 验证：键盘反向序列 "4321" 应被检测
    #[test]
    fn test_keyboard_sequence_reverse() {
        let result = validate_password("4321!Abc");
        assert!(!result.is_valid, "反向键盘序列应被拒绝");
    }

    /// #7 验证：l33t 变体 "passw0rd" 严格匹配
    #[test]
    fn test_l33t_strict_match() {
        let result = validate_password("passw0rd");
        assert!(!result.is_valid, "passw0rd 已在黑名单，应被拒绝");
    }

    /// #7 验证：合法强密码仍通过
    #[test]
    fn test_strong_password_still_accepted() {
        let result = validate_password("Tr0ub4dor&3xYz!@#");
        assert!(
            result.is_valid,
            "强密码应通过验证，实际 errors: {:?}",
            result.errors
        );
    }

    /// #7 验证：边界 - 长度不足的密码不被键盘序列误判
    #[test]
    fn test_short_password_no_false_positive() {
        let result = validate_password("Ab1!");
        // 长度 < 4，键盘序列不会触发
        assert!(
            !result.errors.iter().any(|e| e.contains("键盘序列")),
            "短密码不应触发键盘序列错误"
        );
    }
}