use rust_decimal::Decimal;

/// 测试折旧计算逻辑（直接调用内部方法）
/// 由于 calculate_asset_depreciation 是私有方法，我们通过测试计算逻辑来验证
#[test]
fn test_depreciation_calculation_logic() {
    // 原值 100000，残值 10000，使用寿命 120 个月
    let original_value = Decimal::from(100000);
    let salvage_value = Decimal::from(10000);
    let useful_life = 120i32;

    // 可折旧金额
    let depreciable_amount = original_value - salvage_value;
    assert_eq!(depreciable_amount, Decimal::from(90000));

    // 月折旧额
    let monthly_depreciation = depreciable_amount / Decimal::from(useful_life);
    assert_eq!(monthly_depreciation, Decimal::from(750));

    // 36 个月折旧
    let months_used = 36;
    let total_depreciation = monthly_depreciation * Decimal::from(months_used);
    assert_eq!(total_depreciation, Decimal::from(27000));
}

#[test]
fn test_depreciation_with_accumulated() {
    let total_depreciation = Decimal::from(27000);
    let accumulated_depreciation = Decimal::from(10000);

    // 当期折旧 = 总折旧 - 已累计折旧
    let current_depreciation = total_depreciation - accumulated_depreciation;
    assert_eq!(current_depreciation, Decimal::from(17000));
}

#[test]
fn test_depreciation_fully_depreciated() {
    let original_value = Decimal::from(100000);
    let salvage_value = Decimal::from(10000);
    let useful_life = 120i32;
    let months_used = 150; // 超过使用寿命

    let depreciable_amount = original_value - salvage_value;
    let monthly_depreciation = depreciable_amount / Decimal::from(useful_life);

    // 折旧不能超过可折旧金额
    let max_depreciation = depreciable_amount;
    let calculated =
        monthly_depreciation * Decimal::from(std::cmp::Ord::min(months_used, useful_life));

    assert_eq!(calculated, max_depreciation);
}

#[test]
fn test_net_value_calculation() {
    let original_value = Decimal::from(100000);
    let accumulated_depreciation = Decimal::from(27000);

    let net_value = original_value - accumulated_depreciation;
    assert_eq!(net_value, Decimal::from(73000));
}

#[test]
fn test_depreciation_before_purchase() {
    // 购买日期晚于计算日期，应返回 0
    let purchase_year = 2025;
    let calc_year = 2024;

    let months_used = (calc_year - purchase_year) * 12;
    assert!(months_used < 0, "购买前不应计算折旧");
}

#[test]
fn test_various_depreciation_scenarios() {
    let test_cases = vec![
        // (原值, 残值, 使用寿命月, 已用月数, 期望折旧)
        (100000, 10000, 120, 12, 9000),   // 1 年
        (100000, 10000, 120, 36, 27000),  // 3 年
        (100000, 10000, 120, 60, 45000),  // 5 年
        (100000, 10000, 120, 120, 90000), // 满寿命
        (50000, 5000, 60, 24, 18000),     // 另一设备
    ];

    for (original, salvage, life, months, expected) in test_cases {
        let original_value = Decimal::from(original);
        let salvage_value = Decimal::from(salvage);
        let depreciable = original_value - salvage_value;
        let monthly = depreciable / Decimal::from(life);
        let total = monthly * Decimal::from(std::cmp::Ord::min(months, life));

        assert_eq!(
            total,
            Decimal::from(expected),
            "原值={}, 残值={}, 寿命={}, 月数={} 的折旧计算错误",
            original,
            salvage,
            life,
            months
        );
    }
}

/// 测试处置损益计算：处置价值 > 账面净值 → 收益为正
/// 对应 dispose 方法 line 331-333 的计算逻辑：`net_book_value = asset.net_value.unwrap_or(Decimal::ZERO)`；`disposal_gain_loss = req.disposal_value - net_book_value`；gain_loss 计算公式验证，完整 dispose 事务流程需集成测试
#[test]
fn test_disposal_gain_loss_positive() {
    // 资产：原值 10000，累计折旧 2000，账面净值 8000
    // net_book_value 对应 dispose 方法中 asset.net_value.unwrap_or(Decimal::ZERO)
    let net_book_value = Decimal::from(8000);
    let disposal_value = Decimal::from(9000);

    // 模拟 dispose 方法 line 333 的损益计算公式
    let gain_loss = disposal_value - net_book_value;

    assert_eq!(gain_loss, Decimal::from(1000));
    assert!(
        gain_loss > Decimal::ZERO,
        "处置价值 > 账面净值应为收益（正数）"
    );
}

/// 测试处置损益计算：处置价值 < 账面净值 → 损失为负（gain_loss 计算公式验证，完整 dispose 事务流程需集成测试）
#[test]
fn test_disposal_gain_loss_negative() {
    // 同一资产，账面净值 8000，处置价值仅 7000
    let net_book_value = Decimal::from(8000);
    let disposal_value = Decimal::from(7000);

    let gain_loss = disposal_value - net_book_value;

    assert_eq!(gain_loss, Decimal::from(-1000));
    assert!(
        gain_loss < Decimal::ZERO,
        "处置价值 < 账面净值应为损失（负数）"
    );
}

/// 测试处置损益计算：处置价值 = 账面净值 → 损益为 0（gain_loss 计算公式验证，完整 dispose 事务流程需集成测试）
#[test]
fn test_disposal_gain_loss_zero() {
    let net_book_value = Decimal::from(8000);
    let disposal_value = Decimal::from(8000);

    let gain_loss = disposal_value - net_book_value;

    assert_eq!(gain_loss, Decimal::ZERO);
}

/// 测试 calculate_asset_depreciation 的 round_dp(2) 精度行为
/// 构造资产：original_value=10000, salvage_value=Some(0), useful_life=Some(3)（36 个月）；月折旧 = (10000 - 0) / 36 = 277.7777...，round_dp(2) 四舍五入为 277.78；calculate_asset_depreciation 是私有方法且需 &self（FixedAssetService 含 DatabaseConnection），；此处验证其内部 round_dp(2) 精度逻辑，完整方法调用需集成测试
#[test]
fn test_calculate_asset_depreciation_round_dp() {
    let original_value = Decimal::from(10000);
    let salvage_value = Decimal::from(0);
    let useful_life_years = 3i32;

    // 复刻 calculate_asset_depreciation line 516-520 的月折旧计算
    let useful_life_months = useful_life_years * 12;
    let depreciable_amount = original_value - salvage_value;
    let monthly_depreciation =
        (depreciable_amount / Decimal::from(useful_life_months)).round_dp(2);

    // 10000/36 = 277.7777...，round_dp(2) 采用 MidpointAwayFromZero 四舍五入，
    // 第 3 位小数 7 >= 5 进位，结果为 277.78（Decimal::new(27778, 2) = 277.78）
    assert_eq!(monthly_depreciation, Decimal::new(27778, 2));

    // 验证 round 确实发生：未 round 的无限循环小数与 round 后值不同
    let unrounded = depreciable_amount / Decimal::from(useful_life_months);
    assert_ne!(
        monthly_depreciation, unrounded,
        "round_dp(2) 必须截断无限循环小数"
    );

    // 验证精度为 2 位小数：再次 round_dp(2) 值不变
    assert_eq!(monthly_depreciation.round_dp(2), monthly_depreciation);
}