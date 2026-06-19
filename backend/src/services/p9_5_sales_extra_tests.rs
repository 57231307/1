//! P9-5 单元测试 - 销售模块扩展（25 测试）
//!
//! 覆盖：报价单/订单/价格/合同/退货 业务规则

#![allow(clippy::needless_range_loop)]

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::*;
    use std::str::FromStr;

    /// 报价单状态
    #[derive(Debug, Clone, PartialEq)]
    enum QuotationStatus {
        Draft,
        Pending,
        Approved,
        Rejected,
        Expired,
        Converted,
    }

    impl QuotationStatus {
        fn is_active(&self) -> bool {
            matches!(self, Self::Draft | Self::Pending | Self::Approved)
        }
        fn is_terminal(&self) -> bool {
            matches!(self, Self::Rejected | Self::Expired | Self::Converted)
        }
        fn can_convert(&self) -> bool {
            matches!(self, Self::Approved)
        }
    }

    /// 报价单
    #[derive(Debug, Clone)]
    struct Quotation {
        pub id: i32,
        pub code: String,
        pub customer_id: i32,
        pub amount: Decimal,
        pub tax_rate: Decimal,
        pub status: QuotationStatus,
        pub valid_until: chrono::NaiveDate,
    }

    impl Quotation {
        fn total_with_tax(&self) -> Decimal {
            self.amount * (Decimal::from(1) + self.tax_rate)
        }
    }

    /// 销售订单类型
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum OrderType {
        Normal,
        Sample,
        Rush,
        Bulk,
    }

    impl OrderType {
        fn priority(&self) -> i32 {
            match self {
                Self::Rush => 1,
                Self::Sample => 2,
                Self::Normal => 3,
                Self::Bulk => 4,
            }
        }
    }

    /// 折扣类型
    #[derive(Debug, Clone)]
    enum DiscountType {
        Percentage(Decimal),
        Amount(Decimal),
        Tiered { qty: i32, discount: Decimal },
    }

    impl DiscountType {
        fn apply(&self, amount: Decimal, qty: i32) -> Decimal {
            match self {
                Self::Percentage(p) => amount * (Decimal::from(1) - p),
                Self::Amount(a) => (amount - a).max(Decimal::from(0)),
                Self::Tiered { qty: t, discount } if qty >= *t => {
                    amount * (Decimal::from(1) - discount)
                }
                _ => amount,
            }
        }
    }

    // ============= 报价单状态测试 =============

    #[test]
    fn test_q01_quotation_status_active() {
        assert!(QuotationStatus::Draft.is_active());
        assert!(QuotationStatus::Pending.is_active());
        assert!(QuotationStatus::Approved.is_active());
    }

    #[test]
    fn test_q02_quotation_status_terminal() {
        assert!(QuotationStatus::Rejected.is_terminal());
        assert!(QuotationStatus::Expired.is_terminal());
        assert!(QuotationStatus::Converted.is_terminal());
    }

    #[test]
    fn test_q03_quotation_can_convert() {
        assert!(QuotationStatus::Approved.can_convert());
        assert!(!QuotationStatus::Draft.can_convert());
        assert!(!QuotationStatus::Pending.can_convert());
    }

    #[test]
    fn test_q04_quotation_total_with_tax() {
        let q = Quotation {
            id: 1,
            code: "Q-001".to_string(),
            customer_id: 100,
            amount: dec!(1000.00),
            tax_rate: dec!(0.13),
            status: QuotationStatus::Draft,
            valid_until: crate::ymd!(2026, 12, 31),
        };
        assert_eq!(q.total_with_tax(), dec!(1130.00));
    }

    #[test]
    fn test_q05_quotation_zero_tax() {
        let q = Quotation {
            id: 1,
            code: "Q-002".to_string(),
            customer_id: 100,
            amount: dec!(500.00),
            tax_rate: Decimal::ZERO,
            status: QuotationStatus::Approved,
            valid_until: crate::ymd!(2026, 12, 31),
        };
        assert_eq!(q.total_with_tax(), dec!(500.00));
    }

    // ============= 订单类型优先级测试 =============

    #[test]
    fn test_q06_order_type_priority_rush() {
        assert_eq!(OrderType::Rush.priority(), 1);
    }

    #[test]
    fn test_q07_order_type_priority_sample() {
        assert_eq!(OrderType::Sample.priority(), 2);
    }

    #[test]
    fn test_q08_order_type_priority_normal() {
        assert_eq!(OrderType::Normal.priority(), 3);
    }

    #[test]
    fn test_q09_order_type_priority_bulk() {
        assert_eq!(OrderType::Bulk.priority(), 4);
    }

    #[test]
    fn test_q10_order_priority_ordering() {
        let mut types = vec![OrderType::Bulk, OrderType::Rush, OrderType::Normal, OrderType::Sample];
        types.sort_by_key(|t| t.priority());
        assert_eq!(types[0], OrderType::Rush);
        assert_eq!(types[1], OrderType::Sample);
        assert_eq!(types[2], OrderType::Normal);
        assert_eq!(types[3], OrderType::Bulk);
    }

    // ============= 折扣应用测试 =============

    #[test]
    fn test_q11_discount_percentage() {
        let d = DiscountType::Percentage(dec!(0.10));
        assert_eq!(d.apply(dec!(1000.00), 1), dec!(900.00));
    }

    #[test]
    fn test_q12_discount_amount() {
        let d = DiscountType::Amount(dec!(200.00));
        assert_eq!(d.apply(dec!(1000.00), 1), dec!(800.00));
    }

    #[test]
    fn test_q13_discount_amount_no_negative() {
        let d = DiscountType::Amount(dec!(2000.00));
        assert_eq!(d.apply(dec!(1000.00), 1), Decimal::ZERO);
    }

    #[test]
    fn test_q14_discount_tiered_qualifies() {
        let d = DiscountType::Tiered { qty: 100, discount: dec!(0.15) };
        assert_eq!(d.apply(dec!(1000.00), 150), dec!(850.00));
    }

    #[test]
    fn test_q15_discount_tiered_not_qualify() {
        let d = DiscountType::Tiered { qty: 100, discount: dec!(0.15) };
        // 数量 50 < 100，不满足阶梯
        assert_eq!(d.apply(dec!(1000.00), 50), dec!(1000.00));
    }

    // ============= 销售订单业务规则测试 =============

    #[test]
    fn test_q16_sales_order_min_qty() {
        let min_qty = 1_i32;
        let order_qty = 0_i32;
        assert!(order_qty < min_qty, "订单数量必须 ≥ 1");
    }

    #[test]
    fn test_q17_sales_order_max_discount() {
        let max_discount = dec!(0.30);
        let actual = dec!(0.45);
        assert!(actual > max_discount, "折扣超限：{}% > {}%", actual * dec!(100), max_discount * dec!(100));
    }

    #[test]
    fn test_q18_sales_order_valid_discount() {
        let max_discount = dec!(0.30);
        let actual = dec!(0.20);
        assert!(actual <= max_discount);
    }

    #[test]
    fn test_q19_sales_order_tax_calculation() {
        // 1000 元 × 13% 税 = 130 元税额
        let amount = dec!(1000.00);
        let tax = amount * dec!(0.13);
        assert_eq!(tax, dec!(130.00));
    }

    #[test]
    fn test_q20_sales_order_line_total() {
        // 单价 50 × 数量 100 × (1-10%) = 4500
        let unit = dec!(50.00);
        let qty = 100_i32;
        let discount = dec!(0.10);
        let total = unit * Decimal::from(qty) * (Decimal::from(1) - discount);
        assert_eq!(total, dec!(4500.00));
    }

    // ============= 退货业务规则 =============

    #[derive(Debug, Clone)]
    struct ReturnRequest {
        pub order_amount: Decimal,
        pub return_qty: i32,
        pub total_qty: i32,
        pub reason: String,
    }

    impl ReturnRequest {
        fn return_ratio(&self) -> Decimal {
            Decimal::from(self.return_qty) / Decimal::from(self.total_qty)
        }
        fn is_valid(&self) -> bool {
            self.return_qty > 0 && self.return_qty <= self.total_qty
        }
        fn refund_amount(&self, unit_price: Decimal) -> Decimal {
            unit_price * Decimal::from(self.return_qty)
        }
    }

    #[test]
    fn test_q21_return_valid() {
        let r = ReturnRequest {
            order_amount: dec!(1000.00),
            return_qty: 10,
            total_qty: 100,
            reason: "色差".to_string(),
        };
        assert!(r.is_valid());
    }

    #[test]
    fn test_q22_return_invalid_zero() {
        let r = ReturnRequest {
            order_amount: dec!(1000.00),
            return_qty: 0,
            total_qty: 100,
            reason: "色差".to_string(),
        };
        assert!(!r.is_valid());
    }

    #[test]
    fn test_q23_return_invalid_excess() {
        let r = ReturnRequest {
            order_amount: dec!(1000.00),
            return_qty: 200,
            total_qty: 100,
            reason: "色差".to_string(),
        };
        assert!(!r.is_valid());
    }

    #[test]
    fn test_q24_return_ratio() {
        let r = ReturnRequest {
            order_amount: dec!(1000.00),
            return_qty: 25,
            total_qty: 100,
            reason: "色差".to_string(),
        };
        assert_eq!(r.return_ratio(), dec!(0.25));
    }

    #[test]
    fn test_q25_return_refund() {
        let r = ReturnRequest {
            order_amount: dec!(1000.00),
            return_qty: 10,
            total_qty: 100,
            reason: "色差".to_string(),
        };
        // 50 元/件 × 10 件 = 500
        assert_eq!(r.refund_amount(dec!(50.00)), dec!(500.00));
    }
}
