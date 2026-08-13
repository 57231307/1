#[cfg(test)]
mod tests {
    use bingxi_backend::handlers::supplier_handler::*;
    use bingxi_backend::models::supplier::Model as SupplierModel;
    use chrono::Utc;
    use serde_json::json;

    /// 构造测试用的供应商模型
    fn make_supplier_model(id: i32) -> SupplierModel {
        SupplierModel {
            id,
            supplier_no: format!("S-2026-{:04}", id),
            name: "测试供应商".to_string(),
            short_name: Some("测试".to_string()),
            english_name: Some("Test Supplier".to_string()),
            supplier_type: Some("manufacturer".to_string()),
            industry: Some("纺织".to_string()),
            region: Some("中国".to_string()),
            province: Some("浙江".to_string()),
            city: Some("杭州".to_string()),
            address: Some("测试地址".to_string()),
            contact_person: Some("李四".to_string()),
            contact_phone: Some("13900139000".to_string()),
            contact_email: Some("supplier@example.com".to_string()),
            tax_no: Some("91330100MA27K5XH0J".to_string()),
            bank_name: Some("工商银行".to_string()),
            bank_account: Some("0987654321".to_string()),
            payment_terms: Some("30天".to_string()),
            lead_time: Some(7),
            rating: Some("A".to_string()),
            status: Some("active".to_string()),
            notes: Some("测试备注".to_string()),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ===== 模型测试 =====

    #[test]
    fn test_supplier_model_serialization() {
        let supplier = make_supplier_model(1);
        let json = serde_json::to_value(&supplier).expect("供应商序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["supplier_no"], "S-2026-0001");
        assert_eq!(json["name"], "测试供应商");
        assert_eq!(json["status"], "active");
    }

    #[test]
    fn test_supplier_contact_info() {
        let supplier = make_supplier_model(1);

        // 验证联系信息
        assert_eq!(supplier.contact_person, Some("李四".to_string()));
        assert_eq!(supplier.contact_phone, Some("13900139000".to_string()));
        assert_eq!(supplier.contact_email, Some("supplier@example.com".to_string()));
    }

    // ===== 供应商类型测试 =====

    #[test]
    fn test_supplier_type_manufacturer() {
        let supplier = make_supplier_model(1);
        assert_eq!(supplier.supplier_type, Some("manufacturer".to_string()));
    }

    // ===== 评级测试 =====

    #[test]
    fn test_supplier_rating_a() {
        let supplier = make_supplier_model(1);
        assert_eq!(supplier.rating, Some("A".to_string()));
    }

    // ===== 交期测试 =====

    #[test]
    fn test_supplier_lead_time() {
        let supplier = make_supplier_model(1);
        assert_eq!(supplier.lead_time, Some(7));
    }

    // ===== 状态测试 =====

    #[test]
    fn test_supplier_status_active() {
        let supplier = make_supplier_model(1);
        assert_eq!(supplier.status, Some("active".to_string()));
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_supplier_json_roundtrip() {
        let supplier = make_supplier_model(1);
        let json = serde_json::to_value(&supplier).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("supplier_no").is_some());
        assert!(json.get("name").is_some());
        assert!(json.get("status").is_some());
    }

    #[test]
    fn test_supplier_json_contact_fields() {
        let supplier = make_supplier_model(1);
        let json = serde_json::to_value(&supplier).expect("序列化失败");

        // 验证联系信息字段
        assert!(json.get("contact_person").is_some());
        assert!(json.get("contact_phone").is_some());
        assert!(json.get("contact_email").is_some());
    }
}
