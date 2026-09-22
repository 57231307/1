//! 质量检验记录结论取值校验（handlers/quality_inspection_handler.rs::validate_inspection_result）
//!
//! `quality_inspection_records.inspection_result` 是无 CHECK 的 VARCHAR，历史上同时收过
//! 界面自造的 pass/fail/pending 与自动写入方（委外收回单确认回仓）落的中文结论，
//! 两套写法混在同一列会让按结论筛选与合格率统计静默失真。
//! 本用例钉住归一后的词表、越界拒绝，以及错误信息必须列出合法值。

use bingxi_backend::handlers::quality_inspection_handler::validate_inspection_result;
use bingxi_backend::models::status::quality_inspection_result;

#[test]
fn test_result_values_are_the_written_domain() {
    assert_eq!(quality_inspection_result::ALL, &["待检", "合格", "不合格"]);
}

#[test]
fn test_validate_accepts_every_canonical_value() {
    for value in quality_inspection_result::ALL {
        assert!(
            validate_inspection_result(value).is_ok(),
            "规范取值被拒绝：{value}"
        );
    }
}

#[test]
fn test_validate_rejects_other_spellings_and_lists_allowed_values() {
    // pass/fail/pending 是界面自造码；qualified 属委外收回单结论域；空白与带空格都不放行
    for bad in [
        "pass",
        "fail",
        "pending",
        "qualified",
        "unqualified",
        "合 格",
        " 合格",
        "",
    ] {
        let err = validate_inspection_result(bad);
        assert!(err.is_err(), "越界结论被放行：{bad}");
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("待检") && msg.contains("合格") && msg.contains("不合格"),
            "错误信息未列出合法值，调用方无法自纠：{msg}"
        );
    }
}
