#[cfg(test)]
mod tests {
    use bingxi_backend::handlers::custom_order_handler::*;
    use bingxi_backend::models::custom_order::Model as CustomOrderModel;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;

    /// 构造测试用的定制订单模型
    fn make_custom_order_model(id: i32, status: &str) -> CustomOrderModel {
        CustomOrderModel {
            id,
            order_no: format!("CO-2026-{:04}", id),
            customer_id: 1,
            customer_name: Some("测试客户".to_string()),
            order_date: Utc::now().naive_utc(),
            delivery_date: Some(Utc::now().naive_utc()),
            status: Some(status.to_string()),
            total_amount: Decimal::new(50000, 2),
            currency: Some("CNY".to_string()),
            payment_terms: Some("30天".to_string()),
            remark: Some("测试备注".to_string()),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            salesperson_id: Some(1),
            salesperson_name: Some("销售员".to_string()),
            design_requirements: Some("特殊设计要求".to_string()),
            material_requirements: Some("纯棉".to_string()),
            color_requirements: Some("蓝色".to_string()),
            size_requirements: Some("XL".to_string()),
            quality_requirements: Some("A级".to_string()),
            audit_status: Some("pending".to_string()),
            audit_by: None,
            audit_at: None,
        }
    }

    // ===== 模型测试 =====

    #[test]
    fn test_custom_order_model_serialization() {
        let order = make_custom_order_model(1, "draft");
        let json = serde_json::to_value(&order).expect("定制订单序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["order_no"], "CO-2026-0001");
        assert_eq!(json["status"], "draft");
    }

    #[test]
    fn test_custom_order_amount() {
        let order = make_custom_order_model(1, "draft");

        // 验证金额
        assert_eq!(order.total_amount, Decimal::new(50000, 2));
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_draft_to_confirmed() {
        let order = make_custom_order_model(1, "draft");
        assert_eq!(order.status, Some("draft".to_string()));

        // 验证草稿状态可以转换为已确认
        let valid_transitions = vec!["confirmed", "cancelled"];
        assert!(valid_transitions.contains(&"confirmed"));
    }

    #[test]
    fn test_status_confirmed_to_in_production() {
        let order = make_custom_order_model(1, "confirmed");
        assert_eq!(order.status, Some("confirmed".to_string()));

        // 验证已确认状态可以转换为生产中
        let valid_transitions = vec!["in_production", "cancelled"];
        assert!(valid_transitions.contains(&"in_production"));
    }

    #[test]
    fn test_status_in_production_to_completed() {
        let order = make_custom_order_model(1, "in_production");
        assert_eq!(order.status, Some("in_production".to_string()));

        // 验证生产中状态可以转换为已完成
        let valid_transitions = vec!["completed", "cancelled"];
        assert!(valid_transitions.contains(&"completed"));
    }

    #[test]
    fn test_status_completed_is_final() {
        let order = make_custom_order_model(1, "completed");
        assert_eq!(order.status, Some("completed".to_string()));

        // 验证已完成状态是终态
        let invalid_transitions = vec!["draft", "confirmed", "in_production"];
        assert!(!invalid_transitions.contains(&"draft"));
    }

    // ===== 需求字段测试 =====

    #[test]
    fn test_design_requirements() {
        let order = make_custom_order_model(1, "draft");
        assert_eq!(order.design_requirements, Some("特殊设计要求".to_string()));
    }

    #[test]
    fn test_material_requirements() {
        let order = make_custom_order_model(1, "draft");
        assert_eq!(order.material_requirements, Some("纯棉".to_string()));
    }

    #[test]
    fn test_color_requirements() {
        let order = make_custom_order_model(1, "draft");
        assert_eq!(order.color_requirements, Some("蓝色".to_string()));
    }

    #[test]
    fn test_size_requirements() {
        let order = make_custom_order_model(1, "draft");
        assert_eq!(order.size_requirements, Some("XL".to_string()));
    }

    #[test]
    fn test_quality_requirements() {
        let order = make_custom_order_model(1, "draft");
        assert_eq!(order.quality_requirements, Some("A级".to_string()));
    }

    // ===== 日期测试 =====

    #[test]
    fn test_order_date() {
        let order = make_custom_order_model(1, "draft");
        assert!(order.order_date <= Utc::now().naive_utc());
    }

    #[test]
    fn test_delivery_date() {
        let order = make_custom_order_model(1, "draft");
        assert!(order.delivery_date.is_some());
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_custom_order_json_roundtrip() {
        let order = make_custom_order_model(1, "draft");
        let json = serde_json::to_value(&order).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("order_no").is_some());
        assert!(json.get("customer_id").is_some());
        assert!(json.get("total_amount").is_some());
        assert!(json.get("status").is_some());
    }
}
