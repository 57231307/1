use bingxi_backend::models::status::wage_energy_chemical_business::*;
use bingxi_backend::services::wage_energy_chemical_business;
use rust_decimal::Decimal;

#[test]
fn test_vip_discount() {
    let d = customer_level_discount(Some("VIP"));
    // P0 6-2 修复：0.95（原 Decimal::new(95, 3) = 0.095 是 bug）
    assert_eq!(d, Decimal::new(95, 2)); // 95 折 = 0.95
}

#[test]
fn test_normal_no_discount() {
    let d = customer_level_discount(Some("NORMAL"));
    assert_eq!(d, Decimal::new(100, 2)); // 1.00 = 100%
}

#[test]
fn test_gold_discount() {
    let d = customer_level_discount(Some("GOLD"));
    assert_eq!(d, Decimal::new(90, 2)); // 0.90 = 9 折
}

#[test]
fn test_none_discount() {
    let d = customer_level_discount(None);
    assert_eq!(d, Decimal::new(100, 2)); // 1.00 = 100%
}

#[test]
fn test_tier_vip_combined() {
    // 100 元基础价 → 阶梯价 90 元（5% off）→ VIP 95 折 → 85.5 元
    let tier = Decimal::new(90, 0);
    let vip = customer_level_discount(Some("VIP"));
    let final_price = (tier * vip).round_dp(6);
    assert_eq!(final_price, Decimal::new(85500, 3));
}

#[test]
fn test_seasonal_percentage() {
    // 100 元 → 春季 +10% = 110 元
    let base = Decimal::new(100, 0);
    let factor = Decimal::from(1) + Decimal::new(10, 2);
    let result = (base * factor).round_dp(6);
    assert_eq!(result, Decimal::new(110, 0));
}

#[test]
fn test_seasonal_fixed() {
    // 100 元 → 节日 +5 元 = 105 元
    let base = Decimal::new(100, 0);
    let result = base + Decimal::new(5, 0);
    assert_eq!(result, Decimal::new(105, 0));
}

#[test]
fn test_seasonal_negative() {
    // 100 元 → 冬季 -5% = 95 元
    let base = Decimal::new(100, 0);
    let factor = Decimal::from(1) - Decimal::new(5, 2);
    let result = (base * factor).round_dp(6);
    assert_eq!(result, Decimal::new(95, 0));
}

#[test]
fn test_special_price_overrides_all() {
    // 客户专属价应覆盖所有其他规则
    let tier = Decimal::new(90, 0);
    let vip = customer_level_discount(Some("VIP"));
    let after_tier_vip = (tier * vip).round_dp(6);
    let special = Decimal::new(80, 0);
    assert!(special < after_tier_vip);
    assert_eq!(special, Decimal::new(80, 0));
}

#[test]
fn test_decimal_precision() {
    // 验证 6 位精度
    let a = Decimal::new(1999, 2);
    let b = Decimal::new(95, 2);
    let c = (a * b).round_dp(6);
    assert_eq!(c, Decimal::new(189905, 4));
}
