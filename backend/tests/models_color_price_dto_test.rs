use bingxi_backend::models::color_price_dto::*;
use rust_decimal::Decimal;
use serde_json::json;
use chrono::NaiveDate;

// ===== DTO 测试 =====

#[test]
fn test_color_price_create_dto() {
    let dto = CreateColorPriceDto {
        product_id: 1,
        color_id: 1,
        base_price: Decimal::new(1000, 2),
        currency: "CNY".to_string(),
        effective_from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        effective_to: Some(chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        customer_level: Some("VIP".to_string()),
        min_quantity: Some(Decimal::new(100, 0)),
        max_quantity: Some(Decimal::new(1000, 0)),
        customer_id: None,
        season: None,
        priority: Some(0),
        notes: Some("测试备注".to_string()),
    };

    assert_eq!(dto.product_id, 1);
    assert_eq!(dto.color_id, 1);
    assert_eq!(dto.base_price, Decimal::new(1000, 2));
}

#[test]
fn test_color_price_update_dto() {
    let dto = UpdateColorPriceDto {
        base_price: Some(Decimal::new(1200, 2)),
        currency: Some("USD".to_string()),
        effective_from: Some(chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
        effective_to: Some(chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        customer_level: None,
        min_quantity: Some(Decimal::new(50, 0)),
        max_quantity: Some(Decimal::new(500, 0)),
        customer_id: None,
        season: None,
        is_active: Some(true),
        priority: None,
        notes: Some("更新备注".to_string()),
    };

    assert_eq!(dto.base_price, Some(Decimal::new(1200, 2)));
}

#[test]
fn test_color_price_query_dto() {
    let dto = ListColorPricesQuery {
        product_id: Some(1),
        color_id: Some(1),
        page: Some(1),
        page_size: Some(10),
        customer_id: None,
        customer_level: None,
        season: None,
        currency: None,
        is_active: Some(true),
        approval_status: None,
        keyword: None,
    };

    assert_eq!(dto.product_id, Some(1));
    assert_eq!(dto.page, Some(1));
}

// ===== 序列化测试 =====

#[test]
fn test_create_color_price_dto_serialization() {
    let dto = CreateColorPriceDto {
        product_id: 1,
        color_id: 1,
        base_price: Decimal::new(1000, 2),
        currency: "CNY".to_string(),
        effective_from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        effective_to: Some(chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        customer_level: None,
        min_quantity: Some(Decimal::new(100, 0)),
        max_quantity: Some(Decimal::new(1000, 0)),
        customer_id: None,
        season: None,
        priority: None,
        notes: Some("测试备注".to_string()),
    };

    let json = serde_json::to_value(&dto).expect("序列化失败");
    assert_eq!(json["product_id"], 1);
    assert_eq!(json["base_price"], "10.00");
}

#[test]
fn test_color_price_query_dto_deserialization() {
    let json = json!({
        "product_id": 1,
        "color_id": 1,
        "page": 1,
        "page_size": 10
    });

    let dto: ListColorPricesQuery = serde_json::from_value(json).expect("反序列化失败");
    assert_eq!(dto.product_id, Some(1));
    assert_eq!(dto.page, Some(1));
}

// ===== 价格计算测试 =====

#[test]
fn test_price_with_quantity_discount() {
    let price_val = Decimal::new(1000, 2);
    let discount_rate = Decimal::new(90, 2); // 90%
    let final_price = price_val * discount_rate;

    assert_eq!(final_price, Decimal::new(900, 2));
}

#[test]
fn test_price_range() {
    let min_price = Decimal::new(500, 2);
    let max_price = Decimal::new(2000, 2);
    let actual_price = Decimal::new(1000, 2);

    assert!(actual_price >= min_price);
    assert!(actual_price <= max_price);
}
