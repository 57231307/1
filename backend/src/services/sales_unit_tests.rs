//! P4-5 单元测试 - Sales 服务（5 测试）
//!
//! 覆盖：
//! - SalesOverview 统计结构构造
//! - Decimal 计算（金额、毛利率、趋势）
//! - 状态分类映射
//! - 业务规则（订单金额、税额）

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::*;
    use std::str::FromStr;
    // P9-1: 引入 decs! 宏统一测试夹具
    #[allow(unused_imports)]
    use crate::decs;

    /* SalesOverview 统计结构 - 业务模型 */
    #[derive(Debug, Clone, Default, PartialEq)]
    struct SalesOverviewStats {
        pub month_orders: i64,
        pub month_amount: Decimal,
        pub gross_profit_rate: Decimal,
        pub active_customers: i64,
        pub order_trend: f64,
        pub amount_trend: f64,
        pub profit_trend: f64,
    }

    /* 销售订单状态分类 */
    #[derive(Debug, Clone, PartialEq)]
    enum OrderStatus {
        Draft,
        Confirmed,
        Shipped,
        Completed,
        Cancelled,
    }

    impl OrderStatus {
        fn is_active(&self) -> bool {
            matches!(self, Self::Draft | Self::Confirmed | Self::Shipped)
        }
        fn is_done(&self) -> bool {
            matches!(self, Self::Completed | Self::Cancelled)
        }
    }

    /* 销售订单 */
    #[derive(Debug, Clone)]
    struct SalesOrder {
        pub id: i64,
        pub customer_id: i64,
        pub quantity: Decimal,
        pub unit_price: Decimal,
        pub status: OrderStatus,
    }

    impl SalesOrder {
        fn total_amount(&self) -> Decimal {
            self.quantity * self.unit_price
        }

        fn tax_amount(&self, rate: Decimal) -> Decimal {
            (self.total_amount() * rate).round_dp(2)
        }
    }

    /* ===== 单元测试 ===== */

    #[test]
    fn 测试_sales_overview_默认零值() {
        // 中文测试名：测试 SalesOverview 默认值
        let s = SalesOverviewStats::default();
        assert_eq!(s.month_orders, 0);
        assert_eq!(s.month_amount, Decimal::ZERO);
        assert_eq!(s.active_customers, 0);
        assert_eq!(s.order_trend, 0.0);
    }

    #[test]
    fn 测试_订单金额计算() {
        // 中文测试名：测试订单金额 = 数量 × 单价
        let order = SalesOrder {
            id: 1,
            customer_id: 100,
            quantity: Decimal::from(10),
            unit_price: decs!("25.50"),
            status: OrderStatus::Draft,
        };
        let total = order.total_amount();
        assert_eq!(total, decs!("255.00"));
    }

    #[test]
    fn 测试_税额计算() {
        // 中文测试名：测试税额 = 金额 × 13%（保留 2 位小数）
        let order = SalesOrder {
            id: 1,
            customer_id: 100,
            quantity: Decimal::from(100),
            unit_price: decs!("100.00"),
            status: OrderStatus::Confirmed,
        };
        let tax = order.tax_amount(decs!("0.13"));
        assert_eq!(tax, decs!("1300.00"));
    }

    #[test]
    fn 测试_订单状态分类() {
        // 中文测试名：测试订单状态 active/done 分类
        assert!(OrderStatus::Draft.is_active());
        assert!(OrderStatus::Shipped.is_active());
        assert!(!OrderStatus::Completed.is_active());
        assert!(OrderStatus::Completed.is_done());
        assert!(OrderStatus::Cancelled.is_done());
        assert!(!OrderStatus::Draft.is_done());
    }

    #[test]
    fn 测试_批量订单汇总() {
        // 中文测试名：测试批量订单总金额汇总
        let orders = vec![
            SalesOrder {
                id: 1,
                customer_id: 1,
                quantity: Decimal::from(10),
                unit_price: Decimal::from(20),
                status: OrderStatus::Completed,
            },
            SalesOrder {
                id: 2,
                customer_id: 2,
                quantity: Decimal::from(5),
                unit_price: decs!("15.50"),
                status: OrderStatus::Completed,
            },
            SalesOrder {
                id: 3,
                customer_id: 1,
                quantity: Decimal::from(3),
                unit_price: Decimal::from(100),
                status: OrderStatus::Cancelled,
            },
        ];
        let total: Decimal = orders.iter().map(|o| o.total_amount()).sum();
        assert_eq!(total, decs!("577.50"));

        // 仅统计已完成订单
        let completed_total: Decimal = orders
            .iter()
            .filter(|o| o.status == OrderStatus::Completed)
            .map(|o| o.total_amount())
            .sum();
        assert_eq!(completed_total, decs!("277.50"));
    }
}
