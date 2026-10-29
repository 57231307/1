//! 产品主档录入面收口契约锁（新建必录判据、DECIMAL 入参宽容形态、更新三态、出米数换算口径）。
//!
//! 钉住的对象与不连库判据：
//! 1. `bingxi_backend::services::product_service::validate_product_create_fields`：
//!    面料类新建必录（幅宽/克重/面料成分）与产品类型显式提交的判据源，正向放行与反向点名拒绝双向断言。
//! 2. `bingxi_backend::handlers::product_handler::{CreateProductRequest, UpdateProductRequest}`：
//!    十进制字符串与 JSON 数字两形态都必须能反序列化（编辑回显原样回传不再 400 的直接钉点）；
//!    更新 DTO 数值主档列三态（键缺席保持原值、显式 null 清空、有值覆盖）。
//! 3. `bingxi_backend::utils::dual_unit_converter::DualUnitConverter::meters_per_kg`：
//!    每公斤出米数向下取整保留一位小数（floor 非四舍五入），缺值与非正返回 None。
//! 全部为纯函数与 serde 形态断言，不启动服务、不连库、不起 dev server。

use bingxi_backend::handlers::product_handler::{CreateProductRequest, UpdateProductRequest};
use bingxi_backend::services::product_service::validate_product_create_fields;
use bingxi_backend::utils::dual_unit_converter::DualUnitConverter;
use rust_decimal::Decimal;
use serde_json::json;

/// 幅宽与克重以十进制字符串形态提交（库里 DECIMAL(10,2) 的线上形态）必须可反序列化。
/// 正向钉 CreateProductRequest 与 UpdateProductRequest 两个 DTO：值取到且等于十进制真相。
/// 反向钉坏形态（"abc"）必须被拒，不许静默折叠成 0 或缺值。
#[test]
fn decimal_string_form_accepted_by_both_dto_directions() {
    let created: CreateProductRequest = serde_json::from_value(json!({
        "name": "全棉汗布",
        "product_type": "成品布",
        "width": "165.50",
        "gram_weight": "180.25",
        "standard_price": "12.30",
        "cost_price": "9.80",
        "min_order_quantity": "300.00",
        "meters_per_piece": "100.00"
    }))
    .expect(
        "十进制字符串形态的幅宽克重价格必须可反序列化，不得报 invalid type: string, expected f64",
    );
    assert_eq!(created.width, Some(Decimal::new(16550, 2)));
    assert_eq!(created.gram_weight, Some(Decimal::new(18025, 2)));
    assert_eq!(
        created.standard_price,
        Some(Decimal::new(1230, 2)),
        "标准价出参回传同为字符串形态，必须同口径可收"
    );
    assert_eq!(
        created.min_order_quantity,
        Some(Decimal::new(30000, 2)),
        "最小起订量出参回传同为字符串形态，必须同口径可收"
    );

    let updated: UpdateProductRequest = serde_json::from_value(json!({
        "width": "165.50",
        "gram_weight": "180.25"
    }))
    .expect("更新 DTO 同样必须接受十进制字符串形态");
    assert_eq!(updated.width, Some(Some(Decimal::new(16550, 2))));
    assert_eq!(updated.gram_weight, Some(Some(Decimal::new(18025, 2))));

    let bad = serde_json::from_value::<CreateProductRequest>(json!({
        "name": "坏形态",
        "width": "abc"
    }));
    assert!(
        bad.is_err(),
        "非数字文本必须是可追踪的反序列化失败，不得折叠成缺值或 0"
    );
}

/// JSON 数字形态保持可反序列化（既有前端数值提交面不回归）。
#[test]
fn decimal_number_form_still_accepted() {
    let created: CreateProductRequest = serde_json::from_value(json!({
        "name": "数字形态",
        "product_type": "坯布",
        "width": 165.5,
        "gram_weight": 180,
        "standard_price": 12.3
    }))
    .expect("数字形态必须继续可反序列化");
    assert_eq!(
        created.width,
        Some(Decimal::new(1655, 1)),
        "数字 165.5 归一为十进制 165.5，值不因形态改变"
    );
    assert_eq!(
        created.gram_weight,
        Some(Decimal::new(180, 0)),
        "整数 180 收为十进制 180，值不因形态改变"
    );
}

/// 更新 DTO 数值主档列三态：键缺席为 None（保持原值）、显式 null 为 Some(None)（清空）、
/// 有值为 Some(Some(v))（覆盖）。钉清空通道存在，防止界面清空被静默丢弃。
#[test]
fn update_decimal_columns_are_tri_state() {
    let absent: UpdateProductRequest =
        serde_json::from_value(json!({ "name": "只改名" })).expect("键缺席必须可解析");
    assert_eq!(absent.width, None, "键缺席必须是外层 None，即保持原值");
    assert_eq!(absent.standard_price, None);

    let cleared: UpdateProductRequest = serde_json::from_value(json!({
        "width": null,
        "gram_weight": null,
        "standard_price": null,
        "cost_price": null,
        "min_order_quantity": null,
        "lead_time": null
    }))
    .expect("显式 null 必须可解析");
    assert_eq!(
        cleared.width,
        Some(None),
        "显式 null 必须是 Some(None)，即清空该列"
    );
    assert_eq!(cleared.gram_weight, Some(None));
    assert_eq!(cleared.standard_price, Some(None));
    assert_eq!(cleared.min_order_quantity, Some(None));
    assert_eq!(cleared.lead_time, Some(None));

    let covered: UpdateProductRequest =
        serde_json::from_value(json!({ "width": "165.50" })).expect("有值必须可解析");
    assert_eq!(
        covered.width,
        Some(Some(Decimal::new(16550, 2))),
        "有值必须是 Some(Some(v))，即覆盖为该值"
    );
}

/// 面料类新建必录判据正向面：类型显式提交且三列齐备（含 0 下限外的合法值）必须放行。
#[test]
fn fabric_create_gate_passes_complete_rows() {
    validate_product_create_fields(
        Some("成品布"),
        Some(Decimal::new(16550, 2)),
        Some(Decimal::new(18000, 2)),
        Some("95%棉5%氨纶"),
    )
    .expect("面料类齐备行必须放行");
    validate_product_create_fields(
        Some("坯布"),
        Some(Decimal::new(160, 0)),
        Some(Decimal::new(200, 0)),
        Some(" 全棉 "),
    )
    .expect("成分含实际字符（两侧空白）必须放行，判据按 trim 后非空");
    validate_product_create_fields(Some("辅料"), None, None, None)
        .expect("非面料类不要求幅宽克重成分，必须放行");
}

/// 面料类新建必录判据反向面：每个缺失点各点名各字段，拒绝文案可外显真实原因。
/// 断言按「命中哪一缺项就出现哪一名词」逐项判定，防止合并成一句模糊提示后失去定位力。
#[test]
fn fabric_create_gate_rejects_each_missing_column_by_name() {
    let no_type =
        validate_product_create_fields(None, Some(Decimal::ONE), Some(Decimal::ONE), Some("全棉"))
            .expect_err("产品类型缺省必须被拒");
    assert!(
        no_type.to_string().contains("产品类型"),
        "拒绝文案必须点名产品类型，实际为：{}",
        no_type
    );

    let blank_type = validate_product_create_fields(
        Some("   "),
        Some(Decimal::ONE),
        Some(Decimal::ONE),
        Some("全棉"),
    )
    .expect_err("纯空白类型等同缺失，必须被拒");
    assert!(
        blank_type.to_string().contains("产品类型"),
        "实际为：{}",
        blank_type
    );

    let no_width =
        validate_product_create_fields(Some("成品布"), None, Some(Decimal::ONE), Some("全棉"))
            .expect_err("面料类缺幅宽必须被拒");
    assert!(
        no_width.to_string().contains("幅宽"),
        "实际为：{}",
        no_width
    );

    let zero_width = validate_product_create_fields(
        Some("坯布"),
        Some(Decimal::ZERO),
        Some(Decimal::ONE),
        Some("全棉"),
    )
    .expect_err("幅宽为 0 不是合法录入，必须被拒而不是落库成 0");
    assert!(
        zero_width.to_string().contains("幅宽"),
        "实际为：{}",
        zero_width
    );

    let negative_width = validate_product_create_fields(
        Some("成品布"),
        Some(Decimal::new(-100, 2)),
        Some(Decimal::ONE),
        Some("全棉"),
    )
    .expect_err("负幅宽必须被拒");
    assert!(
        negative_width.to_string().contains("幅宽"),
        "实际为：{}",
        negative_width
    );

    let no_gram =
        validate_product_create_fields(Some("成品布"), Some(Decimal::ONE), None, Some("全棉"))
            .expect_err("面料类缺克重必须被拒");
    assert!(no_gram.to_string().contains("克重"), "实际为：{}", no_gram);

    let no_composition = validate_product_create_fields(
        Some("成品布"),
        Some(Decimal::ONE),
        Some(Decimal::ONE),
        None,
    )
    .expect_err("面料类缺成分必须被拒");
    assert!(
        no_composition.to_string().contains("面料成分"),
        "实际为：{}",
        no_composition
    );

    let blank_composition = validate_product_create_fields(
        Some("坯布"),
        Some(Decimal::ONE),
        Some(Decimal::ONE),
        Some("   "),
    )
    .expect_err("纯空白成分等同缺失，必须被拒");
    assert!(
        blank_composition.to_string().contains("面料成分"),
        "实际为：{}",
        blank_composition
    );
}

/// 判据错误必须是可外显校验族（HTTP 400 / VALIDATION_ERROR 且 message 携带点名原因），
/// 不许落进脱敏固定文案把「缺哪一列」吞掉。
#[test]
fn gate_rejection_is_displayable_validation_family() {
    use axum::response::IntoResponse as _;
    let err =
        validate_product_create_fields(Some("成品布"), None, Some(Decimal::ONE), Some("全棉"))
            .expect_err("缺幅宽必须返回错误");
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "判据拒绝必须留在输入校验族"
    );
    assert!(
        err.to_response().message.contains("幅宽"),
        "出参 message 必须携带点名原因（fail-visible），实际为：{}",
        err.to_response().message
    );
    assert_eq!(
        err.into_response().status(),
        axum::http::StatusCode::BAD_REQUEST,
        "判据拒绝必须是 400"
    );
}

/// 每公斤出米数向下取整保留一位小数：3.333→3.3、3.267→3.2（floor 与 round 在 .5 边界可分辨），
/// 缺值与非正返回 None（不得把未录入算成 0 米）。
#[test]
fn meters_per_kg_floors_to_one_decimal() {
    assert_eq!(
        DualUnitConverter::meters_per_kg(Some(Decimal::new(200, 0)), Some(Decimal::new(150, 0))),
        Some(Decimal::new(33, 1)),
        "1000÷(200×1.5)=3.333… 向下截断一位小数应为 3.3"
    );
    assert_eq!(
        DualUnitConverter::meters_per_kg(Some(Decimal::new(170, 0)), Some(Decimal::new(180, 0))),
        Some(Decimal::new(32, 1)),
        "1000÷(170×1.8)=3.267… 向下截断应为 3.2（round 会变 3.3，本断言分辨口径）"
    );
    assert_eq!(
        DualUnitConverter::meters_per_kg(None, Some(Decimal::ONE)),
        None,
        "缺克重不可算，不得下发 0"
    );
    assert_eq!(
        DualUnitConverter::meters_per_kg(Some(Decimal::ONE), None),
        None,
        "缺幅宽不可算，不得下发 0"
    );
    assert_eq!(
        DualUnitConverter::meters_per_kg(Some(Decimal::ZERO), Some(Decimal::ONE)),
        None,
        "克重为 0 不可算，不得下发 0 冒充结果"
    );
    assert_eq!(
        DualUnitConverter::meters_per_kg(Some(Decimal::ONE), Some(Decimal::new(-150, 0))),
        None,
        "负幅宽不可算，必须 None"
    );
}

/// 产品查询出参真带派生键 meters_per_kg，且与幅宽克重两列的取值关系正确：
/// 两列为十进制字符串时给出向下截断一位小数的字符串值；缺任一键或非正时该键存在且恒为 null
/// （存在性确定、取值确定，前端不许因键缺席走本地兜底口径）。
/// 直测生产注入面 append_meters_per_kg（列表与详情两条出参链共用本函数）。
#[test]
fn product_output_carries_meters_per_kg_with_deterministic_shape() {
    use bingxi_backend::handlers::product_handler::append_meters_per_kg;

    let mut row = serde_json::json!({
        "code": "PRD-001",
        "width": "150.00",
        "gram_weight": "200.00"
    });
    append_meters_per_kg(&mut row);
    assert_eq!(
        row.get("meters_per_kg"),
        Some(&serde_json::json!("3.3")),
        "齐备行必须随出参下发派生键，值为向下截断一位小数的十进制字符串"
    );

    let mut missing_gram = serde_json::json!({ "code": "PRD-002", "width": "150.00" });
    append_meters_per_kg(&mut missing_gram);
    assert_eq!(
        missing_gram.get("meters_per_kg"),
        Some(&serde_json::json!(null)),
        "缺克重时键必须存在且为 null，不得缺席也不得落 0"
    );

    let mut missing_width = serde_json::json!({ "code": "PRD-003", "gram_weight": "200.00" });
    append_meters_per_kg(&mut missing_width);
    assert_eq!(
        missing_width.get("meters_per_kg"),
        Some(&serde_json::json!(null)),
        "缺幅宽时键必须存在且为 null"
    );

    let mut zero_width =
        serde_json::json!({ "code": "PRD-004", "width": "0.00", "gram_weight": "200.00" });
    append_meters_per_kg(&mut zero_width);
    assert_eq!(
        zero_width.get("meters_per_kg"),
        Some(&serde_json::json!(null)),
        "幅宽为 0（未录入形态）时键必须存在且为 null，不得把 0 当可算输入"
    );
}
