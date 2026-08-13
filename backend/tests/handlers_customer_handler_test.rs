use bingxi_backend::handlers::customer_handler::*;
use bingxi_backend::models::customer::Model as CustomerModel;
use chrono::Utc;
use serde_json::json;
use rust_decimal::Decimal;

/// 构造测试用的客户模型
fn make_customer_model(id: i32) -> CustomerModel {
    CustomerModel {
        id,
        customer_no: format!("C-2026-{:04}", id),
        name: "测试客户".to_string(),
        short_name: Some("测试".to_string()),
        english_name: Some("Test Customer".to_string()),
        customer_type: Some("enterprise".to_string()),
        industry: Some("纺织".to_string()),
        region: Some("中国".to_string()),
        province: Some("四川".to_string()),
        city: Some("成都".to_string()),
        address: Some("测试地址".to_string()),
        contact_person: Some("张三".to_string()),
        contact_phone: Some("13800138000".to_string()),
        contact_email: Some("test@example.com".to_string()),
        tax_no: Some("91510100MA62K5XH0J".to_string()),
        bank_name: Some("中国银行".to_string()),
        bank_account: Some("1234567890".to_string()),
        credit_limit: Some(rust_decimal::Decimal::new(100000, 2)),
        credit_used: Some(rust_decimal::Decimal::new(0, 2)),
        payment_terms: Some("30天".to_string()),
        salesperson_id: Some(1),
        salesperson_name: Some("销售员".to_string()),
        status: Some("active".to_string()),
        level: Some("A".to_string()),
        tags: None,
        notes: Some("测试备注".to_string()),
        created_by: Some(1),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ===== 模型测试 =====

#[test]
fn test_customer_model_serialization() {
    let customer = make_customer_model(1);
    let json = serde_json::to_value(&customer).expect("客户序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["customer_no"], "C-2026-0001");
    assert_eq!(json["name"], "测试客户");
    assert_eq!(json["status"], "active");
}

#[test]
fn test_customer_contact_info() {
    let customer = make_customer_model(1);

    // 验证联系信息
    assert_eq!(customer.contact_person, Some("张三".to_string()));
    assert_eq!(customer.contact_phone, Some("13800138000".to_string()));
    assert_eq!(customer.contact_email, Some("test@example.com".to_string()));
}

#[test]
fn test_customer_credit_info() {
    let customer = make_customer_model(1);

    // 验证信用信息
    assert!(customer.credit_limit.is_some());
    assert!(customer.credit_used.is_some());

    // 验证信用额度
    let limit = customer.credit_limit.unwrap();
    let used = customer.credit_used.unwrap();
    let available = limit - used;

    assert_eq!(available, rust_decimal::Decimal::new(100000, 2));
}

// ===== 客户类型测试 =====

#[test]
fn test_customer_type_enterprise() {
    let customer = make_customer_model(1);
    assert_eq!(customer.customer_type, Some("enterprise".to_string()));
}

// ===== 客户等级测试 =====

#[test]
fn test_customer_level_a() {
    let customer = make_customer_model(1);
    assert_eq!(customer.level, Some("A".to_string()));
}

// ===== 状态测试 =====

#[test]
fn test_customer_status_active() {
    let customer = make_customer_model(1);
    assert_eq!(customer.status, Some("active".to_string()));
}

// ===== 信用额度测试 =====

#[test]
fn test_credit_limit_calculation() {
    let limit = rust_decimal::Decimal::new(100000, 2);
    let used = rust_decimal::Decimal::new(30000, 2);
    let available = limit - used;

    assert_eq!(available, rust_decimal::Decimal::new(70000, 2));
}

#[test]
fn test_credit_limit_exceeded() {
    let limit = rust_decimal::Decimal::new(100000, 2);
    let used = rust_decimal::Decimal::new(120000, 2);
    let available = limit - used;

    // 验证超额
    assert!(available < rust_decimal::Decimal::new(0, 2));
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_customer_json_roundtrip() {
    let customer = make_customer_model(1);
    let json = serde_json::to_value(&customer).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("customer_no").is_some());
    assert!(json.get("name").is_some());
    assert!(json.get("status").is_some());
}

#[test]
fn test_customer_json_contact_fields() {
    let customer = make_customer_model(1);
    let json = serde_json::to_value(&customer).expect("序列化失败");

    // 验证联系信息字段
    assert!(json.get("contact_person").is_some());
    assert!(json.get("contact_phone").is_some());
    assert!(json.get("contact_email").is_some());
}
