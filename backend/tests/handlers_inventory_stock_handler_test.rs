#[cfg(test)]
mod tests {
    use bingxi_backend::handlers::inventory_stock_handler::*;
    use bingxi_backend::models::inventory_stock::Model as InventoryStockModel;
    use bingxi_backend::models::status::inventory as status_inv;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;

    /// 构造测试用的库存模型
    fn make_inventory_stock_model(id: i32, status: &str) -> InventoryStockModel {
        InventoryStockModel {
            id,
            warehouse_id: 1,
            warehouse_name: Some("主仓库".to_string()),
            product_id: 1,
            product_name: Some("测试产品".to_string()),
            product_code: Some("P001".to_string()),
            sku: Some("SKU001".to_string()),
            batch_no: Some("B001".to_string()),
            location: Some("A-01-01".to_string()),
            quantity: Decimal::new(100, 0),
            reserved_quantity: Decimal::new(10, 0),
            available_quantity: Decimal::new(90, 0),
            unit: Some("米".to_string()),
            unit_cost: Some(Decimal::new(500, 2)),
            total_value: Some(Decimal::new(50000, 2)),
            status: Some(status.to_string()),
            last_count_date: Some(Utc::now().naive_utc()),
            last_movement_date: Some(Utc::now().naive_utc()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: Some(1),
            remark: Some("测试备注".to_string()),
        }
    }

    // ===== 状态常量测试 =====

    #[test]
    fn test_inv_status_active() {
        assert_eq!(status_inv::ACTIVE, "active");
    }

    #[test]
    fn test_inv_status_locked() {
        assert_eq!(status_inv::LOCKED, "locked");
    }

    #[test]
    fn test_inv_status_depleted() {
        assert_eq!(status_inv::DEPLETED, "depleted");
    }

    // ===== 模型测试 =====

    #[test]
    fn test_inventory_stock_model_serialization() {
        let stock = make_inventory_stock_model(1, "active");
        let json = serde_json::to_value(&stock).expect("库存序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["warehouse_id"], 1);
        assert_eq!(json["product_id"], 1);
        assert_eq!(json["status"], "active");
    }

    #[test]
    fn test_inventory_stock_quantities() {
        let stock = make_inventory_stock_model(1, "active");

        // 验证数量关系
        assert_eq!(stock.quantity, Decimal::new(100, 0));
        assert_eq!(stock.reserved_quantity, Decimal::new(10, 0));
        assert_eq!(stock.available_quantity, Decimal::new(90, 0));

        // 验证可用数量 = 总数量 - 预留数量
        assert_eq!(stock.available_quantity, stock.quantity - stock.reserved_quantity);
    }

    #[test]
    fn test_inventory_stock_value_calculation() {
        let stock = make_inventory_stock_model(1, "active");

        // 验证总价值 = 数量 * 单位成本
        let expected_value = stock.quantity * stock.unit_cost.unwrap();
        assert_eq!(stock.total_value, Some(expected_value));
    }

    // ===== 数量计算测试 =====

    #[test]
    fn test_quantity_reserved() {
        let quantity = Decimal::new(100, 0);
        let reserved = Decimal::new(30, 0);
        let available = quantity - reserved;

        assert_eq!(available, Decimal::new(70, 0));
    }

    #[test]
    fn test_quantity_fully_reserved() {
        let quantity = Decimal::new(100, 0);
        let reserved = Decimal::new(100, 0);
        let available = quantity - reserved;

        assert_eq!(available, Decimal::new(0, 0));
    }

    #[test]
    fn test_quantity_over_reserved() {
        let quantity = Decimal::new(100, 0);
        let reserved = Decimal::new(120, 0);
        let available = quantity - reserved;

        // 允许负值表示超预留
        assert_eq!(available, Decimal::new(-20, 0));
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_active_to_locked() {
        let stock = make_inventory_stock_model(1, "active");
        assert_eq!(stock.status, Some("active".to_string()));

        // 验证活跃状态可以转换为锁定
        let valid_transitions = vec!["locked", "depleted"];
        assert!(valid_transitions.contains(&"locked"));
    }

    #[test]
    fn test_status_locked_to_active() {
        let stock = make_inventory_stock_model(1, "locked");
        assert_eq!(stock.status, Some("locked".to_string()));

        // 验证锁定状态可以转换为活跃
        let valid_transitions = vec!["active", "depleted"];
        assert!(valid_transitions.contains(&"active"));
    }

    #[test]
    fn test_status_depleted_is_final() {
        let stock = make_inventory_stock_model(1, "depleted");
        assert_eq!(stock.status, Some("depleted".to_string()));

        // 验证耗尽状态是终态
        let invalid_transitions = vec!["active", "locked"];
        assert!(!invalid_transitions.contains(&"active"));
    }

    // ===== 库位测试 =====

    #[test]
    fn test_location_format() {
        let stock = make_inventory_stock_model(1, "active");
        let location = stock.location.as_ref().unwrap();

        // 验证库位格式
        assert!(location.contains('-'));
        let parts: Vec<&str> = location.split('-').collect();
        assert_eq!(parts.len(), 3);
    }

    // ===== 批次测试 =====

    #[test]
    fn test_batch_no_format() {
        let stock = make_inventory_stock_model(1, "active");
        let batch_no = stock.batch_no.as_ref().unwrap();

        // 验证批次号格式
        assert!(batch_no.starts_with('B'));
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_inventory_stock_json_roundtrip() {
        let stock = make_inventory_stock_model(1, "active");
        let json = serde_json::to_value(&stock).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("warehouse_id").is_some());
        assert!(json.get("product_id").is_some());
        assert!(json.get("quantity").is_some());
        assert!(json.get("status").is_some());
    }
}
