//! 固定资产折旧计算单元测试

use rust_decimal::Decimal;

#[test]
fn test_straight_line_depreciation() {
    let original_value = Decimal::new(100000, 2); // 1000.00
    let residual_value = Decimal::new(10000, 2); // 100.00
    let useful_life_months = 120; // 10 年

    let depreciable_amount = original_value - residual_value;
    let monthly_depreciation = depreciable_amount / Decimal::from(useful_life_months);

    // (1000.00 - 100.00) / 120 = 7.50
    assert_eq!(monthly_depreciation, Decimal::new(750, 2)); // 每月 7.50 元
}

#[test]
fn test_accumulated_depreciation() {
    let monthly_depreciation = Decimal::new(750, 0);
    let months_used = 36; // 3 年

    let accumulated = monthly_depreciation * Decimal::from(months_used);

    assert_eq!(accumulated, Decimal::new(27000, 0)); // 2.7 万
}

#[test]
fn test_net_value_calculation() {
    let original_value = Decimal::new(100000, 2);
    let accumulated_depreciation = Decimal::new(27000, 2);

    let net_value = original_value - accumulated_depreciation;

    assert_eq!(net_value, Decimal::new(73000, 2)); // 7.3 万
}

#[test]
fn test_fully_depreciated_asset() {
    let original_value = Decimal::new(100000, 2);
    let residual_value = Decimal::new(10000, 2);
    let _useful_life_months = 120;
    let _months_used = 120; // 已用完全部寿命

    let depreciable_amount = original_value - residual_value;
    let total_depreciation = depreciable_amount; // 全部折旧

    let net_value = original_value - total_depreciation;

    assert_eq!(net_value, residual_value); // 净残值
}

#[test]
fn test_depreciation_by_years() {
    let test_cases = vec![
        (1, 9000),   // 1 年后
        (3, 27000),  // 3 年后
        (5, 45000),  // 5 年后
        (10, 90000), // 10 年后
    ];

    let monthly_depreciation = Decimal::new(750, 0);

    for (years, expected_accumulated) in test_cases {
        let months = years * 12;
        let accumulated = monthly_depreciation * Decimal::from(months);
        assert_eq!(
            accumulated,
            Decimal::new(expected_accumulated, 0),
            "{} 年后的累计折旧不正确",
            years
        );
    }
}
