// P9-1: 用统一宏替代散落的 expect 调用，集中到 unwrap_safe 模块
// 批次 343 v11 复审 P3 修复：移除 #[allow(unused_imports)]，dec! 宏已被广泛使用
use bingxi_backend::dec;
use bingxi_backend::utils::dual_unit_converter::DualUnitConverter;

#[test]
fn test_meters_to_kg_basic() {
    let quantity = dec!(1000.0); // 1000 米
    let gram_weight = dec!(170.0); // 170g/m²
    let width = dec!(180.0); // 180cm

    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width)
        .expect("conversion should succeed");

    // 预期：1000 × 170 × 1.8 ÷ 1000 = 306 公斤
    assert_eq!(result, dec!(306.0));
}

#[test]
fn test_kg_to_meters_basic() {
    let quantity = dec!(306.0); // 306 公斤
    let gram_weight = dec!(170.0); // 170g/m²
    let width = dec!(180.0); // 180cm

    let result = DualUnitConverter::kg_to_meters(quantity, gram_weight, width)
        .expect("conversion should succeed");

    // 预期：306 × 1000 ÷ 170 ÷ 1.8 = 1000 米
    assert_eq!(result, dec!(1000.0));
}

#[test]
fn test_validate_dual_unit_valid() {
    let quantity_meters = dec!(1000.0);
    let quantity_kg = dec!(306.0);
    let gram_weight = dec!(170.0);
    let width = dec!(180.0);

    let is_valid = DualUnitConverter::validate_dual_unit(
        quantity_meters,
        quantity_kg,
        gram_weight,
        width,
        None,
    )
    .expect("validation should succeed");

    assert!(is_valid);
}

#[test]
fn test_validate_dual_unit_invalid() {
    let quantity_meters = dec!(1000.0);
    let quantity_kg = dec!(350.0); // 错误的公斤数
    let gram_weight = dec!(170.0);
    let width = dec!(180.0);

    let is_valid = DualUnitConverter::validate_dual_unit(
        quantity_meters,
        quantity_kg,
        gram_weight,
        width,
        None,
    )
    .expect("validation should succeed");

    assert!(!is_valid);
}

#[test]
fn test_negative_quantity_should_fail() {
    let quantity = dec!(-100.0);
    let gram_weight = dec!(170.0);
    let width = dec!(180.0);

    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "米数不能为负数");
}

#[test]
fn test_zero_gram_weight_should_fail() {
    let quantity = dec!(1000.0);
    let gram_weight = dec!(0.0);
    let width = dec!(180.0);

    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "克重必须大于 0");
}
