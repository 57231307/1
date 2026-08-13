use bingxi_backend::utils::field_mask::*;


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

#[test]
fn test_mask_text_pii_phone() {
    let input = "客户电话 13812348888 反馈色差";
    let masked = mask_text_pii(input);
    assert!(
        !masked.contains("13812348888"),
        "手机号应被脱敏，实际 {}",
        masked
    );
    assert!(masked.contains("色差"), "非 PII 文本应保留");
}

#[test]
fn test_mask_text_pii_email() {
    let input = "联系 alice@example.com 处理";
    let masked = mask_text_pii(input);
    assert!(
        !masked.contains("alice@example.com"),
        "邮箱应被脱敏，实际 {}",
        masked
    );
}

#[test]
fn test_mask_text_pii_id_card() {
    let input = "身份证 110101199001011234 已核验";
    let masked = mask_text_pii(input);
    assert!(
        !masked.contains("110101199001011234"),
        "身份证号应被脱敏，实际 {}",
        masked
    );
}

#[test]
fn test_mask_text_pii_empty() {
    assert_eq!(mask_text_pii(""), "");
}

#[test]
fn test_mask_text_pii_no_pii() {
    let input = "颜色偏差严重，需要返工";
    assert_eq!(mask_text_pii(input), input);
}

#[test]
fn test_mask_bank_card_normal() {
    assert_eq!(mask_bank_card("6228480000001234567"), "6228****4567");
}

#[test]
fn test_mask_bank_card_short() {
    assert_eq!(mask_bank_card("1234567"), "*******");
}

#[test]
fn test_desensitize_json_phone_field() {
    let input = serde_json::json!({
        "phone": "13812348888",
        "name": "张三"
    });
    let masked = desensitize_json(input);
    assert_eq!(masked["phone"], "138****8888");
    assert_eq!(masked["name"], "张三");
}

#[test]
fn test_desensitize_json_email_field() {
    let input = serde_json::json!({"email": "alice@example.com"});
    let masked = desensitize_json(input);
    assert_eq!(masked["email"], "a***@example.com");
}

#[test]
fn test_desensitize_json_id_card_field() {
    let input = serde_json::json!({"id_card": "110101199001011234"});
    let masked = desensitize_json(input);
    assert_eq!(masked["id_card"], "110***********1234");
}

#[test]
fn test_desensitize_json_bank_card_field() {
    let input = serde_json::json!({"bank_card": "6228480000001234567"});
    let masked = desensitize_json(input);
    assert_eq!(masked["bank_card"], "6228****4567");
}

#[test]
fn test_desensitize_json_nested_array_and_object() {
    let input = serde_json::json!({
        "contacts": [
            {"mobile": "13812348888"},
            {"telephone": "02187654321"}
        ],
        "meta": {"email": "alice@example.com"}
    });
    let masked = desensitize_json(input);
    assert_eq!(masked["contacts"][0]["mobile"], "138****8888");
    assert_eq!(masked["contacts"][1]["telephone"], "021****4321");
    assert_eq!(masked["meta"]["email"], "a***@example.com");
}

#[test]
fn test_desensitize_json_string_value_with_pii() {
    let input = serde_json::json!("客户电话 13812348888 反馈色差");
    let masked = desensitize_json(input);
    let s = masked.as_str().unwrap();
    assert!(!s.contains("13812348888"));
    assert!(s.contains("色差"));
}

#[test]
fn test_desensitize_json_number_and_bool_passthrough() {
    let input = serde_json::json!({"count": 42, "active": true});
    let masked = desensitize_json(input);
    assert_eq!(masked["count"], 42);
    assert_eq!(masked["active"], true);
}