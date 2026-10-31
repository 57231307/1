//! 价税三值钩稽契约锁（契约锁族，纯计算不连库）
//!
//! 锁定的契约，含税金额是录入事实，净额只能由 含税 除以 (1 加 税率) 反算，
//! 税额取 含税 减 净额 的差额，并且 净额 加 税额 必须与含税按**存储精度**逐分相等。
//! 判定一律用 Decimal 的等值比较与 round_dp(MONEY_SCALE) 后的零差额比较，
//! 不用浮点 epsilon，也不得用 abs 小于某个容差 的写法放宽到位以下。
//! 覆盖面，税率 0 与 税率 100 与 缺税率 与 含税小于净额 与 多条明细行分别换算后求和
//! 对 单头直接换算 的差异，以及销售订单行与表头的同源关系。
//! 每条断言都必须判到具体数值与差额来源，禁止只断 `is_ok()`。

use bingxi_backend::services::so::amounts::{calculate_line_amounts, sum_line_amounts};
use bingxi_backend::utils::tax::{
    MONEY_SCALE, TAX_PERCENT_MAX, TaxDirectionError, TaxSplit, assert_three_way_reconcile,
    net_from_inclusive, split_from_inclusive, split_from_net, tax_from_inclusive,
};
use rust_decimal::Decimal;
use std::str::FromStr;

/// 夹具金额字面量，解析失败立即点名，不允许静默变成 0
fn dec(literal: &str) -> Decimal {
    Decimal::from_str(literal)
        .unwrap_or_else(|e| panic!("契约锁金额字面量必须可解析，实际 {literal}，错误 {e}"))
}

/// 按存储精度判三值钩稽，差额必须为 0 分，不用浮点 epsilon
fn assert_cent_exact(
    split: &TaxSplit,
    expected_net: &str,
    expected_tax: &str,
    expected_gross: &str,
) {
    assert_eq!(
        split.net,
        dec(expected_net),
        "净额必须等于除法反算并收敛到分的值，期望 {expected_net}，实际 {}",
        split.net
    );
    assert_eq!(
        split.tax,
        dec(expected_tax),
        "税额必须等于含税减净额的差额，期望 {expected_tax}，实际 {}",
        split.tax
    );
    assert_eq!(
        split.inclusive,
        dec(expected_gross),
        "含税必须等于收敛到分后的录入事实，期望 {expected_gross}，实际 {}",
        split.inclusive
    );
    let gap = (split.net + split.tax - split.inclusive).round_dp(MONEY_SCALE);
    assert_eq!(
        gap,
        Decimal::ZERO,
        "三值钩稽差额必须为 0 分，净额 {} 税额 {} 含税 {}",
        split.net,
        split.tax,
        split.inclusive
    );
}

/// 税率 0，免税腿走同一条除法并退化成等值，税额为 0 分而不是缺省档位
#[test]
fn zero_rate_still_divides_and_hooks_to_zero_cent_gap() {
    let split = split_from_inclusive(dec("1000.00"), Some(Decimal::ZERO))
        .expect("0 税率是合法的业务声明值，不该被拒绝");
    assert_cent_exact(&split, "1000.00", "0", "1000.00");
    assert_eq!(
        split.tax,
        Decimal::ZERO,
        "0 税率下税额必须是 0 分，不得取任何档位代算，实际 {}",
        split.tax
    );
}

/// 税率 100，因子为 2，含税 600 元必须劈成 300 与 300 并逐分钩稽
#[test]
fn hundred_percent_rate_splits_exactly_and_hooks() {
    let split = split_from_inclusive(dec("600"), Some(dec("100"))).expect("100 税率是合法夹具");
    assert_cent_exact(&split, "300", "300", "600");
    // 落库列域上界同样是一条合法税率，越界才拒绝
    let at_max = split_from_inclusive(dec("1000"), Some(TAX_PERCENT_MAX))
        .expect("列域上界是合法税率，不该被拒绝");
    assert!(
        at_max.net < at_max.inclusive,
        "列域上界税率下净额必须远低于含税，实际净额 {} 含税 {}",
        at_max.net,
        at_max.inclusive
    );
    let gap = (at_max.net + at_max.tax - at_max.inclusive).round_dp(MONEY_SCALE);
    assert_eq!(
        gap,
        Decimal::ZERO,
        "上界税率的三值钩稽差额必须为 0 分，实际 {gap}"
    );
}

/// 缺税率点名拒绝，三条入口都不得把缺键退化成 0 税率或任何档位
#[test]
fn missing_rate_is_rejected_by_every_inclusive_entry() {
    for (name, result) in [
        (
            "split_from_inclusive",
            split_from_inclusive(dec("1130"), None),
        ),
        ("net_from_inclusive", net_from_inclusive(dec("1130"), None)),
        ("tax_from_inclusive", tax_from_inclusive(dec("1130"), None)),
        ("split_from_net", split_from_net(dec("1000"), None)),
    ] {
        assert_eq!(
            result.err(),
            Some(TaxDirectionError::MissingTaxPercent),
            "{name} 在缺税率时必须出 MissingTaxPercent，不能按 0 税率静默算出净额"
        );
    }
    assert!(
        TaxDirectionError::MissingTaxPercent
            .to_string()
            .contains("tax_percent"),
        "缺税率文案必须带字段名，前端才能定位到录入项"
    );
}

/// 含税小于净额是价税矛盾，判定必须点名两侧值而不是静默取其中一侧
#[test]
fn inclusive_below_net_is_rejected_with_both_sides_named() {
    let err = assert_three_way_reconcile(dec("1000.00"), dec("130.00"), dec("999.99"))
        .expect_err("含税比净额小 1 分就是矛盾，必须拒绝");
    assert_eq!(
        err,
        TaxDirectionError::InclusiveBelowNet {
            net: dec("1000"),
            inclusive: dec("999.99"),
        },
        "必须出 InclusiveBelowNet 并回显两侧，实际 {err:?}"
    );
    let text = err.to_string();
    assert!(
        text.contains("1000") && text.contains("999.99"),
        "拒绝文案必须带净额与含税两个实际值，实际文案 {text}"
    );
}

/// 1 分钩稽破口按存储精度判红，差额必须原样回显，不得用 epsilon 放过
#[test]
fn one_cent_gap_is_red_at_storage_precision() {
    let err = assert_three_way_reconcile(dec("1000"), dec("130.01"), dec("1130"))
        .expect_err("税额多 1 分必须被拒绝");
    match err {
        TaxDirectionError::NotReconciled {
            net,
            tax,
            inclusive,
            gap,
        } => {
            assert_eq!(
                (net, tax, inclusive),
                (dec("1000"), dec("130.01"), dec("1130"))
            );
            assert_eq!(gap, dec("0.01"), "差额必须按分给出，实际 {gap}");
        }
        other => panic!("必须出 NotReconciled，实际 {other:?}"),
    }
    assert_eq!(
        assert_three_way_reconcile(dec("1000"), dec("130"), dec("1130")).map(|s| (
            s.net,
            s.tax,
            s.inclusive
        )),
        Ok((dec("1000"), dec("130"), dec("1130"))),
        "逐分相等的票面三值必须原样放行且不改任何取值"
    );
}

/// 多条明细行分别换算后求和 与 合计直接换算 可以差 1 分，两者必须都被钉住
/// 面料单据常见错源就是表头走一条口径、明细走另一条口径，这里钉死表头只能取行的和
#[test]
fn line_by_line_conversion_summed_differs_from_direct_total_conversion() {
    // 两行各含税 1000.00 元、税率 13 个点
    let mut net_sum = Decimal::ZERO;
    let mut tax_sum = Decimal::ZERO;
    let mut gross_sum = Decimal::ZERO;
    for _ in 0..2 {
        let split =
            split_from_inclusive(dec("1000"), Some(dec("13"))).expect("单行含税夹具必须合法");
        assert_cent_exact(&split, "884.96", "115.04", "1000");
        net_sum += split.net;
        tax_sum += split.tax;
        gross_sum += split.inclusive;
    }
    assert_eq!(
        net_sum,
        dec("1769.92"),
        "两行逐行反算的净额之和必须是 1769.92"
    );
    assert_eq!(tax_sum, dec("230.08"), "两行差额之和必须是 230.08");

    let direct = split_from_inclusive(gross_sum, Some(dec("13"))).expect("合计直接换算必须合法");
    assert_eq!(
        direct.net,
        dec("1769.91"),
        "合计直接换算是 1769.91，与逐行求和差 1 分，这正是两种口径必须二选一的根因"
    );
    assert_ne!(
        direct.net, net_sum,
        "夹具若不再能区分两种口径，本锁就失去意义，实际两者都是 {net_sum}"
    );
    // 表头取行的和才与明细逐分对上，且钩稽仍成立
    let header = assert_three_way_reconcile(net_sum, tax_sum, gross_sum)
        .expect("表头三值取自明细行求和时必须逐分钩稽");
    assert_cent_exact(&header, "1769.92", "230.08", "2000");
}

/// 销售订单行与表头同源，表头取行折算求和而不是对合计再算一次
#[test]
fn sales_order_header_equals_sum_of_line_conversions() {
    let lines = [
        calculate_line_amounts(dec("100"), dec("11.3"), None, Some(dec("13"))),
        calculate_line_amounts(dec("3"), dec("7.777"), Some(dec("10")), Some(dec("13"))),
        calculate_line_amounts(dec("1"), dec("0.01"), None, Some(Decimal::ZERO)),
    ];
    for line in &lines {
        let gap = (line.amount_after_discount + line.tax_amount - line.total_amount)
            .round_dp(MONEY_SCALE);
        assert_eq!(
            gap,
            Decimal::ZERO,
            "销售行折后额加税额必须逐分等于含税额，实际折后 {} 税额 {} 含税 {}",
            line.amount_after_discount,
            line.tax_amount,
            line.total_amount
        );
    }
    let totals = sum_line_amounts(&lines);
    let expect_subtotal: Decimal = lines.iter().map(|l| l.subtotal).sum();
    let expect_total: Decimal = lines.iter().map(|l| l.total_amount).sum();
    assert_eq!(
        totals.subtotal, expect_subtotal,
        "表头小计必须等于各行成交额之和，期望 {expect_subtotal}，实际 {}",
        totals.subtotal
    );
    assert_eq!(
        totals.total_amount, expect_total,
        "表头含税额必须等于各行含税额之和，期望 {expect_total}，实际 {}",
        totals.total_amount
    );
    let header_gap =
        (totals.subtotal - totals.discount_amount + totals.tax_amount) - totals.total_amount;
    assert_eq!(
        header_gap.round_dp(MONEY_SCALE),
        Decimal::ZERO,
        "表头三值钩稽破口，小计 {} 折扣 {} 税额 {} 合计 {}",
        totals.subtotal,
        totals.discount_amount,
        totals.tax_amount,
        totals.total_amount
    );
}

/// 舍入只做一次，含税原始尾差先收敛到分再除法，反算侧不得二次舍入
#[test]
fn rounding_happens_once_and_round_trip_is_stable() {
    let split = split_from_inclusive(dec("1130.004"), Some(dec("13"))).expect("尾差夹具应通过");
    assert_cent_exact(&split, "1000", "130", "1130");
    // 反算出的净额再正算回去必须回到同一含税事实，不被第二次舍入漂移
    let back = split_from_net(split.net, Some(dec("13"))).expect("正向回算应通过");
    assert_eq!(
        back.inclusive, split.inclusive,
        "净额正算回含税必须等于原含税事实，否则两次舍入已把票面推走，实际 {} 与 {}",
        back.inclusive, split.inclusive
    );
    // 分位级除法，税额取差额而不是独立舍入值，0.13 元含税是最小分歧样本
    let tiny = split_from_inclusive(dec("0.13"), Some(dec("13"))).expect("分位夹具应通过");
    assert_cent_exact(&tiny, "0.12", "0.01", "0.13");
    let independent_rounding = tax_from_inclusive(dec("0.13"), Some(dec("13"))).expect("同值入口");
    assert_eq!(
        independent_rounding, tiny.tax,
        "只取税额的入口必须与整套构造同值，实际 {independent_rounding} 与 {}",
        tiny.tax
    );
}
