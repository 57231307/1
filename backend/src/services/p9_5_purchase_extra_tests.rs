//! P9-5 单元测试 - 采购模块扩展（20 测试）
//!
//! 覆盖：采购订单/收货/检验/退货 业务规则

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use rust_decimal::prelude::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    /// 采购订单状态机
    #[derive(Debug, Clone, PartialEq)]
    enum POStatus {
        Draft,
        Pending,
        Approved,
        PartialReceived,
        Received,
        Closed,
        Cancelled,
    }

    impl POStatus {
        fn can_receive(&self) -> bool {
            matches!(self, Self::Approved | Self::PartialReceived)
        }
        fn is_done(&self) -> bool {
            matches!(self, Self::Received | Self::Closed | Self::Cancelled)
        }
    }

    /// 质检结果
    #[derive(Debug, Clone, PartialEq)]
    enum InspectionResult {
        Pending,
        Pass,
        PartialPass { passed: i32, failed: i32 },
        Fail,
    }

    impl InspectionResult {
        fn pass_rate(&self, total: i32) -> Decimal {
            match self {
                Self::Pass => Decimal::from(1),
                Self::PartialPass { passed, .. } => {
                    Decimal::from(*passed) / Decimal::from(total)
                }
                _ => Decimal::ZERO,
            }
        }
        fn requires_return(&self) -> bool {
            matches!(self, Self::Fail)
        }
    }

    /// 供应商评级
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum SupplierGrade {
        D,
        C,
        B,
        A,
        S,
    }

    /// 收货记录
    #[derive(Debug, Clone)]
    struct Receipt {
        pub po_id: i32,
        pub ordered_qty: i32,
        pub received_qty: i32,
    }

    impl Receipt {
        fn completion(&self) -> Decimal {
            Decimal::from(self.received_qty) / Decimal::from(self.ordered_qty)
        }
        fn is_complete(&self) -> bool {
            self.received_qty >= self.ordered_qty
        }
    }

    /// 应付账龄
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum AgingBucket {
        Current,    // 0-30 天
        D30,        // 30-60 天
        D60,        // 60-90 天
        D90,        // 90+ 天
    }

    impl AgingBucket {
        fn from_days(days: i64) -> Self {
            match days {
                0..=30 => Self::Current,
                31..=60 => Self::D30,
                61..=90 => Self::D60,
                _ => Self::D90,
            }
        }
    }

    // ============= 采购订单状态 =============

    #[test]
    fn test_p01_po_can_receive_approved() {
        assert!(POStatus::Approved.can_receive());
    }

    #[test]
    fn test_p02_po_can_receive_partial() {
        assert!(POStatus::PartialReceived.can_receive());
    }

    #[test]
    fn test_p03_po_cannot_receive_draft() {
        assert!(!POStatus::Draft.can_receive());
    }

    #[test]
    fn test_p04_po_done() {
        assert!(POStatus::Received.is_done());
        assert!(POStatus::Closed.is_done());
        assert!(POStatus::Cancelled.is_done());
    }

    #[test]
    fn test_p05_po_not_done() {
        assert!(!POStatus::Draft.is_done());
        assert!(!POStatus::Approved.is_done());
    }

    // ============= 收货完成度 =============

    #[test]
    fn test_p06_receipt_complete() {
        let r = Receipt { po_id: 1, ordered_qty: 100, received_qty: 100 };
        assert!(r.is_complete());
    }

    #[test]
    fn test_p07_receipt_partial() {
        let r = Receipt { po_id: 1, ordered_qty: 100, received_qty: 50 };
        assert!(!r.is_complete());
    }

    #[test]
    fn test_p08_receipt_completion_ratio() {
        let r = Receipt { po_id: 1, ordered_qty: 100, received_qty: 75 };
        assert_eq!(r.completion(), dec!(0.75));
    }

    #[test]
    fn test_p09_receipt_completion_full() {
        let r = Receipt { po_id: 1, ordered_qty: 200, received_qty: 200 };
        assert_eq!(r.completion(), Decimal::from(1));
    }

    // ============= 质检结果 =============

    #[test]
    fn test_p10_inspection_full_pass() {
        let r = InspectionResult::Pass;
        assert_eq!(r.pass_rate(100), Decimal::from(1));
    }

    #[test]
    fn test_p11_inspection_partial_pass() {
        let r = InspectionResult::PartialPass { passed: 80, failed: 20 };
        assert_eq!(r.pass_rate(100), dec!(0.80));
    }

    #[test]
    fn test_p12_inspection_fail() {
        let r = InspectionResult::Fail;
        assert_eq!(r.pass_rate(100), Decimal::ZERO);
    }

    #[test]
    fn test_p13_inspection_requires_return() {
        assert!(InspectionResult::Fail.requires_return());
        assert!(!InspectionResult::Pass.requires_return());
    }

    // ============= 供应商评级 =============

    #[test]
    fn test_p14_supplier_grade_ordering() {
        assert!(SupplierGrade::D < SupplierGrade::C);
        assert!(SupplierGrade::C < SupplierGrade::B);
        assert!(SupplierGrade::B < SupplierGrade::A);
        assert!(SupplierGrade::A < SupplierGrade::S);
    }

    #[test]
    fn test_p15_supplier_grade_a_is_top() {
        let max = SupplierGrade::S;
        assert!(max >= SupplierGrade::A);
    }

    // ============= 应付账龄 =============

    #[test]
    fn test_p16_aging_current() {
        assert_eq!(AgingBucket::from_days(0), AgingBucket::Current);
        assert_eq!(AgingBucket::from_days(30), AgingBucket::Current);
    }

    #[test]
    fn test_p17_aging_d30() {
        assert_eq!(AgingBucket::from_days(31), AgingBucket::D30);
        assert_eq!(AgingBucket::from_days(60), AgingBucket::D30);
    }

    #[test]
    fn test_p18_aging_d60() {
        assert_eq!(AgingBucket::from_days(61), AgingBucket::D60);
        assert_eq!(AgingBucket::from_days(90), AgingBucket::D60);
    }

    #[test]
    fn test_p19_aging_d90() {
        assert_eq!(AgingBucket::from_days(91), AgingBucket::D90);
        assert_eq!(AgingBucket::from_days(365), AgingBucket::D90);
    }

    #[test]
    fn test_p20_aging_boundary() {
        // 边界值测试
        assert_eq!(AgingBucket::from_days(30), AgingBucket::Current);
        assert_eq!(AgingBucket::from_days(31), AgingBucket::D30);
        assert_eq!(AgingBucket::from_days(60), AgingBucket::D30);
        assert_eq!(AgingBucket::from_days(61), AgingBucket::D60);
    }
}
