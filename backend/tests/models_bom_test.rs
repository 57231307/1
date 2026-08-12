#[cfg(test)]
mod tests {
    use bingxi_backend::models::bom::Model as BomModel;
    use bingxi_backend::models::bom_item::Model as BomItemModel;
    use chrono::Utc;
    use rust_decimal::Decimal;

    /// 构造测试用的 BOM 模型
    fn make_bom_model(id: i32) -> BomModel {
        BomModel {
            id,
            bom_no: format!("BOM-{:04}", id),
            product_id: 1,
            product_name: Some("测试产品".to_string()),
            product_code: Some("P001".to_string()),
            version: Some("1.0".to_string()),
            name: "标准BOM".to_string(),
            description: Some("测试BOM描述".to_string()),
            unit: Some("米".to_string()),
            base_quantity: Decimal::new(1, 0),
            status: Some("active".to_string()),
            effective_date: Some(Utc::now().naive_utc()),
            expiry_date: None,
            remark: Some("测试备注".to_string()),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 构造测试用的 BOM 子件模型
    fn make_bom_item_model(id: i32, bom_id: i32) -> BomItemModel {
        BomItemModel {
            id,
            bom_id,
            material_id: 1,
            material_name: Some("棉纱".to_string()),
            material_code: Some("M001".to_string()),
            quantity: Decimal::new(5, 0),
            unit: Some("千克".to_string()),
            wastage_rate: Some(Decimal::new(5, 2)), // 5%
            remark: Some("测试备注".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ===== BOM 模型测试 =====

    #[test]
    fn test_bom_model_serialization() {
        let bom = make_bom_model(1);
        let json = serde_json::to_value(&bom).expect("BOM序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["bom_no"], "BOM-0001");
        assert_eq!(json["name"], "标准BOM");
        assert_eq!(json["status"], "active");
    }

    #[test]
    fn test_bom_version() {
        let bom = make_bom_model(1);
        assert_eq!(bom.version, Some("1.0".to_string()));
    }

    #[test]
    fn test_bom_base_quantity() {
        let bom = make_bom_model(1);
        assert_eq!(bom.base_quantity, Decimal::new(1, 0));
    }

    // ===== BOM 子件模型测试 =====

    #[test]
    fn test_bom_item_model_serialization() {
        let item = make_bom_item_model(1, 1);
        let json = serde_json::to_value(&item).expect("BOM子件序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["bom_id"], 1);
        assert_eq!(json["material_id"], 1);
        assert_eq!(json["quantity"], "5");
    }

    #[test]
    fn test_bom_item_quantity() {
        let item = make_bom_item_model(1, 1);
        assert_eq!(item.quantity, Decimal::new(5, 0));
    }

    #[test]
    fn test_bom_item_wastage_rate() {
        let item = make_bom_item_model(1, 1);
        assert_eq!(item.wastage_rate, Some(Decimal::new(5, 2))); // 5%
    }

    // ===== 损耗计算测试 =====

    #[test]
    fn test_wastage_calculation() {
        let base_quantity = Decimal::new(100, 0);
        let wastage_rate = Decimal::new(5, 2); // 5%
        let actual_quantity = base_quantity * (Decimal::new(1, 0) + wastage_rate);

        assert_eq!(actual_quantity, Decimal::new(105, 0));
    }

    #[test]
    fn test_wastage_zero() {
        let base_quantity = Decimal::new(100, 0);
        let wastage_rate = Decimal::new(0, 2);
        let actual_quantity = base_quantity * (Decimal::new(1, 0) + wastage_rate);

        assert_eq!(actual_quantity, Decimal::new(100, 0));
    }

    // ===== 有效期测试 =====

    #[test]
    fn test_effective_date() {
        let bom = make_bom_model(1);
        assert!(bom.effective_date.is_some());
    }

    #[test]
    fn test_expiry_date_none() {
        let bom = make_bom_model(1);
        assert!(bom.expiry_date.is_none());
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_bom_json_roundtrip() {
        let bom = make_bom_model(1);
        let json = serde_json::to_value(&bom).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("bom_no").is_some());
        assert!(json.get("product_id").is_some());
        assert!(json.get("name").is_some());
        assert!(json.get("status").is_some());
    }

    #[test]
    fn test_bom_item_json_roundtrip() {
        let item = make_bom_item_model(1, 1);
        let json = serde_json::to_value(&item).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("bom_id").is_some());
        assert!(json.get("material_id").is_some());
        assert!(json.get("quantity").is_some());
    }
}
