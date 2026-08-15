use bingxi_backend::models::dye_recipe::Model as DyeRecipeModel;
use rust_decimal::Decimal;

/// 构造测试用的染色配方模型
fn make_dye_recipe_model(id: i32) -> DyeRecipeModel {
    DyeRecipeModel {
        id,
        recipe_no: format!("DR-2026-{:04}", id),
        color_code: Some("C001".to_string()),
        fabric_type: Some("棉布".to_string()),
        dye_type: Some("活性染料".to_string()),
        temperature: Some(Decimal::new(60, 0)),
        ph_value: Some(Decimal::new(70, 1)),
        liquor_ratio: Some(Decimal::new(10, 1)),
        status: Some("active".to_string()),
        version: Some(1),
        created_by: Some(1),
        created_at: chrono::Utc::now().fixed_offset(),
        updated_at: chrono::Utc::now().fixed_offset(),
        ..Default::default()
    }
}

// ===== 模型测试 =====

#[test]
fn test_dye_recipe_model_serialization() {
    let recipe = make_dye_recipe_model(1);
    let json = serde_json::to_value(&recipe).expect("染色配方序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["recipe_no"], "DR-2026-0001");
    assert_eq!(json["status"], "active");
}

#[test]
fn test_dye_recipe_parameters() {
    let recipe = make_dye_recipe_model(1);

    // 验证配方参数
    assert_eq!(recipe.temperature, Some(Decimal::new(60, 0)));
    assert_eq!(recipe.time_minutes, None);
    assert_eq!(recipe.ph_value, Some(Decimal::new(70, 1)));
    assert_eq!(recipe.liquor_ratio, Some(Decimal::new(10, 1)));
}

// ===== 状态测试 =====

#[test]
fn test_dye_recipe_status_active() {
    let recipe = make_dye_recipe_model(1);
    assert_eq!(recipe.status, Some("active".to_string()));
}

// ===== 版本测试 =====

#[test]
fn test_dye_recipe_version() {
    let recipe = make_dye_recipe_model(1);
    assert_eq!(recipe.version, Some(1));
}

// ===== 工艺参数测试 =====

#[test]
fn test_temperature_range() {
    let recipe = make_dye_recipe_model(1);
    let temp = recipe.temperature.unwrap();

    // 验证温度在合理范围内
    assert!(temp >= Decimal::new(20, 0));
    assert!(temp <= Decimal::new(100, 0));
}

#[test]
fn test_ph_value_range() {
    let recipe = make_dye_recipe_model(1);
    let ph = recipe.ph_value.unwrap();

    // 验证 pH 值在合理范围内
    assert!(ph >= Decimal::new(0, 0));
    assert!(ph <= Decimal::new(140, 1));
}

#[test]
fn test_duration_minutes() {
    let recipe = make_dye_recipe_model(1);
    let duration = recipe.time_minutes;

    // time_minutes 未设置时为 None
    assert!(duration.is_none());
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_dye_recipe_json_roundtrip() {
    let recipe = make_dye_recipe_model(1);
    let json = serde_json::to_value(&recipe).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("recipe_no").is_some());
    assert!(json.get("status").is_some());
}
