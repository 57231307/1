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

/// V15 P1 6.1：对自由文本做 PII 脱敏，将手机号/邮箱/身份证号替换为掩码，避免 AI 推理数据泄露
pub fn mask_text_pii(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    let mut result = text.to_string();
    // 手机号：1 开头 + 10 位数字（共 11 位）
    let phone_re =
        regex::Regex::new(r"1[3-9]\d{9}").unwrap_or_else(|_| regex::Regex::new(r"^$").unwrap());
    result = phone_re.replace_all(&result, "1***********").to_string();
    // 邮箱
    let email_re = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
        .unwrap_or_else(|_| regex::Regex::new(r"^$").unwrap());
    result = email_re.replace_all(&result, "***@***.***").to_string();
    // 身份证号：17 位数字 + 1 位数字/X
    let id_re =
        regex::Regex::new(r"\d{17}[\dXx]").unwrap_or_else(|_| regex::Regex::new(r"^$").unwrap());
    result = id_re.replace_all(&result, "******************").to_string();
    result
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

/// P1-08-5：银行卡号脱敏（保留前 4 后 4，例 6228480000001234567→6228****4567）
pub fn mask_bank_card(card: &str) -> String {
    let digits: Vec<char> = card.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return "*".repeat(digits.len().max(1));
    }
    let prefix: String = digits.iter().take(4).collect();
    let suffix: String = digits.iter().skip(digits.len() - 4).take(4).collect();
    format!("{}****{}", prefix, suffix)
}

/// P1-08-5：对客户/供应商/销售订单/运单响应做基于角色的手机号/邮箱脱敏（管理员不脱敏，非管理员 mask_phone/mask_email）
pub fn mask_contact_fields_for_role(mut value: Value, role_id: Option<i32>) -> Value {
    if role_id == Some(1) {
        return value;
    }
    let phone_keys = [
        "contact_phone",
        "phone",
        "mobile",
        "mobile_phone",
        "tel_phone",
        "telephone",
        "driver_phone",
        "contact_mobile",
    ];
    let email_keys = ["contact_email", "email", "email_address"];
    if let Some(obj) = value.as_object_mut() {
        for k in phone_keys.iter() {
            if let Some(s) = obj.get(*k).and_then(|v| v.as_str())  && !s.is_empty()   && let Some(s) = obj.get(*k).and_then(|v| v.as_str())   && !s.is_empty()  {
                    obj.insert(k.to_string(), Value::String(mask_phone(s)));
                }
            }
        }
        for k in email_keys.iter() {
                    obj.insert(k.to_string(), Value::String(mask_email(s)));
                }
            }
        }
    }
    value
}

/// P1-08-5：批量脱敏列表中每条记录的手机号/邮箱字段（非管理员）
pub fn mask_contact_fields_batch_for_role(
    mut value: Value,
    role_id: Option<i32>,
    list_key: &str,
) -> Value {
    if role_id == Some(1) {
        return value;
    }
    if let Some(list) = value.get_mut(list_key).and_then(|v| v.as_array_mut()) {
        for item in list.iter_mut() {
            *item = mask_contact_fields_for_role(item.clone(), role_id);
        }
    }
    value
}

/// V15 P1 batch-16 缺陷 7.4：递归脱敏 JSON 中的敏感字段（已知敏感字段按类型 mask，字符串值 mask_text_pii，数组/对象递归）
pub fn desensitize_json(value: Value) -> Value {
    match value {
        Value::Object(mut map) => {
            for (k, v) in map.iter_mut() {
                *v = desensitize_json_value(v.clone(), k);
            }
            Value::Object(map)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(desensitize_json).collect()),
        Value::String(s) => Value::String(mask_text_pii(&s)),
        other => other,
    }
}

/// 缺陷 7.4：对已知敏感字段名应用 mask 函数，其他字段递归处理
fn desensitize_json_value(value: Value, field_name: &str) -> Value {
    let lower = field_name.to_lowercase();
    let is_phone = matches!(
        lower.as_str(),
        "phone" | "mobile" | "telephone" | "tel" | "contact_phone" | "phone_no"
    );
    let is_email = lower == "email" || lower == "email_address";
    let is_id_card = matches!(
        lower.as_str(),
        "id_card" | "id_card_no" | "id_number" | "identity_card" | "identity_no" | "idcard"
    );
    let is_bank = matches!(
        lower.as_str(),
        "bank_card" | "bank_account" | "account_number" | "card_number" | "bank_card_no"
    );

    if is_phone || is_email || is_id_card || is_bank {
        match &value {
            Value::String(s) => {
                let masked = if is_phone {
                    mask_phone(s)
                } else if is_email {
                    mask_email(s)
                } else if is_id_card {
                    mask_id_card(s)
                } else {
                    mask_bank_card(s)
                };
                return Value::String(masked);
            }
            Value::Number(n) => {
                let s = n.to_string();
                let masked = if is_phone {
                    mask_phone(&s)
                } else if is_id_card {
                    mask_id_card(&s)
                } else if is_bank {
                    mask_bank_card(&s)
                } else {
                    s
                };
                return Value::String(masked);
            }
            _ => {}
        }
    }
    desensitize_json(value)
}
