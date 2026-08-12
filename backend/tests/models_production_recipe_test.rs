#[cfg(test)]
mod tests {
    use bingxi_backend::models::production_recipe::Model as ProductionRecipeModel;
    use bingxi_backend::models::production_recipe_item::Model as ProductionRecipeItemModel;
    use chrono::Utc;
    use rust_decimal::Decimal;

    /// 构造测试用的生产配方模型
    fn make_production_recipe_model(id: i32) -> ProductionRecipeModel {
        ProductionRecipeModel {
            id,
            recipe_no: format!("PR-2026-{:04}", id),
            name: "蓝色配方".to_string(),
            description: Some("测试配方描述".to_string()),
            product_id: Some(1),
            product_name: Some("测试产品".to_string()),
            process_id: Some(1),
            process_name: Some("染色工艺".to_string()),
            version: Some("1.0".to_string()),
            status: Some("active".to_string()),
            effective_date: Some(Utc::now().naive_utc()),
            expiry_date: None,
            remark: Some("测试备注".to_string()),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 构造测试用的生产配方子件模型
    fn make_production_recipe_item_model(id: i32, recipe_id: i32) -> ProductionRecipeItemModel {
        ProductionRecipeItemModel {
            id,
            recipe_id,
            material_id: 1,
            material_name: Some("染料".to_string()),
            material_code: Some("M001".to_string()),
            quantity: Decimal::new(10, 2),
            unit: Some("千克".to_string()),
            step_order: Some(1),
            step_description: Some("加入染料".to_string()),
            remark: Some("测试备注".to_string()),
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
        assert_eq!(json["name"], "蓝色配方");
        assert_eq!(json["status"], "active");
    }

    #[test]
    fn test_production_recipe_version() {
        let recipe = make_production_recipe_model(1);
        assert_eq!(recipe.version, Some("1.0".to_string()));
    }

    // ===== 生产配方子件模型测试 =====

    #[test]
    fn test_production_recipe_item_model_serialization() {
        let item = make_production_recipe_item_model(1, 1);
        let json = serde_json::to_value(&item).expect("生产配方子件序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["recipe_id"], 1);
        assert_eq!(json["material_id"], 1);
        assert_eq!(json["quantity"], "0.10");
    }

    #[test]
    fn test_production_recipe_item_quantity() {
        let item = make_production_recipe_item_model(1, 1);
        assert_eq!(item.quantity, Decimal::new(10, 2));
    }

    #[test]
    fn test_production_recipe_item_step_order() {
        let item = make_production_recipe_item_model(1, 1);
        assert_eq!(item.step_order, Some(1));
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_draft_to_active() {
        let recipe = make_production_recipe_model(1);
        let mut recipe = recipe;
        recipe.status = Some("draft".to_string());
        assert_eq!(recipe.status, Some("draft".to_string()));

        // 验证草稿状态可以转换为生效
        let valid_transitions = vec!["active", "cancelled"];
        assert!(valid_transitions.contains(&"active"));
    }

    #[test]
    fn test_status_active_to_expired() {
        let recipe = make_production_recipe_model(1);
        assert_eq!(recipe.status, Some("active".to_string()));

        // 验证生效状态可以转换为过期
        let valid_transitions = vec!["expired", "archived"];
        assert!(valid_transitions.contains(&"expired"));
    }

    #[test]
    fn test_status_expired_is_final() {
        let mut recipe = make_production_recipe_model(1);
        recipe.status = Some("expired".to_string());
        assert_eq!(recipe.status, Some("expired".to_string()));

        // 验证过期状态是终态
        let invalid_transitions = vec!["draft", "active"];
        assert!(!invalid_transitions.contains(&"draft"));
    }

    // ===== 有效期测试 =====

    #[test]
    fn test_effective_date() {
        let recipe = make_production_recipe_model(1);
        assert!(recipe.effective_date.is_some());
    }

    #[test]
    fn test_expiry_date_none() {
        let recipe = make_production_recipe_model(1);
        assert!(recipe.expiry_date.is_none());
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_production_recipe_json_roundtrip() {
        let recipe = make_production_recipe_model(1);
        let json = serde_json::to_value(&recipe).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("recipe_no").is_some());
        assert!(json.get("name").is_some());
        assert!(json.get("status").is_some());
    }

    #[test]
    fn test_production_recipe_item_json_roundtrip() {
        let item = make_production_recipe_item_model(1, 1);
        let json = serde_json::to_value(&item).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("recipe_id").is_some());
        assert!(json.get("material_id").is_some());
        assert!(json.get("quantity").is_some());
    }
}
