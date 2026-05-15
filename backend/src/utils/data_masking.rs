//! 敏感数据脱敏工具
//!
//! 提供各类敏感数据的脱敏处理功能，防止日志和接口中泄露敏感信息

#![allow(dead_code)]

use regex::Regex;
use once_cell::sync::Lazy;

// 预编译正则表达式，避免运行时重复编译和 unwrap 风险
static PASSWORD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""password"\s*:\s*"[^"]*""#).expect("密码脱敏正则表达式编译失败")
});
static PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""phone"\s*:\s*"([^"]*)""#).expect("手机号脱敏正则表达式编译失败")
});
static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""email"\s*:\s*"([^"]*)""#).expect("邮箱脱敏正则表达式编译失败")
});
static ID_CARD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""id_card"\s*:\s*"([^"]*)""#).expect("身份证脱敏正则表达式编译失败")
});
static BANK_CARD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""bank_card"\s*:\s*"([^"]*)""#).expect("银行卡脱敏正则表达式编译失败")
});
static PASSWORD_PARAM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(password|passwd|pwd)=([^&\s]+)").expect("密码参数脱敏正则表达式编译失败")
});

/// 脱敏规则类型
#[derive(Debug, Clone)]
pub enum MaskingRule {
    /// 密码：完全隐藏
    Password,
    /// 手机号：保留前3位和后4位
    Phone,
    /// 邮箱：保留用户名首字符和域名
    Email,
    /// 身份证号：保留前6位和后4位
    IdCard,
    /// 银行卡号：保留前6位和后4位
    BankCard,
    /// 姓名：保留姓
    Name,
    /// 地址：保留省市区，隐藏详细地址
    Address,
    /// 自定义正则替换
    Custom(Regex, String),
}

/// 数据脱敏器
pub struct DataMasker;

impl DataMasker {
    /// 对密码进行脱敏
    pub fn mask_password(_password: &str) -> String {
        "********".to_string()
    }

    /// 对手机号进行脱敏
    /// 示例: 138****8888
    pub fn mask_phone(phone: &str) -> String {
        if phone.len() < 7 {
            return "****".to_string();
        }
        let prefix = &phone[..3];
        let suffix = &phone[phone.len() - 4..];
        format!("{}****{}", prefix, suffix)
    }

    /// 对邮箱进行脱敏
    /// 示例: a***@example.com
    pub fn mask_email(email: &str) -> String {
        if let Some(at_pos) = email.find('@') {
            let local = &email[..at_pos];
            let domain = &email[at_pos..];
            if local.len() <= 1 {
                return format!("*{}*", domain);
            }
            let first = &local[..1];
            format!("{}***{}", first, domain)
        } else {
            "***".to_string()
        }
    }

    /// 对身份证号进行脱敏
    /// 示例: 310101********1234
    pub fn mask_id_card(id_card: &str) -> String {
        if id_card.len() < 10 {
            return "******************".to_string();
        }
        let prefix = &id_card[..6];
        let suffix = &id_card[id_card.len() - 4..];
        let stars = "*".repeat(id_card.len() - 10);
        format!("{}{}{}", prefix, stars, suffix)
    }

    /// 对银行卡号进行脱敏
    /// 示例: 622202******8888
    pub fn mask_bank_card(card: &str) -> String {
        let digits: String = card.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() < 10 {
            return "****".to_string();
        }
        let prefix = &digits[..6];
        let suffix = &digits[digits.len() - 4..];
        format!("{}******{}", prefix, suffix)
    }

    /// 对姓名进行脱敏
    /// 示例: 张**
    pub fn mask_name(name: &str) -> String {
        if name.is_empty() {
            return String::new();
        }
        let first = name.chars().next().unwrap_or('*');
        format!("{}**", first)
    }

    /// 对地址进行脱敏
    /// 示例: 上海市浦东新区***
    pub fn mask_address(address: &str) -> String {
        // 简单处理：保留前6个字符，后面隐藏
        if address.len() <= 6 {
            return address.to_string();
        }
        let prefix = &address[..6];
        format!("{}***", prefix)
    }

    /// 对JSON字符串中的敏感字段进行脱敏
    pub fn mask_json_sensitive_fields(json_str: &str) -> String {
        let mut result = json_str.to_string();

        // 密码字段脱敏
        result = PASSWORD_RE.replace_all(&result, r#""password":"********""#).to_string();

        // 手机号字段脱敏
        result = PHONE_RE.replace_all(&result, |caps: &regex::Captures| {
            let masked = Self::mask_phone(&caps[1]);
            format!(r#""phone":"{}""#, masked)
        }).to_string();

        // 邮箱字段脱敏
        result = EMAIL_RE.replace_all(&result, |caps: &regex::Captures| {
            let masked = Self::mask_email(&caps[1]);
            format!(r#""email":"{}""#, masked)
        }).to_string();

        // 身份证号字段脱敏
        result = ID_CARD_RE.replace_all(&result, |caps: &regex::Captures| {
            let masked = Self::mask_id_card(&caps[1]);
            format!(r#""id_card":"{}""#, masked)
        }).to_string();

        // 银行卡号字段脱敏
        result = BANK_CARD_RE.replace_all(&result, |caps: &regex::Captures| {
            let masked = Self::mask_bank_card(&caps[1]);
            format!(r#""bank_card":"{}""#, masked)
        }).to_string();

        result
    }

    /// 对请求体进行脱敏（用于日志记录）
    pub fn mask_request_body(body: &str) -> String {
        if body.is_empty() {
            return String::new();
        }

        // 尝试作为JSON处理
        if body.starts_with('{') || body.starts_with('[') {
            return Self::mask_json_sensitive_fields(body);
        }

        // 非JSON内容，进行简单处理
        let mut result = body.to_string();

        // 隐藏可能的密码参数
        result = PASSWORD_PARAM_RE.replace_all(&result, "$1=********").to_string();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        assert_eq!(DataMasker::mask_password("secret123"), "********");
    }

    #[test]
    fn test_mask_phone() {
        assert_eq!(DataMasker::mask_phone("13812348888"), "138****8888");
        assert_eq!(DataMasker::mask_phone("123"), "****");
    }

    #[test]
    fn test_mask_email() {
        assert_eq!(DataMasker::mask_email("admin@example.com"), "a***@example.com");
        assert_eq!(DataMasker::mask_email("a@b.com"), "*@b.com*");
    }

    #[test]
    fn test_mask_id_card() {
        assert_eq!(DataMasker::mask_id_card("310101199001011234"), "310101********1234");
    }

    #[test]
    fn test_mask_bank_card() {
        assert_eq!(DataMasker::mask_bank_card("622202123456788888"), "622202******8888");
    }

    #[test]
    fn test_mask_name() {
        assert_eq!(DataMasker::mask_name("张三"), "张**");
    }

    #[test]
    fn test_mask_json() {
        let json = r#"{"username":"admin","password":"secret","phone":"13812348888","email":"admin@example.com"}"#;
        let masked = DataMasker::mask_json_sensitive_fields(json);
        assert!(masked.contains("********"));
        assert!(masked.contains("138****8888"));
        assert!(masked.contains("a***@example.com"));
    }
}
