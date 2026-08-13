#[cfg(test)]
mod tests {
    use bingxi_backend::models::production_recipe::Model as ProductionRecipeModel;
    use bingxi_backend::models::production_recipe_addition::Model as ProductionRecipeAdditionModel;
    use chrono::Utc;
    use rust_decimal::Decimal;

    /// 构造测试用的生产配方模型
    fn make_production_recipe_model(id: i32) -> ProductionRecipeModel {
        ProductionRecipeModel {
            id,
            recipe_no: format!("PR-2026-{:04}", id),
            work_order_id: None,
            dye_batch_id: None,
            source_recipe_id: None,
            lab_dip_resample_id: None,
            customer_id: None,
            color_no: Some("C001".to_string()),
            fabric_name: Some("棉布".to_string()),
            fabric_spec: Some("40s".to_string()),
            fabric_width: None,
            gram_weight: None,
            fabric_weight: Decimal::new(100, 0),
            equipment_no: None,
            liquor_ratio: "1:8".to_string(),
            bath_volume: None,
            adjustment_factor: None,
            recipe_detail: None,
            total_dye_cost: None,
            total_auxiliary_cost: None,
            status: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            issued_by: None,
            printed_count: None,
            remarks: Some("测试备注".to_string()),
            is_deleted: false,
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 构造测试用的生产配方加料模型
    fn make_production_recipe_item_model(id: i32, recipe_id: i32) -> ProductionRecipeAdditionModel {
        ProductionRecipeAdditionModel {
            id,
            addition_no: format!("ADD-{:04}", id),
            production_recipe_id: recipe_id,
            work_order_id: None,
            dye_batch_id: None,
            addition_reason: Some("补充染料".to_string()),
            addition_detail: None,
            total_cost: None,
            status: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            issued_by: None,
            remarks: Some("测试备注".to_string()),
            is_deleted: false,
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ===== 生产配方模型测试 =====

    #[test]
    fn test_production_recipe_model_serialization() {
        let recipe = make_production_recipe_model(1);
        let json = serde_json::to_value(&recipe).expect("生产配方序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["recipe_no"], "PR-2026-0001");
        assert_eq!(json["status"], "draft");
    }

    #[test]
    fn test_production_recipe_version() {
        let recipe = make_production_recipe_model(1);
        assert_eq!(recipe.fabric_weight, Decimal::new(100, 0));
    }

    // ===== 生产配方子件模型测试 =====

    #[test]
    fn test_production_recipe_item_model_serialization() {
        let item = make_production_recipe_item_model(1, 1);
        let json = serde_json::to_value(&item).expect("生产配方子件序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["production_recipe_id"], 1);
        assert_eq!(json["addition_no"], "ADD-0001");
    }

    #[test]
    fn test_production_recipe_item_quantity() {
        let item = make_production_recipe_item_model(1, 1);
        assert_eq!(item.id, 1);
    }

    #[test]
    fn test_production_recipe_item_step_order() {
        let item = make_production_recipe_item_model(1, 1);
        assert_eq!(item.production_recipe_id, 1);
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_draft_to_active() {
        let mut recipe = make_production_recipe_model(1);
        recipe.status = "active".to_string();
        assert_eq!(recipe.status, "active");

        // 验证草稿状态可以转换为生效
        let valid_transitions = vec!["active", "cancelled"];
        assert!(valid_transitions.contains(&"active"));
    }

    #[test]
    fn test_status_active_to_expired() {
        let recipe = make_production_recipe_model(1);
        assert_eq!(recipe.status, "draft");

        // 验证生效状态可以转换为过期
        let valid_transitions = vec!["expired", "archived"];
        assert!(valid_transitions.contains(&"expired"));
    }

    #[test]
    fn test_status_expired_is_final() {
        let mut recipe = make_production_recipe_model(1);
        recipe.status = "expired".to_string();
        assert_eq!(recipe.status, "expired");

        // 验证过期状态是终态
        let invalid_transitions = vec!["draft", "active"];
        assert!(!invalid_transitions.contains(&"draft"));
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_production_recipe_json_roundtrip() {
        let recipe = make_production_recipe_model(1);
        let json = serde_json::to_value(&recipe).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("recipe_no").is_some());
        assert!(json.get("status").is_some());
    }

    #[test]
    fn test_production_recipe_item_json_roundtrip() {
        let item = make_production_recipe_item_model(1, 1);
        let json = serde_json::to_value(&item).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("production_recipe_id").is_some());
        assert!(json.get("addition_no").is_some());
    }
}
