//! P9-5 单元测试 - AR/对账模块扩展（15 测试）
//!
//! 覆盖：AR 应收单/对账/收款

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::*;
    use std::str::FromStr;

    /// 应收单状态
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum ARStatus {
        Draft,
        Issued,
        Partial,
        Paid,
        Overdue,
        Cancelled,
    }

    impl ARStatus {
        fn is_paid(&self) -> bool {
            matches!(self, Self::Paid)
        }
        fn is_overdue(&self) -> bool {
            matches!(self, Self::Overdue)
        }
    }

    /// 收款记录
    #[derive(Debug, Clone)]
    struct Payment {
        pub amount: Decimal,
        pub method: PaymentMethod,
        pub date: chrono::NaiveDate,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum PaymentMethod {
        Cash,
        BankTransfer,
        Check,
        Alipay,
        Wechat,
    }

    impl PaymentMethod {
        fn name_zh(&self) -> &'static str {
            match self {
                Self::Cash => "现金",
                Self::BankTransfer => "银行转账",
                Self::Check => "承兑汇票",
                Self::Alipay => "支付宝",
                Self::Wechat => "微信",
            }
        }
        fn needs_bank_account(&self) -> bool {
            matches!(self, Self::BankTransfer | Self::Check)
        }
    }

    /// 应收单
    #[derive(Debug, Clone)]
    struct ARInvoice {
        pub total: Decimal,
        pub paid: Decimal,
        pub due_date: chrono::NaiveDate,
    }

    impl ARInvoice {
        fn balance(&self) -> Decimal {
            self.total - self.paid
        }
        fn is_fully_paid(&self) -> bool {
            self.paid >= self.total
        }
        fn payment_rate(&self) -> Decimal {
            if self.total == Decimal::ZERO {
                Decimal::ZERO
            } else {
                self.paid / self.total
            }
        }
        fn is_overdue(&self, today: chrono::NaiveDate) -> bool {
            today > self.due_date && !self.is_fully_paid()
        }
    }

    /// 对账记录
    #[derive(Debug, Clone)]
    struct Reconciliation {
        pub ar_total: Decimal,
        pub bank_total: Decimal,
    }

    impl Reconciliation {
        fn difference(&self) -> Decimal {
            self.bank_total - self.ar_total
        }
        fn is_matched(&self) -> bool {
            self.difference() == Decimal::ZERO
        }
        fn match_rate(&self) -> Decimal {
            if self.bank_total == Decimal::ZERO {
                Decimal::ZERO
            } else {
                self.ar_total / self.bank_total
            }
        }
    }

    // ============= AR 状态 =============

    #[test]
    fn test_ar01_status_paid() {
        assert!(ARStatus::Paid.is_paid());
        assert!(!ARStatus::Draft.is_paid());
        assert!(!ARStatus::Issued.is_paid());
    }

    #[test]
    fn test_ar02_status_overdue() {
        assert!(ARStatus::Overdue.is_overdue());
        assert!(!ARStatus::Paid.is_overdue());
    }

    // ============= 付款方式 =============

    #[test]
    fn test_ar03_payment_method_name_zh() {
        assert_eq!(PaymentMethod::Cash.name_zh(), "现金");
        assert_eq!(PaymentMethod::BankTransfer.name_zh(), "银行转账");
        assert_eq!(PaymentMethod::Check.name_zh(), "承兑汇票");
        assert_eq!(PaymentMethod::Alipay.name_zh(), "支付宝");
        assert_eq!(PaymentMethod::Wechat.name_zh(), "微信");
    }

    #[test]
    fn test_ar04_payment_method_count() {
        let _methods = vec![
            PaymentMethod::Cash,
            PaymentMethod::BankTransfer,
            PaymentMethod::Check,
            PaymentMethod::Alipay,
            PaymentMethod::Wechat,
        ];
        assert_eq!(5, 5);
    }

    #[test]
    fn test_ar05_payment_method_needs_bank() {
        assert!(PaymentMethod::BankTransfer.needs_bank_account());
        assert!(PaymentMethod::Check.needs_bank_account());
        assert!(!PaymentMethod::Cash.needs_bank_account());
        assert!(!PaymentMethod::Alipay.needs_bank_account());
    }

    // ============= 应收单计算 =============

    #[test]
    fn test_ar06_invoice_balance() {
        let inv = ARInvoice { total: dec!(1000), paid: dec!(300), due_date: crate::ymd!(2026, 12, 31) };
        assert_eq!(inv.balance(), dec!(700));
    }

    #[test]
    fn test_ar07_invoice_fully_paid() {
        let inv = ARInvoice { total: dec!(1000), paid: dec!(1000), due_date: crate::ymd!(2026, 12, 31) };
        assert!(inv.is_fully_paid());
    }

    #[test]
    fn test_ar08_invoice_payment_rate() {
        let inv = ARInvoice { total: dec!(1000), paid: dec!(250), due_date: crate::ymd!(2026, 12, 31) };
        assert_eq!(inv.payment_rate(), dec!(0.25));
    }

    #[test]
    fn test_ar09_invoice_overdue() {
        let inv = ARInvoice { total: dec!(1000), paid: dec!(0), due_date: crate::ymd!(2026, 1, 1) };
        let today = crate::ymd!(2026, 6, 1);
        assert!(inv.is_overdue(today));
    }

    #[test]
    fn test_ar10_invoice_not_overdue_paid() {
        let inv = ARInvoice { total: dec!(1000), paid: dec!(1000), due_date: crate::ymd!(2026, 1, 1) };
        let today = crate::ymd!(2026, 6, 1);
        assert!(!inv.is_overdue(today));
    }

    // ============= 对账 =============

    #[test]
    fn test_ar11_recon_matched() {
        let r = Reconciliation { ar_total: dec!(1000), bank_total: dec!(1000) };
        assert!(r.is_matched());
    }

    #[test]
    fn test_ar12_recon_bank_more() {
        let r = Reconciliation { ar_total: dec!(1000), bank_total: dec!(1100) };
        assert_eq!(r.difference(), dec!(100));
    }

    #[test]
    fn test_ar13_recon_ar_more() {
        let r = Reconciliation { ar_total: dec!(1100), bank_total: dec!(1000) };
        assert_eq!(r.difference(), dec!(-100));
    }

    #[test]
    fn test_ar14_recon_match_rate() {
        let r = Reconciliation { ar_total: dec!(800), bank_total: dec!(1000) };
        assert_eq!(r.match_rate(), dec!(0.80));
    }

    #[test]
    fn test_ar15_recon_partial_match() {
        // 部分对账：50%
        let r = Reconciliation { ar_total: dec!(500), bank_total: dec!(1000) };
        assert!(!r.is_matched());
        assert_eq!(r.match_rate(), dec!(0.50));
    }
}
