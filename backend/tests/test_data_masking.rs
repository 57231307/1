//! 数据脱敏工具单元测试

use bingxi_backend::utils::data_masking::DataMasker;

#[test]
fn test_mask_password() {
    assert_eq!(DataMasker::mask_password("secret123"), "********");
    assert_eq!(DataMasker::mask_password(""), "********");
}

#[test]
fn test_mask_phone() {
    assert_eq!(DataMasker::mask_phone("13812348888"), "138****8888");
    assert_eq!(DataMasker::mask_phone("123"), "****");
    assert_eq!(DataMasker::mask_phone(""), "****");
}

#[test]
fn test_mask_email() {
    assert_eq!(DataMasker::mask_email("admin@example.com"), "a***@example.com");
    assert_eq!(DataMasker::mask_email("a@b.com"), "*@b.com*");
    assert_eq!(DataMasker::mask_email("invalid"), "***");
}

#[test]
fn test_mask_id_card() {
    assert_eq!(DataMasker::mask_id_card("310101199001011234"), "310101********1234");
    assert_eq!(DataMasker::mask_id_card("123"), "******************");
}

#[test]
fn test_mask_bank_card() {
    assert_eq!(DataMasker::mask_bank_card("622202123456788888"), "622202******8888");
    assert_eq!(DataMasker::mask_bank_card("123"), "****");
}

#[test]
fn test_mask_name() {
    assert_eq!(DataMasker::mask_name("张三"), "张**");
    assert_eq!(DataMasker::mask_name(""), "");
}

#[test]
fn test_mask_address() {
    assert_eq!(DataMasker::mask_address("上海市浦东新区张江高科技园区"), "上海***");
    assert_eq!(DataMasker::mask_address("短地址"), "短地***");
}

#[test]
fn test_mask_json_sensitive_fields() {
    let json = r#"{"username":"admin","password":"secret123","phone":"13812348888","email":"admin@example.com","id_card":"310101199001011234"}"#;
    let masked = DataMasker::mask_json_sensitive_fields(json);
    
    assert!(masked.contains("\"password\":\"********\""));
    assert!(masked.contains("\"phone\":\"138****8888\""));
    assert!(masked.contains("\"email\":\"a***@example.com\""));
    assert!(masked.contains("\"id_card\":\"310101********1234\""));
    assert!(masked.contains("\"username\":\"admin\"")); // 用户名不脱敏
}

#[test]
fn test_mask_request_body_json() {
    let body = r#"{"password":"secret","phone":"13812348888"}"#;
    let masked = DataMasker::mask_request_body(body);
    assert!(masked.contains("********"));
}

#[test]
fn test_mask_request_body_form() {
    let body = "username=admin&password=secret123&phone=13812348888";
    let masked = DataMasker::mask_request_body(body);
    assert!(masked.contains("password=********"));
}

#[test]
fn test_mask_request_body_empty() {
    let masked = DataMasker::mask_request_body("");
    assert_eq!(masked, "");
}
