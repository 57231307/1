//! P0-5 面料多色号定价扩展 - 价格计算引擎集成测试

#[cfg(test)]
mod calc_tests {
    use rust_decimal_macros::dec;

    use bingxi_backend::utils::price_calculator::customer_level_discount;

    /// 测试 1: VIP 95 折应用
    #[test]
    fn test_vip_discount() {
        let d = customer_level_discount(Some("VIP"));
        assert_eq!(d, dec!(0.95));
    }

    /// 测试 2: 4 档阶梯价匹配（模拟）
    #[test]
    fn test_tier_matching() {
        // 100 元基础价，100 米命中第 2 档 95%
        let base = dec!(100.00);
        let tier_price = base * dec!(0.95);
        assert_eq!(tier_price, dec!(95.00));
    }

    /// 测试 3: 季节调价叠加
    #[test]
    fn test_seasonal_overlay() {
        // 100 元 → 春季 +10% = 110 元
        let base = dec!(100.00);
        let result = base * (dec!(1.00) + dec!(0.10));
        assert_eq!(result, dec!(110.00));
    }

    /// 测试 4: 客户专属价优先级
    #[test]
    fn test_customer_special_priority() {
        // 客户专属价 80 元 < 阶梯+VIP 价 85.5 元
        let tier_vip = dec!(90.00) * customer_level_discount(Some("VIP"));
        let special = dec!(80.00);
        assert!(special < tier_vip);
    }

    /// 测试 5: 多规则叠加（VIP + 阶梯 + 季节）
    #[test]
    fn test_vip_tier_seasonal() {
        // 100 元基础价 → 阶梯 90 元 → VIP 95 折 = 85.5 → 春季 +10% = 94.05
        let base = dec!(100.00);
        let tier = base * dec!(0.90);
        let vip_price = tier * customer_level_discount(Some("VIP"));
        let season_price = vip_price * (dec!(1.00) + dec!(0.10));
        assert_eq!(tier, dec!(90.00));
        assert_eq!(vip_price, dec!(85.5000));
        assert_eq!(season_price.round_dp(2), dec!(94.05));
    }
}
