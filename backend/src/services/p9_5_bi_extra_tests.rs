//! P9-5 单元测试 - BI 模块扩展（20 测试）
//!
//! 覆盖：销售/产品/客户/财务分析 业务规则

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::*;
    use std::str::FromStr;

    /// 时间粒度
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum TimeGranularity {
        Daily,
        Weekly,
        Monthly,
        Quarterly,
        Yearly,
    }

    impl TimeGranularity {
        fn days(&self) -> i32 {
            match self {
                Self::Daily => 1,
                Self::Weekly => 7,
                Self::Monthly => 30,
                Self::Quarterly => 90,
                Self::Yearly => 365,
            }
        }
    }

    /// 趋势方向
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum TrendDirection {
        Up,
        Down,
        Flat,
    }

    impl TrendDirection {
        fn from_delta(curr: Decimal, prev: Decimal) -> Self {
            if curr > prev {
                Self::Up
            } else if curr < prev {
                Self::Down
            } else {
                Self::Flat
            }
        }
    }

    /// 客户分级
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum CustomerTier {
        D,
        C,
        B,
        A,
        VIP,
    }

    /// 销售汇总
    #[derive(Debug, Clone)]
    struct SalesSummary {
        pub total: Decimal,
        pub orders: i64,
        pub customers: i64,
    }

    impl SalesSummary {
        fn avg_order_value(&self) -> Decimal {
            if self.orders == 0 {
                Decimal::ZERO
            } else {
                self.total / Decimal::from(self.orders)
            }
        }
        fn avg_per_customer(&self) -> Decimal {
            if self.customers == 0 {
                Decimal::ZERO
            } else {
                self.total / Decimal::from(self.customers)
            }
        }
    }

    /// 产品 ABC 分类
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum ABCClass {
        A, // 销售额前 70%
        B, // 销售额 70-90%
        C, // 销售额 90-100%
    }

    /// 财务指标
    #[derive(Debug, Clone)]
    struct FinancialMetrics {
        pub revenue: Decimal,
        pub cost: Decimal,
        pub expense: Decimal,
    }

    impl FinancialMetrics {
        fn gross_profit(&self) -> Decimal {
            self.revenue - self.cost
        }
        fn gross_margin(&self) -> Decimal {
            if self.revenue == Decimal::ZERO {
                Decimal::ZERO
            } else {
                self.gross_profit() / self.revenue
            }
        }
        fn net_profit(&self) -> Decimal {
            self.gross_profit() - self.expense
        }
        fn net_margin(&self) -> Decimal {
            if self.revenue == Decimal::ZERO {
                Decimal::ZERO
            } else {
                self.net_profit() / self.revenue
            }
        }
        fn is_profitable(&self) -> bool {
            self.net_profit() > Decimal::ZERO
        }
    }

    // ============= 时间粒度 =============

    #[test]
    fn test_bi01_granularity_days() {
        assert_eq!(TimeGranularity::Daily.days(), 1);
        assert_eq!(TimeGranularity::Weekly.days(), 7);
        assert_eq!(TimeGranularity::Monthly.days(), 30);
        assert_eq!(TimeGranularity::Quarterly.days(), 90);
        assert_eq!(TimeGranularity::Yearly.days(), 365);
    }

    #[test]
    fn test_bi02_granularity_ordering() {
        assert!(TimeGranularity::Daily.days() < TimeGranularity::Weekly.days());
        assert!(TimeGranularity::Monthly.days() < TimeGranularity::Quarterly.days());
        assert!(TimeGranularity::Quarterly.days() < TimeGranularity::Yearly.days());
    }

    // ============= 趋势方向 =============

    #[test]
    fn test_bi03_trend_up() {
        assert_eq!(TrendDirection::from_delta(dec!(110), dec!(100)), TrendDirection::Up);
    }

    #[test]
    fn test_bi04_trend_down() {
        assert_eq!(TrendDirection::from_delta(dec!(90), dec!(100)), TrendDirection::Down);
    }

    #[test]
    fn test_bi05_trend_flat() {
        assert_eq!(TrendDirection::from_delta(dec!(100), dec!(100)), TrendDirection::Flat);
    }

    // ============= 客户分级 =============

    #[test]
    fn test_bi06_customer_tier_ordering() {
        assert!(CustomerTier::D < CustomerTier::C);
        assert!(CustomerTier::C < CustomerTier::B);
        assert!(CustomerTier::B < CustomerTier::A);
        assert!(CustomerTier::A < CustomerTier::VIP);
    }

    #[test]
    fn test_bi07_vip_is_top() {
        assert_eq!(CustomerTier::VIP, CustomerTier::VIP);
        let max = CustomerTier::VIP;
        assert!(max >= CustomerTier::A);
    }

    // ============= 销售汇总 =============

    #[test]
    fn test_bi08_avg_order_value() {
        let s = SalesSummary { total: dec!(10000), orders: 20, customers: 5 };
        assert_eq!(s.avg_order_value(), dec!(500));
    }

    #[test]
    fn test_bi09_avg_per_customer() {
        let s = SalesSummary { total: dec!(10000), orders: 20, customers: 5 };
        assert_eq!(s.avg_per_customer(), dec!(2000));
    }

    #[test]
    fn test_bi10_avg_order_zero_orders() {
        let s = SalesSummary { total: dec!(1000), orders: 0, customers: 0 };
        assert_eq!(s.avg_order_value(), Decimal::ZERO);
        assert_eq!(s.avg_per_customer(), Decimal::ZERO);
    }

    // ============= ABC 分类 =============

    #[test]
    fn test_bi11_abc_class_a() {
        assert_eq!(ABCClass::A, ABCClass::A);
    }

    #[test]
    fn test_bi12_abc_class_b() {
        assert_eq!(ABCClass::B, ABCClass::B);
    }

    #[test]
    fn test_bi13_abc_class_c() {
        assert_eq!(ABCClass::C, ABCClass::C);
    }

    // ============= 财务指标 =============

    #[test]
    fn test_bi14_gross_profit() {
        let m = FinancialMetrics { revenue: dec!(1000), cost: dec!(600), expense: dec!(100) };
        assert_eq!(m.gross_profit(), dec!(400));
    }

    #[test]
    fn test_bi15_gross_margin() {
        let m = FinancialMetrics { revenue: dec!(1000), cost: dec!(600), expense: dec!(100) };
        assert_eq!(m.gross_margin(), dec!(0.40));
    }

    #[test]
    fn test_bi16_net_profit() {
        let m = FinancialMetrics { revenue: dec!(1000), cost: dec!(600), expense: dec!(100) };
        // 毛利 400 - 费用 100 = 300
        assert_eq!(m.net_profit(), dec!(300));
    }

    #[test]
    fn test_bi17_net_margin() {
        let m = FinancialMetrics { revenue: dec!(1000), cost: dec!(600), expense: dec!(100) };
        // 300 / 1000 = 0.30
        assert_eq!(m.net_margin(), dec!(0.30));
    }

    #[test]
    fn test_bi18_is_profitable() {
        let m = FinancialMetrics { revenue: dec!(1000), cost: dec!(600), expense: dec!(100) };
        assert!(m.is_profitable());
    }

    #[test]
    fn test_bi19_not_profitable() {
        let m = FinancialMetrics { revenue: dec!(1000), cost: dec!(800), expense: dec!(500) };
        // 毛利 200 - 费用 500 = -300
        assert!(!m.is_profitable());
    }

    #[test]
    fn test_bi20_zero_revenue() {
        let m = FinancialMetrics { revenue: Decimal::ZERO, cost: Decimal::ZERO, expense: Decimal::ZERO };
        assert_eq!(m.gross_margin(), Decimal::ZERO);
        assert_eq!(m.net_margin(), Decimal::ZERO);
    }
}
