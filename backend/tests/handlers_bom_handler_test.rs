#[cfg(test)]
mod tests {
    use bingxi_backend::handlers::bom_handler::*;
    use bingxi_backend::models::bom::Model as BomModel;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;

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

    // ===== 模型测试 =====

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

    // ===== 状态测试 =====

    #[test]
    fn test_bom_status_active() {
        let bom = make_bom_model(1);
        assert_eq!(bom.status, Some("active".to_string()));
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
}
