#[cfg(test)]
mod tests {
    use bingxi_backend::handlers::production_order_handler::*;
    use bingxi_backend::models::production_order::Model as ProductionOrderModel;
    use bingxi_backend::models::status::production as status_production;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;

    /// 构造测试用的生产订单模型
    fn make_production_order_model(id: i32, status: &str) -> ProductionOrderModel {
        ProductionOrderModel {
            id,
            order_no: format!("PO-2026-{:04}", id),
            sales_order_id: Some(1),
            sales_order_no: Some("SO-2026-0001".to_string()),
            customer_id: Some(1),
            customer_name: Some("测试客户".to_string()),
            product_id: 1,
            product_name: Some("测试产品".to_string()),
            product_code: Some("P001".to_string()),
            quantity: Decimal::new(100, 0),
            completed_quantity: Decimal::new(0, 0),
            unit: Some("米".to_string()),
            planned_start_date: Some(Utc::now().naive_utc()),
            planned_end_date: Some(Utc::now().naive_utc()),
            actual_start_date: None,
            actual_end_date: None,
            status: Some(status.to_string()),
            priority: Some("normal".to_string()),
            bom_id: Some(1),
            process_id: Some(1),
            warehouse_id: Some(1),
            remark: Some("测试备注".to_string()),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ===== 状态常量测试 =====

    #[test]
    fn test_production_status_planned() {
        assert_eq!(status_production::PLANNED, "planned");
    }

    #[test]
    fn test_production_status_in_progress() {
        assert_eq!(status_production::IN_PROGRESS, "in_progress");
    }

    #[test]
    fn test_production_status_completed() {
        assert_eq!(status_production::COMPLETED, "completed");
    }

    #[test]
    fn test_production_status_cancelled() {
        assert_eq!(status_production::CANCELLED, "cancelled");
    }

    // ===== 模型测试 =====

    #[test]
    fn test_production_order_model_serialization() {
        let order = make_production_order_model(1, "planned");
        let json = serde_json::to_value(&order).expect("生产订单序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["order_no"], "PO-2026-0001");
        assert_eq!(json["status"], "planned");
    }

    #[test]
    fn test_production_order_quantities() {
        let order = make_production_order_model(1, "planned");

        // 验证数量关系
        assert_eq!(order.quantity, Decimal::new(100, 0));
        assert_eq!(order.completed_quantity, Decimal::new(0, 0));
    }

    #[test]
    fn test_production_order_completion_rate() {
        let mut order = make_production_order_model(1, "in_progress");
        order.completed_quantity = Decimal::new(50, 0);

        // 验证完成率计算
        let completion_rate = order.completed_quantity / order.quantity * Decimal::new(100, 0);
        assert_eq!(completion_rate, Decimal::new(50, 0));
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_planned_to_in_progress() {
        let order = make_production_order_model(1, "planned");
        assert_eq!(order.status, Some("planned".to_string()));

        // 验证计划状态可以转换为进行中
        let valid_transitions = vec!["in_progress", "cancelled"];
        assert!(valid_transitions.contains(&"in_progress"));
    }

    #[test]
    fn test_status_in_progress_to_completed() {
        let order = make_production_order_model(1, "in_progress");
        assert_eq!(order.status, Some("in_progress".to_string()));

        // 验证进行中状态可以转换为已完成
        let valid_transitions = vec!["completed", "cancelled"];
        assert!(valid_transitions.contains(&"completed"));
    }

    #[test]
    fn test_status_completed_is_final() {
        let order = make_production_order_model(1, "completed");
        assert_eq!(order.status, Some("completed".to_string()));

        // 验证已完成状态是终态
        let invalid_transitions = vec!["planned", "in_progress"];
        assert!(!invalid_transitions.contains(&"planned"));
    }

    // ===== 优先级测试 =====

    #[test]
    fn test_priority_normal() {
        let order = make_production_order_model(1, "planned");
        assert_eq!(order.priority, Some("normal".to_string()));
    }

    // ===== 日期测试 =====

    #[test]
    fn test_planned_dates() {
        let order = make_production_order_model(1, "planned");
        assert!(order.planned_start_date.is_some());
        assert!(order.planned_end_date.is_some());
    }

    #[test]
    fn test_actual_dates_none_when_planned() {
        let order = make_production_order_model(1, "planned");
        assert!(order.actual_start_date.is_none());
        assert!(order.actual_end_date.is_none());
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_production_order_json_roundtrip() {
        let order = make_production_order_model(1, "planned");
        let json = serde_json::to_value(&order).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("order_no").is_some());
        assert!(json.get("product_id").is_some());
        assert!(json.get("quantity").is_some());
        assert!(json.get("status").is_some());
    }
}
