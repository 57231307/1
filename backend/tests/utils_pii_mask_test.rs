#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_phone() {
        assert_eq!(mask_pii("手机号: 13812345678"), "手机号: 138****5678");
        assert_eq!(mask_pii("电话13900001111"), "电话139****1111");
        // 不匹配非手机号
        assert_eq!(mask_pii("数字: 12345678901"), "数字: 12345678901");
        // 不匹配短号
        assert_eq!(mask_pii("短号: 12345"), "短号: 12345");
    }

    #[test]
    fn test_mask_id_card() {
        assert_eq!(
            mask_pii("身份证: 110101199001011234"),
            "身份证: 1101**********1234"
        );
        assert_eq!(
            mask_pii("ID: 11010119900101123X"),
            "ID: 1101**********123X"
        );
    }

    #[test]
    fn test_mask_email() {
        assert_eq!(
            mask_pii("邮箱: user@example.com"),
            "邮箱: u***@example.com"
        );
        assert_eq!(
            mask_pii("email: test.user@domain.org"),
            "email: t***@domain.org"
        );
    }

    #[test]
    fn test_mask_password_field() {
        assert_eq!(
            mask_pii(r#"{"password":"secret123"}"#),
            r#"{"password":"[REDACTED]"}"#
        );
        assert_eq!(
            mask_pii(r#"{"access_token":"eyJhbGciOiJIUzI1NiJ9"}"#),
            r#"{"access_token":"[REDACTED]"}"#
        );
    }

    #[test]
    fn test_mask_mixed_pii() {
        let input = "用户 13812345678 身份证 110101199001011234 邮箱 user@test.com";
        let masked = mask_pii(input);
        assert!(masked.contains("138****5678"));
        assert!(masked.contains("1101**********1234"));
        assert!(masked.contains("u***@test.com"));
    }

    #[test]
    fn test_no_pii() {
        let input = "普通日志消息，没有敏感信息";
        assert_eq!(mask_pii(input), input);
    }
}