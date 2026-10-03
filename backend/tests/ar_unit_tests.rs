//! P4-5 单元测试 - AR（应收账款）服务（5 测试）

use bingxi_backend::services::ar::ArReconciliationService;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

/* 应收账款 */
#[derive(Debug, Clone)]
struct ArInvoice {
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
}

/* 客户信用额度 */
#[derive(Debug, Clone)]
struct CustomerCredit {
    #[allow(dead_code)]
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
fn test_yswfje() {
    // 中文测试名：测试应收未付 = 总额 - 已付
    let inv = ArInvoice {
        amount: Decimal::from(1000),
        due_date: Utc::now() + Duration::days(30),
        paid_amount: Decimal::from(300),
    };
    assert_eq!(inv.outstanding(), Decimal::from(700));
}

#[test]
fn test_dqpd() {
    // 中文测试名：测试应收到期判断
    let overdue = ArInvoice {
        amount: Decimal::from(1000),
        due_date: Utc::now() - Duration::days(10),
        paid_amount: Decimal::ZERO,
    };
    assert!(overdue.is_overdue());
    assert_eq!(overdue.days_overdue(), 10);

    let not_due = ArInvoice {
        amount: Decimal::from(1000),
        due_date: Utc::now() + Duration::days(10),
        paid_amount: Decimal::ZERO,
    };
    assert!(!not_due.is_overdue());
}

#[test]
fn test_zlft() {
    // 中文测试名：测试账龄分桶（改调生产纯函数 compute_aging_bucket_index，
    // vfy_ops/aging.rs，桶序 0=当期 / 1=1-30天 / 2=31-60天 / 3=61-90天 / 4=90天以上）。
    // 注意：本用例原影子实现把 0 天逾期归入 "0-30" 桶，与生产规则不一致
    // （生产：overdue_days <= 0 归第 0 桶"当期"），以生产为准。
    let cases: Vec<(i64, usize)> = vec![
        (-1, 0),
        (0, 0),
        (1, 1),
        (15, 1),
        (30, 1),
        (31, 2),
        (45, 2),
        (60, 2),
        (61, 3),
        (75, 3),
        (90, 3),
        (91, 4),
        (100, 4),
        (365, 4),
    ];
    for (days, expected_idx) in cases {
        assert_eq!(
            ArReconciliationService::compute_aging_bucket_index(days),
            expected_idx,
            "days={}"
        );
    }
}

#[test]
fn test_xyedky() {
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
fn test_yfqhbsyq() {
    // 中文测试名：测试已付清的应收不算逾期
    let paid = ArInvoice {
        amount: Decimal::from(1000),
        due_date: Utc::now() - Duration::days(100),
        paid_amount: Decimal::from(1000),
    };
    assert!(!paid.is_overdue());
    assert_eq!(paid.outstanding(), Decimal::ZERO);
}
