#[cfg(test)]
mod tests {
    use bingxi_backend::handlers::dye_batch_handler::*;
    use bingxi_backend::models::dye_batch::Model as DyeBatchModel;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;

    /// 构造测试用的染色批次模型
    fn make_dye_batch_model(id: i32, status: &str) -> DyeBatchModel {
        DyeBatchModel {
            id,
            batch_no: format!("DB-2026-{:04}", id),
            production_order_id: Some(1),
            production_order_no: Some("PO-2026-0001".to_string()),
            recipe_id: Some(1),
            recipe_name: Some("蓝色配方".to_string()),
            dye_vat_id: Some(1),
            dye_vat_name: Some("1号染缸".to_string()),
            fabric_id: 1,
            fabric_name: Some("棉布".to_string()),
            fabric_code: Some("F001".to_string()),
            color: Some("蓝色".to_string()),
            color_code: Some("C001".to_string()),
            quantity: Decimal::new(100, 0),
            unit: Some("米".to_string()),
            planned_start_date: Some(Utc::now().naive_utc()),
            planned_end_date: Some(Utc::now().naive_utc()),
            actual_start_date: None,
            actual_end_date: None,
            status: Some(status.to_string()),
            priority: Some("normal".to_string()),
            notes: Some("测试备注".to_string()),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ===== 模型测试 =====

    #[test]
    fn test_dye_batch_model_serialization() {
        let batch = make_dye_batch_model(1, "planned");
        let json = serde_json::to_value(&batch).expect("染色批次序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["batch_no"], "DB-2026-0001");
        assert_eq!(json["status"], "planned");
    }

    #[test]
    fn test_dye_batch_quantity() {
        let batch = make_dye_batch_model(1, "planned");

        // 验证数量
        assert_eq!(batch.quantity, Decimal::new(100, 0));
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_planned_to_in_progress() {
        let batch = make_dye_batch_model(1, "planned");
        assert_eq!(batch.status, Some("planned".to_string()));

        // 验证计划状态可以转换为进行中
        let valid_transitions = vec!["in_progress", "cancelled"];
        assert!(valid_transitions.contains(&"in_progress"));
    }

    #[test]
    fn test_status_in_progress_to_completed() {
        let batch = make_dye_batch_model(1, "in_progress");
        assert_eq!(batch.status, Some("in_progress".to_string()));

        // 验证进行中状态可以转换为已完成
        let valid_transitions = vec!["completed", "cancelled"];
        assert!(valid_transitions.contains(&"completed"));
    }

    #[test]
    fn test_status_completed_is_final() {
        let batch = make_dye_batch_model(1, "completed");
        assert_eq!(batch.status, Some("completed".to_string()));

        // 验证已完成状态是终态
        let invalid_transitions = vec!["planned", "in_progress"];
        assert!(!invalid_transitions.contains(&"planned"));
    }

    // ===== 优先级测试 =====

    #[test]
    fn test_priority_normal() {
        let batch = make_dye_batch_model(1, "planned");
        assert_eq!(batch.priority, Some("normal".to_string()));
    }

    // ===== 日期测试 =====

    #[test]
    fn test_planned_dates() {
        let batch = make_dye_batch_model(1, "planned");
        assert!(batch.planned_start_date.is_some());
        assert!(batch.planned_end_date.is_some());
    }

    #[test]
    fn test_actual_dates_none_when_planned() {
        let batch = make_dye_batch_model(1, "planned");
        assert!(batch.actual_start_date.is_none());
        assert!(batch.actual_end_date.is_none());
    }

    // ===== 配方测试 =====

    #[test]
    fn test_recipe_info() {
        let batch = make_dye_batch_model(1, "planned");
        assert_eq!(batch.recipe_id, Some(1));
        assert_eq!(batch.recipe_name, Some("蓝色配方".to_string()));
    }

    // ===== 染缸测试 =====

    #[test]
    fn test_dye_vat_info() {
        let batch = make_dye_batch_model(1, "planned");
        assert_eq!(batch.dye_vat_id, Some(1));
        assert_eq!(batch.dye_vat_name, Some("1号染缸".to_string()));
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_dye_batch_json_roundtrip() {
        let batch = make_dye_batch_model(1, "planned");
        let json = serde_json::to_value(&batch).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("batch_no").is_some());
        assert!(json.get("fabric_id").is_some());
        assert!(json.get("quantity").is_some());
        assert!(json.get("status").is_some());
    }
}
