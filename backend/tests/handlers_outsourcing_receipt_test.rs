//! 委外收回单质检结论取值校验（services/outsourcing_ops/receipt.rs）
//!
//! 该列历史上同时收过四套写法（界面 qualified/concession/unqualified、用例 passed、
//! 模型注释 passed/failed、库存域的中文「合格」），而确认回仓只认字符串 "qualified"，
//! 于是 `passed` 的收回单在确认时被整体判成不合格并生成「不合格」质检记录。
//! 本用例钉住归一后的取值域与越界拒绝，防止别域字面量再次流入。

use bingxi_backend::models::status::outsourcing_receipt_quality_status as quality_status;
use bingxi_backend::services::outsourcing_ops::receipt::validate_receipt_quality_status;

#[test]
fn test_quality_status_values_are_the_normalized_set() {
    assert_eq!(
        quality_status::ALL,
        &["pending", "qualified", "concession", "unqualified"]
    );
}

#[test]
fn test_validate_accepts_only_canonical_values() {
    for value in quality_status::ALL {
        assert_eq!(
            validate_receipt_quality_status(value).unwrap(),
            *value,
            "规范取值应原样通过校验"
        );
    }
}

#[test]
fn test_validate_rejects_other_domains_spellings() {
    // passed / failed 属染化料来料检验域，中文属库存质量状态域，均不得写入本列
    for bad in [
        "passed",
        "failed",
        "quarantine",
        "合格",
        "不合格",
        "待检",
        "",
    ] {
        let err = validate_receipt_quality_status(bad);
        assert!(err.is_err(), "别域写法被放行：{bad}");
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("qualified")
                && msg.contains("concession")
                && msg.contains("unqualified")
                && msg.contains("pending"),
            "错误信息未列出合法值，调用方无法自纠：{msg}"
        );
    }
}
