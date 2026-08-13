use bingxi_backend::models::supplier::Model as SupplierModel;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的供应商模型
fn make_supplier_model(id: i32) -> SupplierModel {
    SupplierModel {
        id,
        supplier_code: format!("S-2026-{:04}", id),
        supplier_name: "测试供应商".to_string(),
        supplier_short_name: "测试".to_string(),
        supplier_type: "manufacturer".to_string(),
        credit_code: "91330100MA27K5XH0J".to_string(),
        registered_address: "测试地址".to_string(),
        business_address: Some("经营地址".to_string()),
        legal_representative: "李四".to_string(),
        registered_capital: Decimal::from(1000),
        establishment_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        business_term: Some("长期".to_string()),
        business_scope: Some("纺织品制造".to_string()),
        taxpayer_type: "一般纳税人".to_string(),
        bank_name: "工商银行".to_string(),
        bank_account: "0987654321".to_string(),
        contact_phone: "13900139000".to_string(),
        fax: None,
        website: None,
        email: Some("test@example.com".to_string()),
        main_business: Some("面料生产".to_string()),
        main_market: Some("国内".to_string()),
        employee_count: Some(100),
        annual_turnover: Some(Decimal::from(5000)),
        status: "active".to_string(),
        created_by: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ===== 模型测试 =====

#[test]
fn test_supplier_model_serialization() {
    let supplier = make_supplier_model(1);
    let json = serde_json::to_value(&supplier).expect("供应商序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["supplier_code"], "S-2026-0001");
    assert_eq!(json["supplier_name"], "测试供应商");
    assert_eq!(json["status"], "active");
}

#[test]
fn test_supplier_code_format() {
    let supplier = make_supplier_model(42);
    assert_eq!(supplier.supplier_code, "S-2026-0042");
}

#[test]
fn test_supplier_type() {
    let supplier = make_supplier_model(1);
    assert_eq!(supplier.supplier_type, "manufacturer");
}

#[test]
fn test_supplier_credit_code() {
    let supplier = make_supplier_model(1);
    assert_eq!(supplier.credit_code, "91330100MA27K5XH0J");
}

#[test]
fn test_supplier_contact() {
    let supplier = make_supplier_model(1);
    assert_eq!(supplier.contact_phone, "13900139000");
    assert_eq!(supplier.email, Some("test@example.com".to_string()));
}

#[test]
fn test_supplier_bank_info() {
    let supplier = make_supplier_model(1);
    assert_eq!(supplier.bank_name, "工商银行");
    assert_eq!(supplier.bank_account, "0987654321");
}
