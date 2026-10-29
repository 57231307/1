//! 销售单据行金额与表头合计的单一算法来源。
//!
//! ## 功能
//! 把数量、单价、折扣率、税率四个入参折算成行级的成交额、折扣额、折后额、税额与含税额，
//! 再把若干行汇总成表头级四项合计。折算口径是「先按折扣率算折扣额，再在折后额上乘税率，
//! 每一步都定点舍入到两位」。
//!
//! ## 为什么收口在这里
//! 同一组数量与单价必须在销售订单与销售合同上算出同一个金额。舍入方向或精度一旦在两处
//! 各写一遍，任一处调整就会与另一处分裂，表现为订单合计与合同合计对不上且不报错。
//! 前端同样不得复刻本算法，只送数量与单价，金额一律由后端从这里折算。
//!
//! ## 调用方
//! 销售订单建单与改单的明细行折算与表头累计（services 下 so 域的 order_crud），
//! 以及销售合同由订单转生成时的行折算与表头累计（services 下 sales_contract_service
//! 的转单路径）。
//!
//! ## 入参与传给谁
//! 入参全部是定点小数，discount_percent 与 tax_percent 按百分数计（13 表示 13 个点）。
//! 行结果落到 sales_order_items 与 sales_contract_items 的金额列，表头合计落到
//! sales_orders 的四金额列与 sales_contracts 的 total_amount 列。
//!
//! ## 存什么存哪里
//! 纯计算模块，不落库、不查库、无副作用。

use rust_decimal::Decimal;

/// 单行金额折算结果。订单侧四列非空、合同侧四列可空，字段与两侧同名列一一对应
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SalesLineAmounts {
    /// 成交额，数量乘单价后舍入两位（订单列 subtotal，合同列 amount）
    pub subtotal: Decimal,
    /// 折扣率百分数入参原值，未传按 0（订单列 discount_percent，合同列 discount_percent）
    pub discount_percent: Decimal,
    /// 折扣金额（订单列 discount_amount，合同列 discount_amount）
    pub discount_amount: Decimal,
    /// 折后成交额，成交额减折扣金额（订单侧无独立列，税额以此为基数）
    pub amount_after_discount: Decimal,
    /// 税率百分数入参原值，未传按 0（订单列 tax_percent，合同列 tax_percent）
    pub tax_percent: Decimal,
    /// 税额，折后成交额乘税率（订单列 tax_amount，合同列 tax_amount）
    pub tax_amount: Decimal,
    /// 含税额，折后成交额加税额（订单列 total_amount，合同表头按行累加）
    pub total_amount: Decimal,
}

/// 表头合计。四项是逐行累加的成交额、折扣额、税额与含税额
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SalesAmountTotals {
    pub subtotal: Decimal,
    pub discount_amount: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,
}

/// 折算单行金额。
///
/// 入参 quantity 与 unit_price 必填，discount_percent 与 tax_percent 可选百分数。
/// 舍入口径与销售订单建单链路的既有实现逐字符一致，本模块只做收口，不改变任何取值。
pub fn calculate_line_amounts(
    quantity: Decimal,
    unit_price: Decimal,
    discount_percent: Option<Decimal>,
    tax_percent: Option<Decimal>,
) -> SalesLineAmounts {
    let percent_base = Decimal::new(100, 0);
    let discount_pct = discount_percent.unwrap_or(Decimal::ZERO);
    let tax_pct = tax_percent.unwrap_or(Decimal::ZERO);
    let subtotal = (quantity * unit_price).round_dp(2);
    let discount_amount = (subtotal * (discount_pct / percent_base)).round_dp(2);
    let amount_after_discount = (subtotal - discount_amount).round_dp(2);
    let tax_amount = (amount_after_discount * (tax_pct / percent_base)).round_dp(2);
    let total_amount = (amount_after_discount + tax_amount).round_dp(2);
    SalesLineAmounts {
        subtotal,
        discount_percent: discount_pct,
        discount_amount,
        amount_after_discount,
        tax_percent: tax_pct,
        tax_amount,
        total_amount,
    }
}

/// 逐行汇总表头合计。
///
/// 入参是已折算好的行结果集合，返回值传给订单表头四金额列与合同表头总金额列。
pub fn sum_line_amounts(lines: &[SalesLineAmounts]) -> SalesAmountTotals {
    let mut totals = SalesAmountTotals::default();
    for line in lines {
        totals.subtotal += line.subtotal;
        totals.discount_amount += line.discount_amount;
        totals.tax_amount += line.tax_amount;
        totals.total_amount += line.total_amount;
    }
    totals
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(literal: &str) -> Decimal {
        Decimal::from_str(literal).expect("测试内金额字面量必须可解析为 Decimal")
    }

    #[test]
    fn line_amounts_without_discount_and_tax_equals_subtotal() {
        let line = calculate_line_amounts(dec("100"), dec("12.5"), None, None);
        assert_eq!(line.subtotal, dec("1250"));
        assert_eq!(line.discount_amount, Decimal::ZERO);
        assert_eq!(line.amount_after_discount, dec("1250"));
        assert_eq!(line.tax_amount, Decimal::ZERO);
        assert_eq!(line.total_amount, dec("1250"));
    }

    #[test]
    fn line_amounts_taxes_the_discounted_base_not_the_gross() {
        let line = calculate_line_amounts(dec("100"), dec("10"), Some(dec("10")), Some(dec("13")));
        assert_eq!(line.discount_amount, dec("100"));
        assert_eq!(line.amount_after_discount, dec("900"));
        assert_eq!(line.tax_amount, dec("117"));
        assert_eq!(line.total_amount, dec("1017"));
    }

    #[test]
    fn line_amounts_round_each_step_so_no_tail_drift_accumulates() {
        let line = calculate_line_amounts(dec("3"), dec("7.777"), None, Some(dec("13")));
        assert_eq!(line.subtotal, dec("23.33"));
        assert_eq!(line.tax_amount, dec("3.03"));
        assert_eq!(line.total_amount, dec("26.36"));
    }

    #[test]
    fn totals_sum_lines_instead_of_recomputing_from_aggregates() {
        let lines = [
            calculate_line_amounts(dec("1"), dec("10"), None, Some(dec("13"))),
            calculate_line_amounts(dec("2"), dec("10"), Some(dec("5")), Some(dec("13"))),
        ];
        let totals = sum_line_amounts(&lines);
        assert_eq!(totals.subtotal, dec("30"));
        assert_eq!(totals.discount_amount, dec("1"));
        assert_eq!(totals.tax_amount, dec("3.77"));
        assert_eq!(totals.total_amount, dec("32.77"));
    }
}
