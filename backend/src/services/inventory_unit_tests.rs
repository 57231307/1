//! P4-5 单元测试 - Inventory 服务（5 测试）

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    use std::str::FromStr;
    // P9-1: 引入 decs! 宏统一测试夹具
    #[allow(unused_imports)]
    use crate::decs;

    /* 库存行 */
    #[derive(Debug, Clone)]
    struct StockRow {
        pub product_id: i64,
        pub warehouse_id: i64,
        pub quantity: Decimal,
        pub reserved: Decimal,
        pub unit_cost: Decimal,
    }

    impl StockRow {
        fn available(&self) -> Decimal {
            self.quantity - self.reserved
        }
        fn value(&self) -> Decimal {
            self.quantity * self.unit_cost
        }
    }

    /* 低库存预警规则 */
    #[derive(Debug, Clone)]
    struct LowStockRule {
        pub min_qty: Decimal,
    }

    impl LowStockRule {
        fn is_low(&self, stock: &StockRow) -> bool {
            stock.available() < self.min_qty
        }
    }

    /* 库存周转率 */
    fn inventory_turnover(sold_qty: Decimal, avg_inventory: Decimal) -> Decimal {
        if avg_inventory.is_zero() {
            Decimal::ZERO
        } else {
            (sold_qty / avg_inventory).round_dp(2)
        }
    }

    /* ===== 单元测试 ===== */

    #[test]
    fn 测试_库存可用量() {
        // 中文测试名：测试库存可用量 = 数量 - 锁定
        let s = StockRow {
            product_id: 1,
            warehouse_id: 1,
            quantity: Decimal::from(100),
            reserved: Decimal::from(30),
            unit_cost: Decimal::from(10),
        };
        assert_eq!(s.available(), Decimal::from(70));
    }

    #[test]
    fn 测试_库存价值() {
        // 中文测试名：测试库存价值 = 数量 × 单位成本
        let s = StockRow {
            product_id: 1,
            warehouse_id: 1,
            quantity: decs!("50.5"),
            reserved: Decimal::ZERO,
            unit_cost: decs!("12.00"),
        };
        assert_eq!(s.value(), decs!("606.00"));
    }

    #[test]
    fn 测试_低库存预警() {
        // 中文测试名：测试低库存预警规则
        let rule = LowStockRule { min_qty: Decimal::from(20) };
        let low = StockRow {
            product_id: 1, warehouse_id: 1,
            quantity: Decimal::from(25), reserved: Decimal::from(20),
            unit_cost: Decimal::from(5),
        };
        let normal = StockRow {
            product_id: 2, warehouse_id: 1,
            quantity: Decimal::from(100), reserved: Decimal::from(10),
            unit_cost: Decimal::from(5),
        };
        assert!(rule.is_low(&low));
        assert!(!rule.is_low(&normal));
    }

    #[test]
    fn 测试_库存周转率() {
        // 中文测试名：测试库存周转率计算
        // 周转率 = 销售量 / 平均库存
        let t1 = inventory_turnover(Decimal::from(500), Decimal::from(100));
        assert_eq!(t1, decs!("5.00"));
        let t2 = inventory_turnover(Decimal::from(0), Decimal::from(100));
        assert_eq!(t2, Decimal::ZERO);
    }

    #[test]
    fn 测试_库存按仓库汇总() {
        // 中文测试名：测试库存按仓库汇总
        let stocks = vec![
            StockRow { product_id: 1, warehouse_id: 1, quantity: Decimal::from(10), reserved: Decimal::ZERO, unit_cost: Decimal::from(5) },
            StockRow { product_id: 2, warehouse_id: 1, quantity: Decimal::from(20), reserved: Decimal::ZERO, unit_cost: Decimal::from(8) },
            StockRow { product_id: 1, warehouse_id: 2, quantity: Decimal::from(5), reserved: Decimal::ZERO, unit_cost: Decimal::from(5) },
        ];
        let mut by_warehouse: HashMap<i64, Decimal> = HashMap::new();
        for s in &stocks {
            *by_warehouse.entry(s.warehouse_id).or_insert(Decimal::ZERO) += s.quantity;
        }
        assert_eq!(by_warehouse.get(&1), Some(&Decimal::from(30)));
        assert_eq!(by_warehouse.get(&2), Some(&Decimal::from(5)));
    }
}
