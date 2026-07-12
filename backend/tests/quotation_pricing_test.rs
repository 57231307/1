//! 销售报价单定价服务测试
//!
//! Week 2 任务 6 - 销售报价单模块
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 6
//!
//! 注意：完整集成测试需要数据库连接，单元测试主要验证
//! 业务逻辑（不涉及 DB 的部分）。

use bingxi_backend::services::quotation_pricing_service::*;
use rust_decimal::Decimal;

#[test]
fn test_vip_discount_rate_is_five_percent() {
    let vip = CustomerLevel::Vip;
    assert_eq!(vip.discount_rate(), Decimal::new(5, 2));
}

#[test]
fn test_normal_discount_rate_is_zero() {
    let normal = CustomerLevel::Normal;
    assert_eq!(normal.discount_rate(), Decimal::ZERO);
}

#[test]
fn test_customer_level_from_code() {
    assert_eq!(CustomerLevel::from_code("VIP"), CustomerLevel::Vip);
    assert_eq!(CustomerLevel::from_code("vip"), CustomerLevel::Vip);
    assert_eq!(CustomerLevel::from_code("NORMAL"), CustomerLevel::Normal);
    assert_eq!(CustomerLevel::from_code("unknown"), CustomerLevel::Normal);
}

#[test]
fn test_pricing_context_serialize() {
    let ctx = PricingContext {
        customer_id: 1,
        customer_level: CustomerLevel::Vip,
        product_id: 100,
        color_id: Some(5),
        quantity: Decimal::from(150),
        currency: "CNY".to_string(),
        quotation_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 16).unwrap(),
    };
    let json = serde_json::to_string(&ctx).unwrap();
    assert!(json.contains("\"customer_id\":1"));
    assert!(json.contains("\"VIP\""));
    assert!(json.contains("\"CNY\""));
}

#[test]
fn test_pricing_result_serialize() {
    let result = PricingResult {
        unit_price: Decimal::from(95),
        unit_price_with_tax: Decimal::from(107),
        tier_breakdown: vec![TierPrice {
            min_quantity: Decimal::ONE,
            max_quantity: None,
            unit_price: Decimal::from(100),
        }],
        discount_applied: Decimal::from(5),
        final_amount: Decimal::from(14250),
        price_source: PriceSource::ColorPrice,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"unit_price\":95"));
    assert!(json.contains("\"color_price\""));
}

#[test]
fn test_tier_price_match_logic() {
    use rust_decimal_macros::dec;
    // 阶梯价匹配单元测试（无 DB 依赖）
    let base = dec!(100);
    let qty = dec!(150);
    let min_q = Some(dec!(100));
    // 当 min_qty <= quantity 时匹配
    let tier = QuotationPricingService::match_tier_for_unit_test(base, qty, min_q);
    assert_eq!(tier.unit_price, dec!(100));
    assert_eq!(tier.min_quantity, dec!(100));

    // 当 min_qty > quantity 时回退到默认
    let tier2 = QuotationPricingService::match_tier_for_unit_test(base, qty, Some(dec!(200)));
    assert_eq!(tier2.unit_price, dec!(100));
    assert_eq!(tier2.min_quantity, Decimal::ONE);
}
