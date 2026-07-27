use crate::middleware::auth_context::AuthContext;
use serde_json::Value;

/// P1-08-5：手机号脱敏（保留前3后4，例 13812348888→138****8888）
pub fn mask_phone(phone: &str) -> String {
    let digits: Vec<char> = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 7 {
        return "*".repeat(digits.len().max(1));
    }
    let prefix: String = digits.iter().take(3).collect();
    let suffix: String = digits.iter().skip(digits.len() - 4).take(4).collect();
    format!("{}****{}", prefix, suffix)
}

/// P1-08-5：邮箱脱敏（首字母+***，例 alice@example.com→a***@example.com）
pub fn mask_email(email: &str) -> String {
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2 {
        return "***".to_string();
    }
    let user = parts[0];
    let domain = parts[1];
    let first = user.chars().next().unwrap_or('*');
    format!("{}***@{}", first, domain)
}

/// P1-08-6：身份证号脱敏（保留前3后4，例 110101199001011234→110***********1234）
pub fn mask_id_card(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() < 7 {
        return "*".repeat(chars.len().max(1));
    }
    let prefix: String = chars.iter().take(3).collect();
    let suffix: String = chars.iter().skip(chars.len() - 4).take(4).collect();
    let middle_stars = "*".repeat(chars.len() - 7);
    format!("{}{}{}", prefix, middle_stars, suffix)
}

/// 脱敏敏感字段（如成本价、敏感金额）
pub fn mask_sensitive_fields(mut value: Value, auth: &AuthContext) -> Value {
    // 假设 role_id = 1 是超级管理员，其他角色脱敏
    // 实际项目中可以根据权限表动态判断 `has_permission(user_id, "view_cost_price")`
    if auth.role_id != Some(1) {
        // P3 维度 3 修复（批次 87）：消除 unwrap，改用 if let 显式模式匹配
        if let Some(obj) = value.as_object_mut() {
            // 移除或掩码成本价
            if obj.contains_key("cost_price") {
                obj.insert("cost_price".to_string(), Value::Null);
            }

            // 可以递归脱敏
            for (_, v) in obj.iter_mut() {
                *v = mask_sensitive_fields(v.clone(), auth);
            }
        } else if let Some(arr) = value.as_array_mut() {
            for item in arr.iter_mut() {
                *item = mask_sensitive_fields(item.clone(), auth);
            }
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_phone_normal() {
        assert_eq!(mask_phone("13812348888"), "138****8888");
    }

    #[test]
    fn test_mask_phone_short() {
        assert_eq!(mask_phone("12345"), "*****");
    }

    #[test]
    fn test_mask_email_normal() {
        assert_eq!(mask_email("alice@example.com"), "a***@example.com");
    }

    #[test]
    fn test_mask_id_card_normal() {
        assert_eq!(mask_id_card("110101199001011234"), "110***********1234");
    }

    #[test]
    fn test_mask_id_card_short() {
        assert_eq!(mask_id_card("123456"), "******");
    }
}
