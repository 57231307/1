#[cfg(test)]
mod tests {
    use bingxi_backend::decs;
    use chrono::{Duration, Utc};
    use rust_decimal::Decimal;

    /// 构造测试用库存 Model（默认"正常"状态）
    fn make_stock_model() -> inventory_stock::Model {
        inventory_stock::Model {
            id: 1,
            warehouse_id: 1,
            product_id: 1,
            quantity_on_hand: decs!("100"),
            quantity_available: decs!("100"),
            quantity_reserved: decs!("0"),
            quantity_shipped: decs!("0"),
            quantity_incoming: decs!("0"),
            reorder_point: decs!("0"),
            max_stock_point: decs!("0"),
            reorder_quantity: decs!("0"),
            bin_location: None,
            last_count_date: None,
            last_movement_date: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            batch_no: "B001".to_string(),
            color_no: "C001".to_string(),
            dye_lot_no: Some("DL001".to_string()),
            grade: "一等品".to_string(),
            production_date: None,
            expiry_date: None,
            quantity_meters: decs!("100"),
            quantity_kg: decs!("10"),
            gram_weight: Some(decs!("200")),
            width: Some(decs!("150")),
            location_id: Some(1),
            shelf_no: Some("A01".to_string()),
            layer_no: Some("1".to_string()),
            stock_status: "正常".to_string(),
            quality_status: "合格".to_string(),
            version: 0,
            replenishment_strategy: "reorder_point".to_string(),
        }
    }

    // ========== 优先级 1: discrepancy（状态异常）==========

    #[test]
    fn test_kcgj_ztycfhdiscrepancy() {
        let mut stock = make_stock_model();
        stock.stock_status = "冻结".to_string();
        assert_eq!(compute_alert_type(&stock), AlertType::Discrepancy.code());
    }

    #[test]
    fn test_kcgj_ztycyxjzg_fgqh() {
        // 状态异常 + 缺货条件同时满足，应返回 discrepancy（优先级 1 > 2）
        let mut stock = make_stock_model();
        stock.stock_status = "待检".to_string();
        stock.reorder_point = decs!("50");
        stock.quantity_available = Decimal::ZERO;
        assert_eq!(compute_alert_type(&stock), AlertType::Discrepancy.code());
    }

    // ========== 优先级 2: out_of_stock（缺货）==========

    #[test]
    fn test_kcgj_qhfhout_of_stock() {
        let mut stock = make_stock_model();
        stock.reorder_point = decs!("50");
        stock.quantity_available = Decimal::ZERO;
        assert_eq!(compute_alert_type(&stock), AlertType::OutOfStock.code());
    }

    #[test]
    fn test_kcgj_qhwszbhdbcfgj() {
        // reorder_point == 0 时不判定缺货（未设置补货策略）
        let mut stock = make_stock_model();
        stock.reorder_point = Decimal::ZERO;
        stock.quantity_available = Decimal::ZERO;
        assert_eq!(compute_alert_type(&stock), ALERT_TYPE_NORMAL);
    }

    // ========== 优先级 3: low_stock（低于下限）==========

    #[test]
    fn test_kcgj_dyxxfhlow_stock() {
        let mut stock = make_stock_model();
        stock.reorder_point = decs!("50");
        stock.quantity_available = decs!("30");
        assert_eq!(compute_alert_type(&stock), AlertType::LowStock.code());
    }

    // ========== 优先级 4: over_stock（高于上限）==========

    #[test]
    fn test_kcgj_gysxfhover_stock() {
        let mut stock = make_stock_model();
        stock.max_stock_point = decs!("200");
        stock.quantity_available = decs!("250");
        assert_eq!(compute_alert_type(&stock), AlertType::OverStock.code());
    }

    #[test]
    fn test_kcgj_gysxwszsxbcfgj() {
        // max_stock_point == 0 时不判定超储
        let mut stock = make_stock_model();
        stock.max_stock_point = Decimal::ZERO;
        stock.quantity_available = decs!("9999");
        assert_eq!(compute_alert_type(&stock), ALERT_TYPE_NORMAL);
    }

    // ========== 优先级 5: expiring（即将过期）==========

    #[test]
    fn test_kcgj_jjgqfhexpiring() {
        let mut stock = make_stock_model();
        // 过期日期距今 10 天（≤ 30 天阈值）
        stock.expiry_date = Some(Utc::now() + Duration::days(10));
        assert_eq!(compute_alert_type(&stock), AlertType::Expiring.code());
    }

    #[test]
    fn test_kcgj_gqrqqdyyzfhexpiring() {
        let mut stock = make_stock_model();
        // 过期日期距今 30 天（== EXPIRING_THRESHOLD_DAYS，边界 ≤ 判定）
        stock.expiry_date = Some(Utc::now() + Duration::days(30));
        assert_eq!(compute_alert_type(&stock), AlertType::Expiring.code());
    }

    // ========== 优先级 6: slow_moving（滞销）==========

    #[test]
    fn test_kcgj_zxfhslow_moving() {
        let mut stock = make_stock_model();
        // 最后变动日期距今 100 天（> 90 天阈值）
        stock.last_movement_date = Some(Utc::now() - Duration::days(100));
        assert_eq!(compute_alert_type(&stock), AlertType::SlowMoving.code());
    }

    #[test]
    fn test_kcgj_cwbdqykcfhslow_moving() {
        let mut stock = make_stock_model();
        // last_movement_date 为 None 且有库存 → 视为滞销
        stock.last_movement_date = None;
        stock.quantity_available = decs!("50");
        assert_eq!(compute_alert_type(&stock), AlertType::SlowMoving.code());
    }

    #[test]
    fn test_kcgj_cwbdqwkcbfhslow_moving() {
        // last_movement_date 为 None 但无库存 → 不判定滞销（无库存无所谓滞销）
        let mut stock = make_stock_model();
        stock.last_movement_date = None;
        stock.quantity_available = Decimal::ZERO;
        stock.reorder_point = Decimal::ZERO; // 避免触发 out_of_stock
        assert_eq!(compute_alert_type(&stock), ALERT_TYPE_NORMAL);
    }

    // ========== 优先级 7: normal（正常）==========

    #[test]
    fn test_kcgj_zckcfhnormal() {
        let stock = make_stock_model();
        assert_eq!(compute_alert_type(&stock), ALERT_TYPE_NORMAL);
    }

    // ========== 优先级链路覆盖测试 ==========

    #[test]
    fn test_kcgj_yxjll_qhgydyxx() {
        // quantity_available == 0 满足缺货条件，同时 < reorder_point 满足低于下限
        // 缺货（优先级 2）应先于低于下限（优先级 3）返回
        let mut stock = make_stock_model();
        stock.reorder_point = decs!("50");
        stock.quantity_available = Decimal::ZERO;
        assert_eq!(compute_alert_type(&stock), AlertType::OutOfStock.code());
    }

    #[test]
    fn test_kcgj_yxjll_dyxxgygysx() {
        // 同时满足低于下限和高于上限不可能（quantity_available < reorder_point 且 > max_stock_point）
        // 此测试验证逻辑不冲突：reorder_point=50, max_stock_point=200, quantity=30 → low_stock
        let mut stock = make_stock_model();
        stock.reorder_point = decs!("50");
        stock.max_stock_point = decs!("200");
        stock.quantity_available = decs!("30");
        assert_eq!(compute_alert_type(&stock), AlertType::LowStock.code());
    }
}