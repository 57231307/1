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
#[test]
fn test_inspection_type_domain_excludes_the_prediction_table_vocabulary() {
    use bingxi_backend::handlers::quality_inspection_handler::validate_inspection_type;
    use bingxi_backend::models::status::quality_inspection_type;

    assert_eq!(
        quality_inspection_type::ALL,
        &[
            "incoming",
            "process",
            "finished",
            "outgoing",
            "outsourcing_receipt"
        ]
    );
    for value in quality_inspection_type::ALL {
        assert!(
            validate_inspection_type(value).is_ok(),
            "规范取值被拒绝：{value}"
        );
    }
    // inprocess / final 属 ai_quality_predictions 那张表的 CHECK 词表，不得用于本列
    for bad in ["inprocess", "final", "all", "进货检验", ""] {
        let err = validate_inspection_type(bad);
        assert!(err.is_err(), "别域写法被放行：{bad}");
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("incoming") && msg.contains("outsourcing_receipt"),
            "错误信息未列出合法值：{msg}"
        );
    }
}

/// 质检结论 → 入库单检验状态的映射（services/quality_inspection_service.rs 回写路径）
///
/// `purchase_receipt.inspection_status` 的取值域是大写码，而质检结论是中文，两列不同源。
/// 历史回写实现把中文结论原样复制进大写码列，令这些入库单在按大写码判断的读取方眼里
/// 等于"从未检验"；本用例钉住映射必须落在本列取值域内，且越界结论一律不静默映射。
#[test]
fn test_receipt_inspection_status_maps_within_its_own_domain() {
    use bingxi_backend::models::status::purchase_receipt_inspection;

    assert_eq!(
        purchase_receipt_inspection::ALL,
        &["PENDING", "PASSED", "REJECTED"]
    );
    for result in quality_inspection_result::ALL {
        let mapped = purchase_receipt_inspection::from_inspection_result(result);
        assert!(
            mapped.is_some(),
            "质检结论 {result} 竟无对应的入库单检验状态，回写路径会被整体拒掉"
        );
        assert!(
            purchase_receipt_inspection::ALL.contains(&mapped.unwrap()),
            "映射结果 {:?} 越出入库单检验状态取值域",
            mapped
        );
    }
    // 英文同义写法与带空白的值都不做静默映射（入口已按取值域校验，走到这里说明数据有问题）
    for bad in ["pass", " Pass", "qualified", "合 格", "合格 ", "", "冻结"] {
        assert!(
            purchase_receipt_inspection::from_inspection_result(bad).is_none(),
            "越界质检结论被静默映射：{bad}"
        );
    }
}
