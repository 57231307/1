use bingxi_backend::models::dye_recipe::{Auxiliaries, AuxiliariesItem};
// 批次 212 P2-5 修复：master_data 仅测试使用，移入 #[cfg(test)] 避免 Clippy unused import
use bingxi_backend::models::dye_recipe::Model as DyeRecipeModel;
use bingxi_backend::models::status::master_data;
use bingxi_backend::services::ai::recipe_opt::sanitize_recipe_for_inference;
use bingxi_backend::services::ai::recipe_opt::{
    AggregatedParams, MAX_SIMILARITY, TYPICAL_TEMPERATURE, TYPICAL_TIME_MINUTES, build_candidates,
    compute_confidence, compute_similarity, find_typical_params, should_use_knn,
    weighted_average_params,
};
use chrono::Utc;
use rust_decimal::Decimal;

/// 染色配方测试夹具参数对象
/// 批次 338 v10 复审 P3 修复：引入参数对象消除 make_recipe 测试夹具的 too_many_arguments 警告。；聚合染色配方构造所需的全部字段，使用生命周期 `&'a str` 借用避免不必要的 to_string()。
struct RecipeFixture<'a> {
    recipe_no: &'a str,
    color_no: &'a str,
    fabric_type: &'a str,
    dye_type: &'a str,
    temperature: f64,
    time_minutes: i32,
    ph: f64,
    liquor: f64,
}

/// 构造一条 `DyeRecipeModel` 测试夹具
/// 批次 338 v10 复审 P3 修复：签名从 8 参数改为单一参数对象 `RecipeFixture`，；消除 `clippy::too_many_arguments` 警告。
fn make_recipe(fixture: RecipeFixture<'_>) -> DyeRecipeModel {
    let RecipeFixture {
        recipe_no,
        color_no,
        fabric_type,
        dye_type,
        temperature,
        time_minutes,
        ph,
        liquor,
    } = fixture;
    DyeRecipeModel {
        id: 0,
        recipe_no: recipe_no.to_string(),
        recipe_name: None,
        color_no: Some(color_no.to_string()),
        formula: None,
        temperature: Some(Decimal::try_from(temperature).unwrap_or(Decimal::ZERO)),
        time_minutes: Some(time_minutes),
        status: Some(master_data::ACTIVE.to_string()),
        is_deleted: Some(false),
        created_at: chrono::Utc::now().into(),
        updated_at: chrono::Utc::now().into(),
        color_code: None,
        color_name: Some("蓝色".to_string()),
        fabric_type: Some(fabric_type.to_string()),
        dye_type: Some(dye_type.to_string()),
        chemical_formula: None,
        ph_value: Some(Decimal::try_from(ph).unwrap_or(Decimal::ZERO)),
        liquor_ratio: Some(Decimal::try_from(liquor).unwrap_or(Decimal::ZERO)),
        auxiliaries: Some(Auxiliaries(vec![AuxiliariesItem {
            name: "助剂A".to_string(),
            amount: Decimal::try_from(1.5_f64).unwrap_or(Decimal::ZERO),
            unit: "g/L".to_string(),
        }])),
        version: Some(1),
        parent_recipe_id: None,
        approved_by: None,
        approved_at: None,
        remarks: None,
        created_by: None,
    }
}

/// 测试 1：典型参数退化路径（当数据库无匹配（或命中 < 3 条）时，返回内置典型参数表；温度 80°C ± 10、时间 45min ± 15、pH 6.0 ± 1、浴比 1:8 ± 2）
#[test]
fn test_typical_params_fallback() {
    let typical = find_typical_params();

    // 温度：80°C（±10）
    assert!(
        (typical.temperature - 80.0).abs() < 0.001,
        "典型温度应为 80.0，实际 {}",
        typical.temperature
    );
    assert!((typical.temperature - 80.0).abs() <= 10.0);

    // 时间：45min（±15）
    assert_eq!(typical.time_minutes as i32, 45);

    // pH：6.0（±1）
    assert!(
        (typical.ph_value - 6.0).abs() < 0.001,
        "典型 pH 应为 6.0，实际 {}",
        typical.ph_value
    );
    assert!((typical.ph_value - 6.0).abs() <= 1.0);

    // 浴比：1:8（±2）
    assert!(
        (typical.liquor_ratio - 8.0).abs() < 0.001,
        "典型浴比应为 8.0，实际 {}",
        typical.liquor_ratio
    );
    assert!((typical.liquor_ratio - 8.0).abs() <= 2.0);

    // 退化路径置信度固定 0.6
    let empty: Vec<(f64, &DyeRecipeModel)> = vec![];
    let conf = compute_confidence(&empty, 5);
    assert!(
        (conf - 0.6).abs() < 0.001,
        "退化置信度应为 0.6，实际 {}",
        conf
    );

    // should_use_knn 边界
    assert!(!should_use_knn(0));
    assert!(!should_use_knn(2));
    assert!(should_use_knn(3));
    assert!(should_use_knn(5));
}

/// 测试 2：颜色完全匹配时使用 k-NN 加权平均
/// 5 条完全匹配的配方 → 加权平均 = 各参数算术平均
#[test]
fn test_color_match_knn() {
    // 5 条全匹配：颜色 BL-301 + 棉 + 活性染料 → 相似度 1.3
    let history: Vec<DyeRecipeModel> = (0..5)
        .map(|i| {
            make_recipe(RecipeFixture {
                recipe_no: &format!("R-BL301-{}", i),
                color_no: "BL-301",
                fabric_type: "棉",
                dye_type: "活性染料",
                temperature: 60.0 + i as f64, // 60, 61, 62, 63, 64
                time_minutes: 40 + i * 2,     // 40, 42, 44, 46, 48
                ph: 6.0 + (i as f64) * 0.1,   // 6.0, 6.1, 6.2, 6.3, 6.4
                liquor: 10.0,
            })
        })
        .collect();

    // 走 k-NN 评分
    let mut scored: Vec<(f64, &DyeRecipeModel)> = history
        .iter()
        .map(|c| (compute_similarity("BL-301", "棉", Some("活性染料"), c), c))
        .filter(|(s, _)| *s > 0.0)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let top: Vec<(f64, &DyeRecipeModel)> = scored.into_iter().take(5).collect();
    assert_eq!(top.len(), 5);

    // 颜色完全匹配的相似度应为 1.0 + 0.2 + 0.1 = 1.3
    for (score, _) in &top {
        assert!(
            (*score - MAX_SIMILARITY).abs() < 0.001,
            "完全匹配相似度应为 {}，实际 {}",
            MAX_SIMILARITY,
            score
        );
    }

    // 加权平均：因为所有权重相同，等价于算术平均
    let agg = weighted_average_params(&top).expect("应当能聚合");
    // 温度均值 = (60+61+62+63+64)/5 = 62.0
    assert!(
        (agg.temperature - 62.0).abs() < 0.001,
        "温度均值应为 62.0，实际 {}",
        agg.temperature
    );
    // 时间均值 = (40+42+44+46+48)/5 = 44.0
    assert!(
        (agg.time_minutes - 44.0).abs() < 0.001,
        "时间均值应为 44.0，实际 {}",
        agg.time_minutes
    );
    // pH 均值 = (6.0+6.1+6.2+6.3+6.4)/5 = 6.2
    assert!(
        (agg.ph_value - 6.2).abs() < 0.001,
        "pH 均值应为 6.2，实际 {}",
        agg.ph_value
    );
    // 置信度：5/5 * 1.0（1.3 归一化） = 1.0
    let conf = compute_confidence(&top, 5);
    assert!(
        (conf - 1.0).abs() < 0.001,
        "5 条全匹配置信度应为 1.0，实际 {}",
        conf
    );

    // candidates 转换
    let cands = build_candidates(&top, 10);
    assert_eq!(cands.len(), 5);
    assert!((cands[0].similarity - 1.0).abs() < 0.001);
}

/// 测试 3：温度推荐 — 加权平均温度落在合理范围
/// 验证不同权重的加权平均算法正确性
#[test]
fn test_temperature_recommendation() {
    // 3 条历史：50 / 60 / 70，权重 1.0 / 1.3 / 0.5
    let r1 = make_recipe(RecipeFixture {
        recipe_no: "R-1",
        color_no: "BL-301",
        fabric_type: "棉",
        dye_type: "活性染料",
        temperature: 50.0,
        time_minutes: 30,
        ph: 7.0,
        liquor: 10.0,
    });
    let r2 = make_recipe(RecipeFixture {
        recipe_no: "R-2",
        color_no: "BL-301",
        fabric_type: "棉",
        dye_type: "活性染料",
        temperature: 60.0,
        time_minutes: 40,
        ph: 7.0,
        liquor: 10.0,
    });
    let r3 = make_recipe(RecipeFixture {
        recipe_no: "R-3",
        color_no: "BL-301",
        fabric_type: "棉",
        dye_type: "活性染料",
        temperature: 70.0,
        time_minutes: 50,
        ph: 7.0,
        liquor: 10.0,
    });
    let hits: Vec<(f64, &DyeRecipeModel)> = vec![(1.0, &r1), (1.3, &r2), (0.5, &r3)];

    let agg = weighted_average_params(&hits).expect("应当能聚合");
    // 期望温度 = (50*1.0 + 60*1.3 + 70*0.5) / (1.0+1.3+0.5) = 163/2.8 ≈ 58.21
    let expected_temp = 163.0_f64 / 2.8_f64;
    assert!(
        (agg.temperature - expected_temp).abs() < 0.01,
        "加权平均温度应为 {:.2}，实际 {:.2}",
        expected_temp,
        agg.temperature
    );

    // 温度应在合理范围（30-100°C）
    assert!(
        (30.0..=100.0).contains(&agg.temperature),
        "温度应在 30-100°C 之间，实际 {}",
        agg.temperature
    );

    // 期望时间 = (30*1.0 + 40*1.3 + 50*0.5) / 2.8 = 107/2.8 ≈ 38.21
    let expected_time = 107.0_f64 / 2.8_f64;
    assert!(
        (agg.time_minutes - expected_time).abs() < 0.01,
        "加权平均时间应为 {:.2}，实际 {:.2}",
        expected_time,
        agg.time_minutes
    );

    // 时间应在 10-120 min
    assert!(
        (10.0..=120.0).contains(&agg.time_minutes),
        "时间应在 10-120 min 之间，实际 {}",
        agg.time_minutes
    );

    // 置信度
    let conf = compute_confidence(&hits, 5);
    assert!(
        conf > 0.0 && conf <= 1.0,
        "置信度应在 0-1 之间，实际 {}",
        conf
    );
}

/// 测试 4：退化路径 — k=0 / 输入异常 / 命中 < 3 时
/// 全部回退到典型参数表
#[test]
fn test_fallback_path() {
    // 4.1 k=0 强制退化
    //   无 hits → 应返回 0.6 置信度
    let empty: Vec<(f64, &DyeRecipeModel)> = vec![];
    let conf_zero = compute_confidence(&empty, 0);
    assert!((conf_zero - 0.6).abs() < 0.001, "空命中置信度应为 0.6");

    // 4.2 命中 < 3 条时
    //   should_use_knn 边界
    assert!(!should_use_knn(0), "0 条应退化");
    assert!(!should_use_knn(1), "1 条应退化");
    assert!(!should_use_knn(2), "2 条应退化");
    assert!(should_use_knn(3), "3 条应走 k-NN");

    // 4.3 输入异常（color_no 全空字符串）
    let r = make_recipe(RecipeFixture {
        recipe_no: "R-1",
        color_no: "",
        fabric_type: "棉",
        dye_type: "活性染料",
        temperature: 60.0,
        time_minutes: 45,
        ph: 7.0,
        liquor: 10.0,
    });
    let s = compute_similarity("BL-301", "棉", Some("活性染料"), &r);
    assert!(s.abs() < 0.001, "候选 color 为空时相似度应为 0.0");

    // 4.4 完全不同 color_no → 相似度为 0
    let r2 = make_recipe(RecipeFixture {
        recipe_no: "R-2",
        color_no: "RD-999",
        fabric_type: "涤纶",
        dye_type: "分散染料",
        temperature: 130.0,
        time_minutes: 30,
        ph: 5.5,
        liquor: 8.0,
    });
    let s2 = compute_similarity("BL-301", "棉", Some("活性染料"), &r2);
    assert!(s2.abs() < 0.001, "完全无关候选相似度应为 0.0");

    // 4.5 颜色前缀 3 位匹配 → 0.7
    //   标准化后 "BL301" 与 "BL310" 前 3 位均为 "BL3"，触发 0.7 分
    let r3 = make_recipe(RecipeFixture {
        recipe_no: "R-3",
        color_no: "BL-310",
        fabric_type: "棉",
        dye_type: "活性染料",
        temperature: 60.0,
        time_minutes: 45,
        ph: 7.0,
        liquor: 10.0,
    });
    let s3 = compute_similarity("BL-301", "棉", Some("活性染料"), &r3);
    // 0.7 (color 前缀) + 0.2 (fabric) + 0.1 (dye) = 1.0
    assert!((s3 - 1.0).abs() < 0.001, "BL 前缀匹配应为 1.0，实际 {}", s3);

    // 4.6 典型参数表兜底
    let typical = find_typical_params();
    assert_eq!(typical.time_minutes as i32, TYPICAL_TIME_MINUTES);
    assert!((typical.temperature - TYPICAL_TEMPERATURE).abs() < 0.001);
}

/// 测试 5：V15 P1 6.1 配方候选脱敏
/// remark 中的手机号/邮箱/身份证号应在写入 candidates_json 前被掩码
#[test]
fn test_sanitize_recipe_masks_pii() {
    let recipe = DyeRecipeModel {
        id: 0,
        recipe_no: "R-PII-1".to_string(),
        recipe_name: None,
        color_no: Some("BL-301".to_string()),
        formula: None,
        temperature: Some(Decimal::try_from(80.0_f64).unwrap_or(Decimal::ZERO)),
        time_minutes: Some(45),
        status: Some(master_data::ACTIVE.to_string()),
        is_deleted: Some(false),
        created_at: chrono::Utc::now().into(),
        updated_at: chrono::Utc::now().into(),
        color_code: None,
        color_name: Some("客户张三 13812348888".to_string()),
        fabric_type: Some("棉".to_string()),
        dye_type: Some("活性".to_string()),
        chemical_formula: None,
        ph_value: Some(Decimal::try_from(6.0_f64).unwrap_or(Decimal::ZERO)),
        liquor_ratio: Some(Decimal::try_from(8.0_f64).unwrap_or(Decimal::ZERO)),
        auxiliaries: None,
        version: Some(1),
        parent_recipe_id: None,
        approved_by: None,
        approved_at: None,
        remarks: Some("联系 13812348888 反馈色差".to_string()),
        created_by: None,
    };
    let sanitized = sanitize_recipe_for_inference(recipe);
    let remark = sanitized.remarks.expect("脱敏后 remark 应保留");
    assert!(
        !remark.contains("13812348888"),
        "手机号应被脱敏，实际 {}",
        remark
    );
    assert!(remark.contains("色差"), "非 PII 文本应保留");
    let name = sanitized.color_name.expect("脱敏后 color_name 应保留");
    assert!(!name.contains("13812348888"), "color_name 中手机号应被脱敏");
}
