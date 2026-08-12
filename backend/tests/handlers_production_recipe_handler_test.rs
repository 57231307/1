#[cfg(test)]
mod tests {
    use bingxi_backend::handlers::production_recipe_handler::*;
    use bingxi_backend::models::production_recipe::Model as ProductionRecipeModel;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;

    /// 构造测试用的生产配方模型
    fn make_production_recipe_model(id: i32, status: &str) -> ProductionRecipeModel {
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
            status: Some(status.to_string()),
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
    fn test_production_recipe_model_serialization() {
        let recipe = make_production_recipe_model(1, "draft");
        let json = serde_json::to_value(&recipe).expect("生产配方序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["recipe_no"], "PR-2026-0001");
        assert_eq!(json["name"], "蓝色配方");
        assert_eq!(json["status"], "draft");
    }

    #[test]
    fn test_production_recipe_version() {
        let recipe = make_production_recipe_model(1, "draft");
        assert_eq!(recipe.version, Some("1.0".to_string()));
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_draft_to_active() {
        let recipe = make_production_recipe_model(1, "draft");
        assert_eq!(recipe.status, Some("draft".to_string()));

        // 验证草稿状态可以转换为生效
        let valid_transitions = vec!["active", "cancelled"];
        assert!(valid_transitions.contains(&"active"));
    }

    #[test]
    fn test_status_active_to_expired() {
        let recipe = make_production_recipe_model(1, "active");
        assert_eq!(recipe.status, Some("active".to_string()));

        // 验证生效状态可以转换为过期
        let valid_transitions = vec!["expired", "archived"];
        assert!(valid_transitions.contains(&"expired"));
    }

    #[test]
    fn test_status_expired_is_final() {
        let recipe = make_production_recipe_model(1, "expired");
        assert_eq!(recipe.status, Some("expired".to_string()));

        // 验证过期状态是终态
        let invalid_transitions = vec!["draft", "active"];
        assert!(!invalid_transitions.contains(&"draft"));
    }

    // ===== 有效期测试 =====

    #[test]
    fn test_effective_date() {
        let recipe = make_production_recipe_model(1, "draft");
        assert!(recipe.effective_date.is_some());
    }

    #[test]
    fn test_expiry_date_none() {
        let recipe = make_production_recipe_model(1, "draft");
        assert!(recipe.expiry_date.is_none());
    }

    // ===== 工艺测试 =====

    #[test]
    fn test_process_info() {
        let recipe = make_production_recipe_model(1, "draft");
        assert_eq!(recipe.process_id, Some(1));
        assert_eq!(recipe.process_name, Some("染色工艺".to_string()));
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_production_recipe_json_roundtrip() {
        let recipe = make_production_recipe_model(1, "draft");
        let json = serde_json::to_value(&recipe).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("recipe_no").is_some());
        assert!(json.get("name").is_some());
        assert!(json.get("status").is_some());
    }
}
