//! P4-5 单元测试 - Purchase 服务（5 测试）

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    // P9-1: 引入 decs! 宏统一测试夹具
    // 批次 343 v11 复审 P3 修复：移除 #[allow(unused_imports)]，decs! 宏已被广泛使用
    use crate::decs;

    /* 采购订单状态 */
    #[derive(Debug, Clone, PartialEq)]
    enum PurchaseStatus {
        Draft,
        Submitted,
        Approved,
        Closed,
    }

    /* 采购订单 */
    #[derive(Debug, Clone)]
    struct PurchaseOrder {
        pub _id: i64,
        pub supplier_id: i64,
        pub quantity: Decimal,
        pub unit_price: Decimal,
        pub received_qty: Decimal,
        pub _status: PurchaseStatus,
    }

    impl PurchaseOrder {
        fn total_amount(&self) -> Decimal {
            self.quantity * self.unit_price
        }

        fn received_rate(&self) -> Decimal {
            if self.quantity.is_zero() {
                Decimal::ZERO
            } else {
                (self.received_qty / self.quantity * Decimal::from(100)).round_dp(2)
            }
        }

        fn is_completable(&self) -> bool {
            !self.quantity.is_zero() && self.received_qty >= self.quantity
        }
    }

    /* 供应商评级 */
    #[derive(Debug, Clone)]
    struct SupplierRating {
        pub _supplier_id: i64,
        pub score: u8,
        pub _on_time_rate: Decimal,
    }

    impl SupplierRating {
        fn tier(&self) -> &'static str {
            match self.score {
                0..=49 => "D",
                50..=69 => "C",
                70..=84 => "B",
                85..=100 => "A",
                _ => "Unknown",
            }
        }
    }

    /* ===== 单元测试 ===== */

    #[test]
    fn test_cgddzje() {
        // 中文测试名：测试采购订单总金额
        let po = PurchaseOrder {
            _id: 1,
            supplier_id: 10,
            quantity: Decimal::from(50),
            unit_price: decs!("12.80"),
            received_qty: Decimal::ZERO,
            _status: PurchaseStatus::Submitted,
        };
        assert_eq!(po.total_amount(), decs!("640.00"));
    }

    #[test]
    fn test_shjdjs() {
        // 中文测试名：测试收货进度（received/qty*100%）
        let mut po = PurchaseOrder {
            _id: 1,
            supplier_id: 10,
            quantity: Decimal::from(100),
            unit_price: Decimal::from(5),
            received_qty: Decimal::from(30),
            _status: PurchaseStatus::Approved,
        };
        assert_eq!(po.received_rate(), decs!("30.00"));
        po.received_qty = Decimal::from(100);
        assert_eq!(po.received_rate(), decs!("100.00"));
        assert!(po.is_completable());
    }

    #[test]
    fn test_shjd_lslbh() {
        // 中文测试名：测试收货进度 - 零数量时除零保护
        let po = PurchaseOrder {
            _id: 1,
            supplier_id: 10,
            quantity: Decimal::ZERO,
            unit_price: Decimal::from(5),
            received_qty: Decimal::ZERO,
            _status: PurchaseStatus::Draft,
        };
        assert_eq!(po.received_rate(), Decimal::ZERO);
        assert!(!po.is_completable());
    }

    #[test]
    fn test_gyspjtier() {
        // 中文测试名：测试供应商评级 tier
        let r1 = SupplierRating {
            _supplier_id: 1,
            score: 90,
            _on_time_rate: decs!("0.95"),
        };
        assert_eq!(r1.tier(), "A");
        let r2 = SupplierRating {
            _supplier_id: 2,
            score: 75,
            _on_time_rate: decs!("0.80"),
        };
        assert_eq!(r2.tier(), "B");
        let r3 = SupplierRating {
            _supplier_id: 3,
            score: 60,
            _on_time_rate: decs!("0.60"),
        };
        assert_eq!(r3.tier(), "C");
        let r4 = SupplierRating {
            _supplier_id: 4,
            score: 30,
            _on_time_rate: decs!("0.30"),
        };
        assert_eq!(r4.tier(), "D");
    }

    #[test]
    fn test_agyshzcge() {
        // 中文测试名：测试按供应商汇总采购金额
        let mut by_supplier: HashMap<i64, Decimal> = HashMap::new();
        let orders = vec![
            PurchaseOrder {
                _id: 1,
                supplier_id: 10,
                quantity: Decimal::from(10),
                unit_price: Decimal::from(20),
                received_qty: Decimal::ZERO,
                _status: PurchaseStatus::Closed,
            },
            PurchaseOrder {
                _id: 2,
                supplier_id: 10,
                quantity: Decimal::from(5),
                unit_price: Decimal::from(30),
                received_qty: Decimal::ZERO,
                _status: PurchaseStatus::Closed,
            },
            PurchaseOrder {
                _id: 3,
                supplier_id: 20,
                quantity: Decimal::from(8),
                unit_price: Decimal::from(50),
                received_qty: Decimal::ZERO,
                _status: PurchaseStatus::Closed,
            },
        ];
        for o in &orders {
            *by_supplier.entry(o.supplier_id).or_insert(Decimal::ZERO) += o.total_amount();
        }
        assert_eq!(by_supplier.get(&10), Some(&Decimal::from(350)));
        assert_eq!(by_supplier.get(&20), Some(&Decimal::from(400)));
    }
}
