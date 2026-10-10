use bingxi_backend::handlers::customer_handler::*;
use bingxi_backend::models::customer::Model as CustomerModel;
use bingxi_backend::utils::error::AppError;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use validator::Validate;

/// 构造测试用的客户模型
fn make_customer_model(id: i32) -> CustomerModel {
    let now: DateTime<Utc> = Utc::now();
    CustomerModel {
        id,
        customer_code: format!("C-2026-{:04}", id),
        customer_name: format!("测试客户-{}", id),
        customer_type: "retail".to_string(),
        province: Some("四川".to_string()),
        city: Some("成都".to_string()),
        address: Some("测试地址".to_string()),
        contact_person: Some("张三".to_string()),
        contact_phone: Some("13800138000".to_string()),
        contact_email: Some("test@example.com".to_string()),
        bank_name: Some("中国银行".to_string()),
        bank_account: Some("1234567890".to_string()),
        credit_limit: Decimal::new(100000, 2),
        payment_terms: 30,
        status: "active".to_string(),
        notes: Some("测试备注".to_string()),
        created_by: Some(1),
        created_at: now,
        updated_at: now,
        owner_id: 1,
        ..Default::default()
    }
}

// ===== 模型测试 =====

#[test]
fn test_customer_model_serialization() {
    let customer = make_customer_model(1);
    let json = serde_json::to_value(&customer).expect("客户序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["customer_code"], "C-2026-0001");
    assert_eq!(json["customer_type"], "retail");
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

    // 验证信用额度
    let limit = customer.credit_limit;
    let expected_limit = rust_decimal::Decimal::new(100000, 2);

    assert_eq!(limit, expected_limit);
}

// ===== 客户类型测试 =====

// 夹具值必须是校验器真实白名单成员（波0 前此处用 'enterprise' 断言"合法客户类型"，
// 而 enterprise 不在白名单——纯 serde 构造绕过校验器的编造合法值，假绿形状）。
#[test]
fn test_customer_type_fixture_uses_allowed_token() {
    let customer = make_customer_model(1);
    assert!(
        bingxi_backend::constants::customer_type::ALLOWED
            .contains(&customer.customer_type.as_str()),
        "夹具 customer_type 必须是唯一词表 ALLOWED 成员，当前值：{}",
        customer.customer_type
    );
}

/// 负例：校验器会拒 `enterprise`（非白名单值经 validator 通道 400 + VALIDATION_ERROR，
/// 真实拒绝形态；不许把校验器扩成接受 enterprise）
#[test]
fn test_enterprise_customer_type_rejected_by_validator() {
    let req: CreateCustomerRequest = serde_json::from_value(json!({
        "customer_name": "负例客户",
        "customer_type": "enterprise",
    }))
    .expect("反序列化失败");
    let errors = req
        .validate()
        .expect_err("enterprise 不在白名单，validator 必须拒绝");
    assert!(
        errors.errors().contains_key("customer_type"),
        "拒绝必须落在 customer_type 字段上"
    );
    let err = AppError::from(errors);
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
}

// ===== 状态测试 =====

#[test]
fn test_customer_status_active() {
    let customer = make_customer_model(1);
    assert_eq!(customer.status, "active".to_string());
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
    assert!(json.get("customer_code").is_some());
    assert!(json.get("customer_type").is_some());
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
