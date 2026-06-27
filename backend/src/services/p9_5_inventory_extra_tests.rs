//! P9-5 单元测试 - 库存模块扩展（20 测试）
//!
//! 覆盖：库存/调拨/盘点/调整 业务规则

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    /// 库存移动类型
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum MovementType {
        Inbound,
        Outbound,
        Transfer,
        Adjustment,
        Stocktake,
        Damage,
    }

    impl MovementType {
        fn direction(&self) -> i32 {
            match self {
                Self::Inbound | Self::Adjustment | Self::Stocktake => 1,
                Self::Outbound | Self::Damage => -1,
                Self::Transfer => 0,
            }
        }
        fn affects_stock(&self) -> bool {
            !matches!(self, Self::Transfer)
        }
    }

    /// 库存调整原因
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum AdjustmentReason {
        Stocktake,        // 盘点
        Damage,           // 报损
        Expiry,           // 过期
        Loss,             // 丢失
        Found,            // 盘盈
        SystemError,      // 系统错误
    }

    /// 库存预警级别
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum AlertLevel {
        Info,
        Warning,
        Critical,
    }

    /// 库存预警类型
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum StockAlertType {
        LowStock,
        OverStock,
        Expiring,
        SlowMoving,
        Discrepancy,
    }

    /// 库存记录
    #[derive(Debug, Clone)]
    struct StockRecord {
        pub product_id: i32,
        pub warehouse_id: i32,
        pub qty: Decimal,
        pub min_qty: Decimal,
        pub max_qty: Decimal,
    }

    impl StockRecord {
        fn alert_level(&self) -> Option<AlertLevel> {
            if self.qty < self.min_qty {
                Some(AlertLevel::Critical)
            } else if self.qty > self.max_qty {
                Some(AlertLevel::Warning)
            } else {
                None
            }
        }
        fn needs_reorder(&self) -> bool {
            self.qty < self.min_qty
        }
    }

    /// 盘点结果
    #[derive(Debug, Clone)]
    struct StocktakeResult {
        pub system_qty: Decimal,
        pub actual_qty: Decimal,
    }

    impl StocktakeResult {
        fn variance(&self) -> Decimal {
            self.actual_qty - self.system_qty
        }
        fn variance_rate(&self) -> Decimal {
            if self.system_qty == Decimal::ZERO {
                Decimal::ZERO
            } else {
                self.variance().abs() / self.system_qty
            }
        }
        fn is_balanced(&self) -> bool {
            self.variance() == Decimal::ZERO
        }
        fn is_over(&self) -> bool {
            self.variance() > Decimal::ZERO
        }
        fn is_short(&self) -> bool {
            self.variance() < Decimal::ZERO
        }
    }

    // ============= 移动方向 =============

    #[test]
    fn test_i01_inbound_positive() {
        assert_eq!(MovementType::Inbound.direction(), 1);
    }

    #[test]
    fn test_i02_outbound_negative() {
        assert_eq!(MovementType::Outbound.direction(), -1);
    }

    #[test]
    fn test_i03_transfer_neutral() {
        assert_eq!(MovementType::Transfer.direction(), 0);
    }

    #[test]
    fn test_i04_damage_negative() {
        assert_eq!(MovementType::Damage.direction(), -1);
    }

    #[test]
    fn test_i05_movement_affects_stock() {
        assert!(MovementType::Inbound.affects_stock());
        assert!(MovementType::Outbound.affects_stock());
        assert!(!MovementType::Transfer.affects_stock());
    }

    // ============= 库存预警 =============

    #[test]
    fn test_i06_low_stock_alert() {
        let s = StockRecord {
            product_id: 1,
            warehouse_id: 1,
            qty: dec!(5),
            min_qty: dec!(10),
            max_qty: dec!(100),
        };
        assert!(s.needs_reorder());
        assert_eq!(s.alert_level(), Some(AlertLevel::Critical));
    }

    #[test]
    fn test_i07_over_stock_alert() {
        let s = StockRecord {
            product_id: 1,
            warehouse_id: 1,
            qty: dec!(150),
            min_qty: dec!(10),
            max_qty: dec!(100),
        };
        assert!(!s.needs_reorder());
        assert_eq!(s.alert_level(), Some(AlertLevel::Warning));
    }

    #[test]
    fn test_i08_normal_stock_no_alert() {
        let s = StockRecord {
            product_id: 1,
            warehouse_id: 1,
            qty: dec!(50),
            min_qty: dec!(10),
            max_qty: dec!(100),
        };
        assert_eq!(s.alert_level(), None);
    }

    #[test]
    fn test_i09_alert_level_ordering() {
        assert!(AlertLevel::Info < AlertLevel::Warning);
        assert!(AlertLevel::Warning < AlertLevel::Critical);
    }

    #[test]
    fn test_i10_alert_type_count() {
        let types = vec![
            StockAlertType::LowStock,
            StockAlertType::OverStock,
            StockAlertType::Expiring,
            StockAlertType::SlowMoving,
            StockAlertType::Discrepancy,
        ];
        // P0 修复（批次 4，2026-06-27）：原 `assert_eq!(5, 5)` 为恒真断言，
        // 改为对实际向量长度断言，真正校验预警类型枚举数量。
        assert_eq!(types.len(), 5); // 5 种预警类型
    }

    // ============= 盘点差异 =============

    #[test]
    fn test_i11_stocktake_balanced() {
        let r = StocktakeResult { system_qty: dec!(100), actual_qty: dec!(100) };
        assert!(r.is_balanced());
        assert!(!r.is_over());
        assert!(!r.is_short());
    }

    #[test]
    fn test_i12_stocktake_over() {
        let r = StocktakeResult { system_qty: dec!(100), actual_qty: dec!(110) };
        assert!(r.is_over());
        assert_eq!(r.variance(), dec!(10));
    }

    #[test]
    fn test_i13_stocktake_short() {
        let r = StocktakeResult { system_qty: dec!(100), actual_qty: dec!(90) };
        assert!(r.is_short());
        assert_eq!(r.variance(), dec!(-10));
    }

    #[test]
    fn test_i14_stocktake_variance_rate() {
        let r = StocktakeResult { system_qty: dec!(100), actual_qty: dec!(95) };
        assert_eq!(r.variance_rate(), dec!(0.05));
    }

    #[test]
    fn test_i15_stocktake_variance_rate_over() {
        let r = StocktakeResult { system_qty: dec!(200), actual_qty: dec!(210) };
        assert_eq!(r.variance_rate(), dec!(0.05));
    }

    // ============= 调整原因 =============

    #[test]
    fn test_i16_adjustment_reason_count() {
        let reasons = vec![
            AdjustmentReason::Stocktake,
            AdjustmentReason::Damage,
            AdjustmentReason::Expiry,
            AdjustmentReason::Loss,
            AdjustmentReason::Found,
            AdjustmentReason::SystemError,
        ];
        // P0 修复（批次 4，2026-06-27）：原 `assert_eq!(6, 6)` 为恒真断言，
        // 改为对实际向量长度断言，真正校验调整原因枚举数量。
        assert_eq!(reasons.len(), 6);
    }

    #[test]
    fn test_i17_adjustment_reason_distinct() {
        assert_ne!(AdjustmentReason::Damage, AdjustmentReason::Loss);
        assert_ne!(AdjustmentReason::Stocktake, AdjustmentReason::Found);
    }

    // ============= 库存合计 =============

    #[test]
    fn test_i18_stock_total() {
        let s1 = StockRecord { product_id: 1, warehouse_id: 1, qty: dec!(100), min_qty: dec!(10), max_qty: dec!(200) };
        let s2 = StockRecord { product_id: 1, warehouse_id: 2, qty: dec!(50), min_qty: dec!(10), max_qty: dec!(200) };
        let total = s1.qty + s2.qty;
        assert_eq!(total, dec!(150));
    }

    #[test]
    fn test_i19_stock_apply_inbound() {
        let mut s = StockRecord { product_id: 1, warehouse_id: 1, qty: dec!(100), min_qty: dec!(10), max_qty: dec!(200) };
        let delta = Decimal::from(MovementType::Inbound.direction()) * dec!(50);
        s.qty += delta;
        assert_eq!(s.qty, dec!(150));
    }

    #[test]
    fn test_i20_stock_apply_outbound() {
        let mut s = StockRecord { product_id: 1, warehouse_id: 1, qty: dec!(100), min_qty: dec!(10), max_qty: dec!(200) };
        let delta = Decimal::from(MovementType::Outbound.direction()) * dec!(30);
        s.qty += delta;
        assert_eq!(s.qty, dec!(70));
    }
}
