//! 销售报价单转订单服务测试
//!
//! Week 2 任务 8 - 销售报价单模块
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 8

use crate::models::sales_quotation_item;
use rust_decimal::Decimal;

#[test]
fn test_compose_color_no_empty_returns_dash() {
    let item = sales_quotation_item::Model {
        id: 1,
        quotation_id: 1,
        product_id: 1,
        color_id: None,
        color_code: None,
        pantone_code: None,
        cncs_code: None,
        specification: None,
        unit: "米".to_string(),
        quantity: Decimal::from(10),
        unit_price: Decimal::from(10),
        unit_price_with_tax: Decimal::from(11),
        amount: Decimal::from(100),
        amount_with_tax: Decimal::from(113),
        tier_pricing: None,
        discount_rate: None,
        discount_amount: None,
        notes: None,
        sequence: 0,
    };
    let s = crate::services::quotation_convert_service::QuotationConvertService::compose_color_no(&item);
    assert_eq!(s, "-");
}

#[test]
fn test_compose_color_no_with_color_code() {
    let item = sales_quotation_item::Model {
        id: 1,
        quotation_id: 1,
        product_id: 1,
        color_id: None,
        color_code: Some("BLUE-09".to_string()),
        pantone_code: None,
        cncs_code: None,
        specification: None,
        unit: "米".to_string(),
        quantity: Decimal::from(10),
        unit_price: Decimal::from(10),
        unit_price_with_tax: Decimal::from(11),
        amount: Decimal::from(100),
        amount_with_tax: Decimal::from(113),
        tier_pricing: None,
        discount_rate: None,
        discount_amount: None,
        notes: None,
        sequence: 0,
    };
    let s = crate::services::quotation_convert_service::QuotationConvertService::compose_color_no(&item);
    assert_eq!(s, "BLUE-09");
}

#[test]
fn test_compose_color_no_with_pantone_and_cncs() {
    let item = sales_quotation_item::Model {
        id: 1,
        quotation_id: 1,
        product_id: 1,
        color_id: None,
        color_code: Some("RED-01".to_string()),
        pantone_code: Some("18-1664".to_string()),
        cncs_code: Some("S1080-Y90R".to_string()),
        specification: None,
        unit: "米".to_string(),
        quantity: Decimal::from(10),
        unit_price: Decimal::from(10),
        unit_price_with_tax: Decimal::from(11),
        amount: Decimal::from(100),
        amount_with_tax: Decimal::from(113),
        tier_pricing: None,
        discount_rate: None,
        discount_amount: None,
        notes: None,
        sequence: 0,
    };
    let s = crate::services::quotation_convert_service::QuotationConvertService::compose_color_no(&item);
    assert!(s.contains("RED-01"));
    assert!(s.contains("PANTONE:18-1664"));
    assert!(s.contains("CNCS:S1080-Y90R"));
}
