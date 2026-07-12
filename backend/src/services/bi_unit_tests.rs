//! P4-5 单元测试 - BI 数据分析服务（5 测试）

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::str::FromStr;
    // P9-1: 引入 decs! 宏统一测试夹具
    // 批次 343 v11 复审 P3 修复：移除 #[allow(unused_imports)]，decs! 宏已被广泛使用
    use crate::decs;

    /* 销售数据点 */
    #[derive(Debug, Clone)]
    struct SalesPoint {
        pub date: chrono::NaiveDate,
        pub amount: Decimal,
        pub orders: i64,
    }

    /* 同比增长率 */
    fn yoy_growth(current: Decimal, previous: Decimal) -> Decimal {
        if previous.is_zero() {
            Decimal::ZERO
        } else {
            ((current - previous) / previous * Decimal::from(100)).round_dp(2)
        }
    }

    /* 平均值 */
    fn average(values: &[Decimal]) -> Decimal {
        if values.is_empty() {
            return Decimal::ZERO;
        }
        let sum: Decimal = values.iter().sum();
        (sum / Decimal::from(values.len() as i64)).round_dp(2)
    }

    /* 中位数 */
    fn median(values: &mut Vec<Decimal>) -> Decimal {
        if values.is_empty() {
            return Decimal::ZERO;
        }
        values.sort();
        let n = values.len();
        if n % 2 == 1 {
            values[n / 2]
        } else {
            (values[n / 2 - 1] + values[n / 2]) / Decimal::from(2)
        }
    }

    /* 简单移动平均 */
    fn moving_average(points: &[Decimal], window: usize) -> Vec<Decimal> {
        if window == 0 || window > points.len() {
            return vec![];
        }
        (0..=(points.len() - window))
            .map(|i| {
                let sum: Decimal = points[i..i + window].iter().sum();
                (sum / Decimal::from(window as i64)).round_dp(2)
            })
            .collect()
    }

    /* 客户分层 (RFM 简化) */
    #[derive(Debug, PartialEq)]
    enum CustomerTier {
        Vip,
        Active,
        Normal,
        Churned,
    }

    fn classify_customer(recent_orders: i64, total_amount: Decimal) -> CustomerTier {
        if recent_orders >= 10 || total_amount >= Decimal::from(100000) {
            CustomerTier::Vip
        } else if recent_orders >= 3 {
            CustomerTier::Active
        } else if recent_orders >= 1 {
            CustomerTier::Normal
        } else {
            CustomerTier::Churned
        }
    }

    /* ===== 单元测试 ===== */

    #[test]
    fn 测试_同比增长率() {
        // 中文测试名：测试同比增长率
        // 今年 110 vs 去年 100 = 10%
        let g = yoy_growth(Decimal::from(110), Decimal::from(100));
        assert_eq!(g, decs!("10.00"));
        // 今年 50 vs 去年 100 = -50%
        let g2 = yoy_growth(Decimal::from(50), Decimal::from(100));
        assert_eq!(g2, decs!("-50.00"));
    }

    #[test]
    fn 测试_同比_零基期保护() {
        // 中文测试名：测试同比 - 零基期除零保护
        let g = yoy_growth(Decimal::from(100), Decimal::ZERO);
        assert_eq!(g, Decimal::ZERO);
    }

    #[test]
    fn 测试_平均值与中位数() {
        // 中文测试名：测试平均值与中位数计算
        let values = vec![
            Decimal::from(10),
            Decimal::from(20),
            Decimal::from(30),
            Decimal::from(40),
            Decimal::from(50),
        ];
        assert_eq!(average(&values), Decimal::from(30));
        let mut m_values = values.clone();
        assert_eq!(median(&mut m_values), Decimal::from(30));
        // 偶数个
        let mut m_even = vec![Decimal::from(1), Decimal::from(2), Decimal::from(3), Decimal::from(4)];
        assert_eq!(median(&mut m_even), decs!("2.5"));
    }

    #[test]
    fn 测试_移动平均() {
        // 中文测试名：测试简单移动平均（窗口 3）
        let points = vec![
            Decimal::from(10),
            Decimal::from(20),
            Decimal::from(30),
            Decimal::from(40),
            Decimal::from(50),
        ];
        let ma = moving_average(&points, 3);
        assert_eq!(ma.len(), 3);
        // (10+20+30)/3 = 20
        assert_eq!(ma[0], Decimal::from(20));
        // (20+30+40)/3 = 30
        assert_eq!(ma[1], Decimal::from(30));
        // (30+40+50)/3 = 40
        assert_eq!(ma[2], Decimal::from(40));
    }

    #[test]
    fn 测试_客户分层() {
        // 中文测试名：测试客户分层 RFM
        // VIP: 10 单
        assert_eq!(
            classify_customer(10, Decimal::from(50000)),
            CustomerTier::Vip
        );
        // VIP: 大额
        assert_eq!(
            classify_customer(1, Decimal::from(150000)),
            CustomerTier::Vip
        );
        // Active: 3 单
        assert_eq!(
            classify_customer(3, Decimal::from(1000)),
            CustomerTier::Active
        );
        // Normal: 1 单
        assert_eq!(
            classify_customer(1, Decimal::from(100)),
            CustomerTier::Normal
        );
        // Churned: 0 单
        assert_eq!(
            classify_customer(0, Decimal::ZERO),
            CustomerTier::Churned
        );
    }
}
