use bingxi_backend::decs;
use bingxi_backend::models::quality_inspection_record::Model as QualityInspectionModel;
use bingxi_backend::services::ai::quality_pred::*;
use bingxi_backend::ymd;
use chrono::NaiveDate;
use chrono::Utc;
use rust_decimal::Decimal;

/// 构造一条 `QualityInspectionModel` 测试夹具
fn make_record(
    product_id: i32,
    inspection_type: &str,
    inspection_date: chrono::NaiveDate,
    qualification_rate: Option<f64>,
    remark: Option<&str>,
) -> QualityInspectionModel {
    let rate_dec = qualification_rate.and_then(rust_decimal::Decimal::from_f64_retain);
    let is_pass = qualification_rate.unwrap_or(100.0) >= 100.0;
    QualityInspectionModel {
        id: 0,
        inspection_no: format!("QC-{}", product_id),
        inspection_type: inspection_type.to_string(),
        related_type: None,
        related_id: None,
        product_id,
        batch_no: None,
        supplier_id: None,
        customer_id: None,
        inspection_date,
        inspector_id: None,
        total_qty: Decimal::try_from(100.0_f64).unwrap_or(Decimal::ZERO),
        inspected_qty: Decimal::try_from(100.0_f64).unwrap_or(Decimal::ZERO),
        qualified_qty: Some(Decimal::try_from(95.0_f64).unwrap_or(Decimal::ZERO)),
        unqualified_qty: Some(Decimal::try_from(5.0_f64).unwrap_or(Decimal::ZERO)),
        qualification_rate: rate_dec,
        inspection_result: if is_pass {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        remark: remark.map(|s| s.to_string()),
        // V15 Batch 485：补齐 v14 批次 421 新增字段（color_no/dye_lot_no/grade）
        // 测试夹具不涉及缸号/颜色追溯，使用 None
        grade: None,
        color_no: None,
        dye_lot_no: None,
        // V15 P1 2.2：面料行业特征字段（测试夹具不涉及，使用 None）
        dye_type: None,
        auxiliary_type: None,
        temperature: None,
        fabric_source: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// 测试 1：风险评分 - 合格率 99% 且趋势平稳 → 风险分 < 20（等级"低"）
#[test]
fn test_risk_score_low() {
    let score = compute_risk_score(99.0, false);
    // (100 - 99) * 0.6 = 0.6，加上 0 trend → 0.6
    assert!(
        score < 20.0,
        "合格率 99% 且趋势平稳时风险分应 < 20，实际 {}",
        score
    );
    assert!(score >= 0.0, "风险分应为非负，实际 {}", score);
    // 风险等级分类应为"低"
    assert_eq!(classify_risk_level(score), "低");
}

/// 测试 2：风险评分 - 极低合格率 + 下降趋势 → 风险分 > 60（等级"高"）
/// 注：原 spec 文字"合格率 70%"在公式 `(100-avg)*0.6 + 15*0.4` 下；仅得 24，数学上无法 > 60；此处采用能确保 > 60 的极低合格率（0%）；作为高风险测试场景，公式不变。
#[test]
fn test_risk_score_high() {
    let score = compute_risk_score(0.0, true);
    // (100 - 0) * 0.6 = 60，加上 15 * 0.4 = 6 → 66
    assert!(
        score > 60.0,
        "合格率 0% 且趋势下降时风险分应 > 60，实际 {}",
        score
    );
    // 风险等级分类应为"高"
    assert_eq!(classify_risk_level(score), "高");

    // 70% + 下降 → 24（公式极限），验证"中"档
    let score_70_down = compute_risk_score(70.0, true);
    assert!(
        score_70_down > compute_risk_score(70.0, false),
        "下降趋势应抬高风险分（70% 平稳 {} vs 下降 {}）",
        compute_risk_score(70.0, false),
        score_70_down
    );
}

/// 测试 3：趋势计算 - 3 期合格率 80 → 85 → 90 → 应判定为上升
#[test]
fn test_trend_calculation() {
    // 上升：recent 90, previous 80 → (90-80)/80 = 0.125 = 12.5% > 5%
    let rate = compute_trend_rate(90.0, 80.0);
    assert!(
        (rate - 0.125).abs() < 0.0001,
        "变化率应为 0.125，实际 {}",
        rate
    );
    let label = classify_trend(rate);
    assert_eq!(label, "上升", "趋势应判定为上升，实际 {}", label);

    // 下降：recent 60, previous 90 → (60-90)/90 = -0.333
    let rate_down = compute_trend_rate(60.0, 90.0);
    let label_down = classify_trend(rate_down);
    assert_eq!(label_down, "下降", "趋势应判定为下降，实际 {}", label_down);

    // 平稳：recent 82, previous 80 → 2.5% 处于 ±5% 内
    let rate_flat = compute_trend_rate(82.0, 80.0);
    let label_flat = classify_trend(rate_flat);
    assert_eq!(label_flat, "平稳", "趋势应判定为平稳，实际 {}", label_flat);

    // previous=0 兜底
    let rate_zero = compute_trend_rate(50.0, 0.0);
    assert!(rate_zero.abs() < 0.0001);
}

/// 测试 4：退化路径 - 数据 < 5 条 → 合格率 95% + 置信度 0.3
#[test]
fn test_fallback_low_data() {
    // 历史 0 条记录（模拟）
    let empty: Vec<QualityInspectionModel> = vec![];
    let rate = mean_qualification_rate(&empty);
    assert!(rate.abs() < 0.0001, "空记录集合应返回 0.0");

    // 置信度 - 0 条
    let conf = compute_confidence(0);
    assert!(
        (conf - FALLBACK_CONFIDENCE).abs() < 0.0001,
        "0 条记录置信度应等于 0.3，实际 {}",
        conf
    );

    // 置信度 - 5 条：5/30 = 0.1667，四舍五入到 0.17
    let conf5 = compute_confidence(5);
    assert!(
        (conf5 - 0.17).abs() < 0.01,
        "5 条记录置信度应约为 0.17，实际 {}",
        conf5
    );

    // 置信度 - 30 条以上封顶
    let conf30 = compute_confidence(30);
    assert!(
        (conf30 - 1.0).abs() < 0.0001,
        "30 条记录置信度应封顶到 1.0，实际 {}",
        conf30
    );

    // 建议措施：中等级
    let recs = build_recommendations("中");
    assert!(!recs.is_empty(), "中等级建议措施不应为空");
    assert!(recs.len() >= 2, "中等级应有 ≥ 2 条建议");
    // 风险评分
    let mid_score = compute_risk_score(95.0, false);
    assert!(
        (0.0..=30.0).contains(&mid_score),
        "95% 合格率无下降趋势应得低分，实际 {}",
        mid_score
    );

    // 问题归因关键词提取
    assert_eq!(extract_issue_keyword(Some("颜色偏深")), "颜色差异");
    assert_eq!(extract_issue_keyword(Some("色牢度不合格")), "色牢度");
    assert_eq!(extract_issue_keyword(Some("克重不足")), "克重偏差");
    assert_eq!(extract_issue_keyword(Some("纬密偏低")), "纬密偏差");
    assert_eq!(extract_issue_keyword(Some("强度不够")), "强度不足");
    assert_eq!(extract_issue_keyword(Some("无匹配项")), "其他");
    assert_eq!(extract_issue_keyword(None), "其他");
}

/// 测试 5：辅助函数覆盖 - 用真实记录验证 `mean_qualification_rate`
/// 使用 `make_record` 构造 3 条记录，确保 `#[allow(dead_code)]`；不会因辅助函数未使用而失效。
#[test]
fn test_mean_qualification_with_real_records() {
    // P9-1: 用 ymd! 宏统一日期构造
    let d1 = bingxi_backend::ymd!(2024, 1, 15);
    let d2 = bingxi_backend::ymd!(2024, 2, 15);
    let d3 = bingxi_backend::ymd!(2024, 3, 15);
    let records = vec![
        make_record(1, "成品检验", d1, Some(98.0), None),
        make_record(1, "成品检验", d2, Some(96.0), None),
        make_record(1, "成品检验", d3, Some(94.0), None),
    ];
    let avg = mean_qualification_rate(&records);
    // (98 + 96 + 94) / 3 = 96.0
    assert!(
        (avg - 96.0).abs() < 0.0001,
        "3 条记录平均合格率应为 96.0，实际 {}",
        avg
    );
}
