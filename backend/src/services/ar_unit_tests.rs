//! P4-5 单元测试 - AR（应收账款）服务（5 测试）

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /* 应收账款 */
    #[derive(Debug, Clone)]
    struct ArInvoice {
        pub id: i64,
        pub customer_id: i64,
        pub amount: Decimal,
        pub due_date: chrono::DateTime<Utc>,
        pub paid_amount: Decimal,
    }

    impl ArInvoice {
        fn outstanding(&self) -> Decimal {
            self.amount - self.paid_amount
        }
        fn is_overdue(&self) -> bool {
            Utc::now() > self.due_date && self.outstanding() > Decimal::ZERO
        }
        fn days_overdue(&self) -> i64 {
            if self.is_overdue() {
                (Utc::now() - self.due_date).num_days()
            } else {
                0
            }
        }
        fn aging_bucket(&self) -> &'static str {
            if !self.is_overdue() { return "current"; }
            let d = self.days_overdue();
            match d {
                0..=30 => "0-30",
                31..=60 => "31-60",
                61..=90 => "61-90",
                _ => "90+",
            }
        }
    }

    /* 客户信用额度 */
    #[derive(Debug, Clone)]
    struct CustomerCredit {
        pub customer_id: i64,
        pub limit: Decimal,
        pub used: Decimal,
    }

    impl CustomerCredit {
        fn available(&self) -> Decimal {
            self.limit - self.used
        }
        fn is_over_limit(&self) -> bool {
            self.used > self.limit
        }
    }

    /* ===== 单元测试 ===== */

    #[test]
    fn 测试_应收未付金额() {
        // 中文测试名：测试应收未付 = 总额 - 已付
        let inv = ArInvoice {
            id: 1,
            customer_id: 100,
            amount: Decimal::from(1000),
            due_date: Utc::now() + Duration::days(30),
            paid_amount: Decimal::from(300),
        };
        assert_eq!(inv.outstanding(), Decimal::from(700));
    }

    #[test]
    fn 测试_到期判断() {
        // 中文测试名：测试应收到期判断
        let overdue = ArInvoice {
            id: 1,
            customer_id: 100,
            amount: Decimal::from(1000),
            due_date: Utc::now() - Duration::days(10),
            paid_amount: Decimal::ZERO,
        };
        assert!(overdue.is_overdue());
        assert_eq!(overdue.days_overdue(), 10);

        let not_due = ArInvoice {
            id: 2,
            customer_id: 100,
            amount: Decimal::from(1000),
            due_date: Utc::now() + Duration::days(10),
            paid_amount: Decimal::ZERO,
        };
        assert!(!not_due.is_overdue());
    }

    #[test]
    fn 测试_账龄分桶() {
        // 中文测试名：测试账龄分桶（0-30 / 31-60 / 61-90 / 90+）
        let cases: Vec<(i64, &str)> = vec![
            (-1, "current"),
            (0, "0-30"),
            (15, "0-30"),
            (30, "0-30"),
            (45, "31-60"),
            (60, "31-60"),
            (75, "61-90"),
            (100, "90+"),
        ];
        for (days, expected) in cases {
            let inv = ArInvoice {
                id: 1,
                customer_id: 100,
                amount: Decimal::from(1000),
                due_date: Utc::now() - Duration::days(days),
                paid_amount: Decimal::ZERO,
            };
            assert_eq!(inv.aging_bucket(), expected, "days={}", days);
        }
    }

    #[test]
    fn 测试_信用额度可用() {
        // 中文测试名：测试信用额度可用余额
        let c = CustomerCredit {
            customer_id: 100,
            limit: Decimal::from(10000),
            used: Decimal::from(3000),
        };
        assert_eq!(c.available(), Decimal::from(7000));
        assert!(!c.is_over_limit());

        let over = CustomerCredit {
            customer_id: 200,
            limit: Decimal::from(5000),
            used: Decimal::from_str("6000.00").unwrap(),
        };
        assert!(over.is_over_limit());
    }

    #[test]
    fn 测试_已付清后不算逾期() {
        // 中文测试名：测试已付清的应收不算逾期
        let paid = ArInvoice {
            id: 1,
            customer_id: 100,
            amount: Decimal::from(1000),
            due_date: Utc::now() - Duration::days(100),
            paid_amount: Decimal::from(1000),
        };
        assert!(!paid.is_overdue());
        assert_eq!(paid.outstanding(), Decimal::ZERO);
        assert_eq!(paid.aging_bucket(), "current");
    }
}
