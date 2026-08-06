//! V15 P2 20.8-C：PII（个人身份信息）脱敏工具
//!
//! 提供统一的手机号/身份证号/邮箱/密码脱敏函数，用于日志输出前自动处理。
//!
//! ## 脱敏规则
//!
//! - 手机号（11 位，1[3-9] 开头）：保留前 3 后 4，中间 4 位替换为 `****`
//!   - 例：`13812345678` → `138****5678`
//! - 身份证号（18 位，末位可为 X/x）：保留前 4 后 4，中间 10 位替换为 `**********`
//!   - 例：`110101199001011234` → `1101**********1234`
//! - 邮箱：保留首字符和 `@` 后域名，`@` 前部分替换为 `***`
//!   - 例：`user@example.com` → `u***@example.com`
//! - 密码字段（key 包含 password/passwd/secret/token/credential）：值替换为 `[REDACTED]`
//!
//! ## 使用方式
//!
//! ```rust
//! use crate::utils::pii_mask::mask_pii;
//!
//! let msg = "用户手机号: 13812345678, 身份证: 110101199001011234";
//! let masked = mask_pii(msg);
//! // "用户手机号: 138****5678, 身份证: 1101**********1234"
//! ```

use regex::Regex;
use std::sync::LazyLock;

/// 手机号正则（11 位，1[3-9] 开头）
/// 注意：Rust regex crate 不支持 look-around，此处使用简单匹配
static PHONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"1[3-9][0-9]{9}").expect("PII_MASK: 手机号正则编译失败")
});

/// 身份证号正则（18 位，末位可为 X/x）
static ID_CARD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[1-9][0-9]{5}(?:19|20)[0-9]{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12][0-9]|3[01])[0-9]{3}[0-9Xx]")
        .expect("PII_MASK: 身份证正则编译失败")
});

/// 邮箱正则
static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})")
        .expect("PII_MASK: 邮箱正则编译失败")
});

/// 密码字段 key 模式（JSON key 或日志字段名）
static PASSWORD_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""((?:password|passwd|secret|token|credential|api_key|apikey|access_token|refresh_token)(?:_[a-z]+)?)"\s*:\s*"([^"]*)""#)
        .expect("PII_MASK: 密码字段正则编译失败")
});

/// 对文本中的 PII 数据进行脱敏
///
/// 自动识别并脱敏以下类型：
/// - 手机号（11 位，1[3-9] 开头）
/// - 身份证号（18 位，末位可为 X/x）
/// - 邮箱地址
/// - JSON 中的密码/密钥字段值
pub fn mask_pii(text: &str) -> String {
    let mut result = text.to_string();

    // 1. 脱敏手机号：138****5678
    result = PHONE_REGEX
        .replace_all(&result, |caps: &regex::Captures| {
            let phone = &caps[0];
            format!("{}****{}", &phone[..3], &phone[7..])
        })
        .to_string();

    // 2. 脱敏身份证号：1101**********1234
    result = ID_CARD_REGEX
        .replace_all(&result, |caps: &regex::Captures| {
            let id = &caps[0];
            format!("{}**********{}", &id[..4], &id[14..])
        })
        .to_string();

    // 3. 脱敏邮箱：u***@example.com
    result = EMAIL_REGEX
        .replace_all(&result, |caps: &regex::Captures| {
            let local = &caps[1];
            let domain = &caps[2];
            let first_char = local.chars().next().unwrap_or('*');
            format!("{}***@{}", first_char, domain)
        })
        .to_string();

    // 4. 脱敏 JSON 中的密码字段
    result = PASSWORD_KEY_REGEX
        .replace_all(&result, |caps: &regex::Captures| {
            let key = &caps[1];
            format!(r#""{}":"[REDACTED]""#, key)
        })
        .to_string();

    result
}

/// 判断字符串是否包含手机号
pub fn contains_phone(text: &str) -> bool {
    PHONE_REGEX.is_match(text)
}

/// 判断字符串是否包含身份证号
pub fn contains_id_card(text: &str) -> bool {
    ID_CARD_REGEX.is_match(text)
}

/// 判断字符串是否包含邮箱
pub fn contains_email(text: &str) -> bool {
    EMAIL_REGEX.is_match(text)
}

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
    fn test_contains_helpers() {
        assert!(contains_phone("电话 13812345678"));
        assert!(!contains_phone("数字 12345"));
        assert!(contains_id_card("身份证 110101199001011234"));
        assert!(contains_email("邮箱 user@test.com"));
    }

    #[test]
    fn test_no_pii() {
        let input = "普通日志消息，没有敏感信息";
        assert_eq!(mask_pii(input), input);
    }
}
