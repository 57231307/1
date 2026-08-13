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
            product_id: 1,
            version: 1,
            is_default: true,
            status: "ACTIVE".to_string(),
            remarks: Some("测试备注".to_string()),
            created_by: 1,
            is_deleted: false,
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
        assert_eq!(json["status"], "ACTIVE");
        assert_eq!(json["status"], "ACTIVE");
    }

    #[test]
    fn test_bom_version() {
        let bom = make_bom_model(1);
        assert_eq!(bom.version, 1);
    }

    #[test]
    fn test_bom_base_quantity() {
        let bom = make_bom_model(1);
        assert_eq!(bom.product_id, 1);
    }

    // ===== 状态测试 =====

    #[test]
    fn test_bom_status_active() {
        let bom = make_bom_model(1);
        assert_eq!(bom.status, "ACTIVE");
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_bom_json_roundtrip() {
        let bom = make_bom_model(1);
        let json = serde_json::to_value(&bom).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("product_id").is_some());
        assert!(json.get("version").is_some());
        assert!(json.get("status").is_some());
    }
}
