//! DualUnitConverter 单元测试

use bingxi_backend::utils::dual_unit_converter::DualUnitConverter;
use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;

#[test]
fn test_meters_to_kg_basic() {
    let quantity = Decimal::from_str("1000").unwrap();
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width).unwrap();
    assert_eq!(result, Decimal::from_str("306.000").unwrap());
}

#[test]
fn test_meters_to_kg_small_quantity() {
    let quantity = Decimal::from_str("1").unwrap();
    let gram_weight = Decimal::from_str("200").unwrap();
    let width = Decimal::from_str("150").unwrap();

    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width).unwrap();
    // 1 * 200 * 1.5 / 1000 = 0.3
    assert_eq!(result, Decimal::from_str("0.300").unwrap());
}

#[test]
fn test_kg_to_meters_basic() {
    let quantity = Decimal::from_str("306").unwrap();
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let result = DualUnitConverter::kg_to_meters(quantity, gram_weight, width).unwrap();
    assert_eq!(result, Decimal::from_str("1000.00").unwrap());
}

#[test]
fn test_kg_to_meters_small_quantity() {
    let quantity = Decimal::from_str("0.3").unwrap();
    let gram_weight = Decimal::from_str("200").unwrap();
    let width = Decimal::from_str("150").unwrap();

    let result = DualUnitConverter::kg_to_meters(quantity, gram_weight, width).unwrap();
    // 0.3 * 1000 / 200 / 1.5 = 1
    assert_eq!(result, Decimal::from_str("1.00").unwrap());
}

#[test]
fn test_basic_meters_to_kg_works() {
    let quantity = Decimal::from_str("100").unwrap();
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    // 业务调用 auto_convert 已删除，验证 DualUnitConverter::meters_to_kg
    // 仍可被业务调用的基本路径即可
    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width);
    assert!(result.is_ok());
}

#[test]
fn test_validate_dual_unit_valid() {
    let meters = Decimal::from_str("1000").unwrap();
    let kg = Decimal::from_str("306").unwrap();
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let result =
        DualUnitConverter::validate_dual_unit(meters, kg, gram_weight, width, None).unwrap();
    assert!(result);
}

#[test]
fn test_validate_dual_unit_invalid() {
    let meters = Decimal::from_str("1000").unwrap();
    let kg = Decimal::from_str("500").unwrap(); // 错误的公斤数
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let result =
        DualUnitConverter::validate_dual_unit(meters, kg, gram_weight, width, None).unwrap();
    assert!(!result);
}

#[test]
fn test_validate_dual_unit_custom_tolerance() {
    let meters = Decimal::from_str("1000").unwrap();
    let kg = Decimal::from_str("308").unwrap(); // 略微偏差
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    // 使用 1% 容差
    let tolerance = Decimal::from_str("0.01").unwrap();
    let result =
        DualUnitConverter::validate_dual_unit(meters, kg, gram_weight, width, Some(tolerance))
            .unwrap();
    assert!(result);
}

#[test]
fn test_calculate_conversion_rate() {
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let rate = DualUnitConverter::calculate_conversion_rate(gram_weight, width).unwrap();
    // 1000 / (170 * 1.8) = 1000 / 306 ≈ 3.2680
    assert!(rate > Decimal::from_str("3.2").unwrap());
    assert!(rate < Decimal::from_str("3.3").unwrap());
}

#[test]
fn test_negative_meters_should_fail() {
    let quantity = Decimal::from_str("-100").unwrap();
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "米数不能为负数");
}

#[test]
fn test_negative_kg_should_fail() {
    let quantity = Decimal::from_str("-100").unwrap();
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let result = DualUnitConverter::kg_to_meters(quantity, gram_weight, width);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "公斤数不能为负数");
}

#[test]
fn test_zero_gram_weight_should_fail() {
    let quantity = Decimal::from_str("100").unwrap();
    let gram_weight = Decimal::from_str("0").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "克重必须大于 0");
}

#[test]
fn test_zero_width_should_fail() {
    let quantity = Decimal::from_str("100").unwrap();
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("0").unwrap();

    let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "幅宽必须大于 0");
}

#[test]
fn test_conversion_roundtrip() {
    let original_meters = Decimal::from_str("1000").unwrap();
    let gram_weight = Decimal::from_str("170").unwrap();
    let width = Decimal::from_str("180").unwrap();

    let kg = DualUnitConverter::meters_to_kg(original_meters, gram_weight, width).unwrap();
    let back_to_meters = DualUnitConverter::kg_to_meters(kg, gram_weight, width).unwrap();

    // 由于四舍五入，往返转换应该非常接近原始值
    let diff = (original_meters - back_to_meters).abs();
    assert!(diff < Decimal::from_str("0.1").unwrap());
}
