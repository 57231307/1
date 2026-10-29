//! 销售分析目标契约字段集锁（纯形态锁，不依赖活库，CI 常规可跑）。
//!
//! 锁定不变量（对应看板缺陷族：派生量恒等、GET 缺五键、dimension_name 挪用）：
//! 1. `SalesTargetDto` 出参 JSON 键集固定为 id/period/target_amount/actual_amount/
//!    completion_rate/variance/status 七枚，金额与比率键线格式为字符串（rust_decimal
//!    仅启用 serde 未启用 serde-float），null 仅出现在"无口径"三态；
//! 2. 唯一映射点 `to_target_dto` 的派生列以入参 actual 为分子：actual 与 target
//!    不相等时 completion_rate 严格不为恒 100、variance 严格不为恒 0
//!    （反向自证：若分子分母又被人改成同源，本组断言必红）；
//! 3. "未设目标"（target 为 0）与"零完成"（target 大于 0 且 actual 为 0）两态可辨，
//!    前者渲染 null、后者渲染 0.00，不得拿 0 兜底混同；
//! 4. service 源码不得重新引入退化形态（actual 取 total_amount 自身、
//!    remarks 挪用写 dimension_name），且实际额必须经取数腿现算。

use bingxi_backend::models::sales_analysis;
use bingxi_backend::services::sales_analysis_service::SalesAnalysisService;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

/// 构造一条目标存储行（total_amount 即目标额列，其余列取中性值）。
/// 仅测试内使用，不落库、不连库。
fn sample_model(period: &str, target_amount: &str) -> sales_analysis::Model {
    sales_analysis::Model {
        id: 1,
        statistic_type: "target".to_string(),
        period: period.to_string(),
        dimension_type: "overall".to_string(),
        dimension_id: None,
        dimension_name: None,
        order_count: 0,
        total_amount: Decimal::from_str(target_amount).unwrap(),
        total_qty: Decimal::ZERO,
        total_cost: Decimal::ZERO,
        gross_profit: Decimal::ZERO,
        gross_profit_rate: Decimal::ZERO,
        avg_order_value: Decimal::ZERO,
        created_at: chrono::Utc::now(),
    }
}

/// 映射点出参序列化后的 JSON 对象（GET 与 PUT 共用同一 DTO，锁一处即锁两腿）。
fn dto_json(
    model: &sales_analysis::Model,
    actual: Option<Decimal>,
) -> serde_json::Map<String, serde_json::Value> {
    let dto = SalesAnalysisService::to_target_dto(model, actual, "active".to_string());
    match serde_json::to_value(&dto).unwrap() {
        serde_json::Value::Object(map) => map,
        other => panic!("目标出参必须是 JSON 对象，实际：{other}"),
    }
}

/// 契约键集锁：七枚键全部存在，缺任何一枚即红。
#[test]
fn target_dto_field_set_contains_all_seven_contract_keys() {
    let model = sample_model("2026-08", "1000.00");
    let map = dto_json(&model, Some(dec!(750.00)));
    for key in [
        "id",
        "period",
        "target_amount",
        "actual_amount",
        "completion_rate",
        "variance",
        "status",
    ] {
        assert!(map.contains_key(key), "目标出参缺少契约键 {key}");
    }
    // DECIMAL 线格式：金额与比率键序列化为字符串而非 JSON number
    assert!(
        map["target_amount"].is_string(),
        "target_amount 必须为字符串"
    );
    assert!(
        map["actual_amount"].is_string(),
        "actual_amount 必须为字符串"
    );
    assert!(
        map["completion_rate"].is_string(),
        "有口径时 completion_rate 必须为字符串"
    );
    assert!(
        map["variance"].is_string(),
        "有口径时 variance 必须为字符串"
    );
}

/// 恒等派生反向自证：actual（入参）与 target（存储列）不相等时，
/// 完成率严格不等于 100、差额严格不等于 0，且数值按真除法与真减法成立。
/// 退化形态（分子取 total_amount 自身）会在这里变红。
#[test]
fn derived_columns_are_not_tautological_when_actual_differs_from_target() {
    let model = sample_model("2026-08", "1000.00");
    let dto = SalesAnalysisService::to_target_dto(&model, Some(dec!(750.00)), "active".to_string());

    let rate = dto
        .completion_rate
        .expect("target 大于 0 且 actual 有值时完成率必须有值");
    assert!(
        rate != dec!(100),
        "完成率恒 100：分子分母疑似同源（actual 被取成 target 自身）"
    );
    assert_eq!(rate, dec!(75), "完成率必须等于 actual/target");
    assert_eq!(rate.to_string(), "75.00", "完成率线格式保持两位小数刻度");

    let variance = dto.variance.expect("actual 有值时差额必须有值");
    assert!(
        variance != Decimal::ZERO,
        "差额恒 0：actual 疑似又被取成 target 自身"
    );
    assert_eq!(variance, dec!(-250), "差额必须等于 actual 减 target");

    assert_eq!(
        dto.actual_amount,
        Some(dec!(750.00)),
        "actual_amount 必须原样取入参而非存储目标列"
    );
    assert_eq!(
        dto.target_amount,
        dec!(1000.00),
        "target_amount 必须取存储行 total_amount 列"
    );
}

/// 两态可辨锁：未设目标（target 为 0）渲染 null 完成率（除法无口径，不以 0 掩盖）；
/// 零完成（target 大于 0 且 actual 为 0）渲染 0.00，与 null 严格区分。
#[test]
fn unset_target_and_zero_completion_are_distinguishable() {
    // 未设目标：target 为 0，actual 有成交
    let unset = sample_model("2026-09", "0.00");
    let map = dto_json(&unset, Some(dec!(200.00)));
    assert!(
        map["completion_rate"].is_null(),
        "未设目标时完成率必须为 null（除法无口径），不得拿 0 兜底"
    );
    assert_eq!(map["actual_amount"].as_str(), Some("200.00"));
    assert_eq!(
        map["variance"].as_str(),
        Some("200.00"),
        "未设目标语境下差额即成交溢出额"
    );

    // 零完成：target 已设，actual 为 0
    let zero = sample_model("2026-09", "1000.00");
    let dto = SalesAnalysisService::to_target_dto(&zero, Some(Decimal::ZERO), "active".to_string());
    let rate = dto
        .completion_rate
        .expect("零完成的完成率必须有值，不得与未设目标混同为 null");
    assert_eq!(rate, Decimal::ZERO);
    assert_eq!(rate.to_string(), "0.00", "零完成保持 0.00 字符串刻度");
    assert_eq!(dto.variance, Some(dec!(-1000)));
}

/// 周期无法对齐取数口径时（actual 为 null），三枚派生列全部 null，
/// target_amount 与 status 不受影响。
#[test]
fn unresolvable_actual_renders_all_derived_columns_null() {
    let model = sample_model("2026/08", "1000.00");
    let map = dto_json(&model, None);
    assert!(
        map["actual_amount"].is_null(),
        "无法判定周期口径时 actual_amount 必须为 null"
    );
    assert!(
        map["completion_rate"].is_null(),
        "无法判定周期口径时 completion_rate 必须为 null"
    );
    assert!(
        map["variance"].is_null(),
        "无法判定周期口径时 variance 必须为 null"
    );
    assert_eq!(map["target_amount"].as_str(), Some("1000.00"));
    assert_eq!(map["status"].as_str(), Some("active"));
}

/// 源码退化锁：service 文件不得重新引入历史退化形态——
/// actual 取 total_amount 自身、remarks 挪用写 dimension_name；
/// 且实际额取数腿必须存在（复用 BI 聚合的 sales_by_time）。
#[test]
fn service_source_does_not_reintroduce_derived_degeneracies() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services/sales_analysis_service.rs");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e));
    for banned in [
        "let actual_amount = updated.total_amount",
        "actual_amount: updated.total_amount",
        "dimension_name = Set(req.remarks",
        "dimension_name: Set(req.remarks",
    ] {
        assert!(
            !src.contains(banned),
            "service 源码重新引入退化形态：{banned}"
        );
    }
    assert!(
        src.contains("sales_by_time"),
        "实际成交额必须经复用 BI 聚合 sales_by_time 现算，取数腿不得消失"
    );
}

/// 查找定行锁：PUT 查目标行必须显式收窄到总体目标腿并做命中基数判定，
/// 不得回退到"周期键跨维度捞多行 + 单行查询"的驱动层异常形态。
#[test]
fn service_source_locks_overall_only_lookup_with_cardinality_check() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services/sales_analysis_service.rs");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e));
    assert!(
        src.contains("DimensionType.eq(\"overall\")") && src.contains("DimensionId.is_null()"),
        "PUT 查找必须同时限定 dimension_type=overall 且 dimension_id 为空"
    );
    assert!(
        src.contains("rows.len() > 1"),
        "命中多于 1 行必须走显式基数判定返回业务错误，不得回退单行查询"
    );
    assert!(
        !src.contains(".one(&*self.db)"),
        "service 内不得残留单行查询形态（多行命中会退化成驱动层 500）"
    );
}
