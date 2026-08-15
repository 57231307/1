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
        legal_representative: "李四".to_string(),
        registered_capital: Decimal::from(1000),
        establishment_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        taxpayer_type: "一般纳税人".to_string(),
        bank_name: "工商银行".to_string(),
        bank_account: "0987654321".to_string(),
        contact_phone: "13900139000".to_string(),
        ..Default::default()
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
    assert!(json["status"].is_null());
}
