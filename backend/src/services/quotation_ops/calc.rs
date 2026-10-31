//! 报价单计算与校验 impl 子模块（quotation_ops/calc）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移计算与校验相关方法。
//! 包含 calculate_totals（金额计算）/ validate_create（业务校验）/ validate_price_terms（贸易术语校验）。

use rust_decimal::Decimal;

use crate::models::quotation_create_dto::{CreateQuotationDto, CreateQuotationItemDto};
use crate::services::quotation_service::{QuotationService, ServiceError};
use crate::utils::tax::{self, TaxDirectionError};

/// 报价单表头价税三值，按单据的含税标志选方向。
///
/// 功能，含税报价走除法反算加差额钩稽，不含税报价走净额事实的正算，两条腿都在
/// utils tax 内核里出，表头不再自己乘税率造含税。
/// 调用方，本模块 QuotationService::calculate_totals（建单腿）与
/// quotation_ops/update.rs 的 recalculate_totals_and_update（改单重算腿）。
/// 入参，net_sum 是明细行不含税金额合计、gross_sum 是明细行含税金额合计、
/// tax_inclusive 是该单的含税标志、tax_rate 是该单自身的税率（百分数口径）。
/// 传给谁，出参三元组由两条调用腿分别写入 sales_quotations 的 subtotal、
/// tax_amount、total_amount 列。存什么存哪里，本函数不落库。
///
/// 含税标志为真时，含税合计是录入事实，净额按 含税 除以 (1 加 税率) 反算、
/// 税额取含税减净额的差额，三值逐分钩稽；
/// 含税标志为假时，净额合计是录入事实，税额按净额正算、含税取两者相加，
/// 同样由内核一次构造，调用方拿不到可以各自舍入的两个数。
/// 税率缺省与越界、含税或净额为负都由 TaxDirectionError 点名，不静默也不兜底。
pub(crate) fn compute_quotation_header(
    net_sum: Decimal,
    gross_sum: Decimal,
    tax_inclusive: bool,
    tax_rate: Decimal,
) -> Result<(Decimal, Decimal, Decimal), TaxDirectionError> {
    let rate = Some(tax_rate);
    let split = if tax_inclusive {
        tax::split_from_inclusive(gross_sum, rate)?
    } else {
        tax::split_from_net(net_sum, rate)?
    };
    Ok((split.net, split.tax, split.inclusive))
}

impl QuotationService {
    /// 计算小计/税额/总金额
    ///
    /// 调用方是建单腿 create（出参进 build_new_quotation_active 写 sales_quotations 的
    /// subtotal、tax_amount、total_amount 三列）。入参 dto 的明细行金额口径与
    /// quotation_ops/crud.rs 落 sales_quotation_item 的 amount 与 amount_with_tax 同源，
    /// 即逐行收敛到分再合计，表头与明细不会因舍入次序不同而分裂。
    /// 换算方向与钩稽判据见本模块 compute_quotation_header。
    pub fn calculate_totals(
        &self,
        dto: &CreateQuotationDto,
    ) -> Result<(Decimal, Decimal, Decimal), ServiceError> {
        let net_sum: Decimal = dto
            .items
            .iter()
            .map(|i: &CreateQuotationItemDto| (i.quantity * i.unit_price).round_dp(2))
            .sum::<Decimal>()
            .round_dp(2);
        let gross_sum: Decimal = dto
            .items
            .iter()
            .map(|i: &CreateQuotationItemDto| (i.quantity * i.unit_price_with_tax).round_dp(2))
            .sum::<Decimal>()
            .round_dp(2);

        let (subtotal, tax_amount, total_amount) =
            compute_quotation_header(net_sum, gross_sum, dto.tax_inclusive, dto.tax_rate)
                .map_err(|e| ServiceError::Validation(e.to_string()))?;

        Ok((subtotal, tax_amount, total_amount))
    }

    /// 业务校验
    pub fn validate_create(&self, dto: &CreateQuotationDto) -> Result<(), ServiceError> {
        if dto.items.is_empty() {
            return Err(ServiceError::Validation("明细至少 1 条".to_string()));
        }
        if dto.valid_until < dto.quotation_date {
            return Err(ServiceError::Validation(
                "有效期截止必须不早于报价日期".to_string(),
            ));
        }
        // 批次 111 P1-2：接入 utils/incoterms.rs，用 Incoterms2020::from_code 解析+校验并记录业务元数据
        let incoterm = Self::validate_price_terms(&dto.price_terms)?;
        tracing::info!(
            incoterm_code = %dto.price_terms,
            incoterm_description = %incoterm.description(),
            includes_insurance = %incoterm.includes_insurance(),
            includes_freight = %incoterm.includes_freight(),
            requires_duty_paid = %incoterm.requires_duty_paid(),
            "报价单贸易术语已校验"
        );
        Ok(())
    }

    /// 校验价格条款（贸易术语）并返回解析后的 Incoterms2020 枚举
    /// 批次 111 P1-2：接入 utils/incoterms.rs，用 all()+code() 派生合法代码列表
    pub fn validate_price_terms(
        code: &str,
    ) -> Result<crate::utils::incoterms::Incoterms2020, ServiceError> {
        crate::utils::incoterms::Incoterms2020::from_code(code).map_err(|msg| {
            // 派生合法代码列表用于错误提示（同时使用 all() + code() 接入业务）
            let valid: Vec<&'static str> = crate::utils::incoterms::Incoterms2020::all()
                .iter()
                .map(|t| t.code())
                .collect();
            ServiceError::Validation(format!("{}（合法取值: {}）", msg, valid.join("/")))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tax::TAX_PERCENT_MAX;
    use std::str::FromStr;

    fn dec(literal: &str) -> Decimal {
        Decimal::from_str(literal)
            .unwrap_or_else(|e| panic!("测试内金额字面量必须可解析，实际 {literal}，错误 {e}"))
    }

    /// 表头三值的公共断言，逐分钩稽用存储精度（分位精确相等），不用浮点 epsilon。
    fn assert_header(
        net_sum: Decimal,
        gross_sum: Decimal,
        tax_inclusive: bool,
        tax_rate: Decimal,
        expected_net: &str,
        expected_tax: &str,
        expected_total: &str,
    ) {
        let (subtotal, tax_amount, total_amount) =
            compute_quotation_header(net_sum, gross_sum, tax_inclusive, tax_rate)
                .unwrap_or_else(|e| {
                    panic!(
                        "含税标志 {tax_inclusive} 净额 {net_sum} 含税 {gross_sum} 税率 {tax_rate} 是合法夹具，被拒绝的原因 {e}"
                    )
                });
        assert_eq!(
            subtotal,
            dec(expected_net),
            "表头小计必须等于期望净额，期望 {expected_net}，实际 {subtotal}"
        );
        assert_eq!(
            tax_amount,
            dec(expected_tax),
            "表头税额必须等于期望税额，期望 {expected_tax}，实际 {tax_amount}"
        );
        assert_eq!(
            total_amount,
            dec(expected_total),
            "表头价税合计必须等于期望含税，期望 {expected_total}，实际 {total_amount}"
        );
        assert_eq!(
            subtotal + tax_amount,
            total_amount,
            "表头三值钩稽破口，小计 {subtotal} 加税额 {tax_amount} 不等于价税合计 {total_amount}"
        );
    }

    /// 含税报价走除法方向，含税合计 1130 元按 13 个点得小计 1000 与税额 130。
    #[test]
    fn inclusive_quotation_header_divides_the_recorded_gross_total() {
        assert_header(
            dec("1000"),
            dec("1130"),
            true,
            dec("13"),
            "1000",
            "130",
            "1130",
        );
    }

    /// 含税报价在除不尽时税额取差额，1000 元含税按 13 个点得小计 884.96 与税额 115.04。
    #[test]
    fn inclusive_quotation_header_takes_tax_as_the_difference() {
        assert_header(
            dec("884.95"),
            dec("1000"),
            true,
            dec("13"),
            "884.96",
            "115.04",
            "1000",
        );
    }

    /// 含税标志为假时净额是事实，税额正算且含税等于两者相加，与含税腿共用同一次构造。
    #[test]
    fn exclusive_quotation_header_uses_the_net_fact() {
        assert_header(
            dec("1000"),
            dec("1130"),
            false,
            dec("13"),
            "1000",
            "130",
            "1130",
        );
        assert_header(
            dec("99.99"),
            dec("0"),
            false,
            dec("13"),
            "99.99",
            "13",
            "112.99",
        );
    }

    /// 税率 0 与 100 两档在两条腿上都走同一条公式，函数内不持有任何档位。
    #[test]
    fn header_handles_zero_and_hundred_percent_rates_on_both_legs() {
        assert_header(
            dec("1000"),
            dec("1000"),
            true,
            Decimal::ZERO,
            "1000",
            "0",
            "1000",
        );
        assert_header(
            dec("1000"),
            dec("0"),
            false,
            Decimal::ZERO,
            "1000",
            "0",
            "1000",
        );
        assert_header(dec("0"), dec("600"), true, dec("100"), "300", "300", "600");
        assert_header(dec("300"), dec("0"), false, dec("100"), "300", "300", "600");
    }

    /// 含税合计为负是非法载荷，点名拒绝并回显实际值，不静默折成 0。
    #[test]
    fn negative_inclusive_total_is_rejected_by_name() {
        let err = compute_quotation_header(dec("-100"), dec("-1130"), true, dec("13"))
            .expect_err("含税合计为负必须被拒绝");
        assert_eq!(
            err,
            TaxDirectionError::NegativeInclusiveAmount {
                inclusive: dec("-1130")
            },
            "必须出 NegativeInclusiveAmount 并回显含税合计，实际 {err:?}"
        );
        assert!(
            err.to_string().contains("-1130"),
            "拒绝文案必须带实际含税合计，实际文案 {err}"
        );
    }

    /// 税率越出落库列域在算表头之前拒绝，避免把非法载荷留到落库炸成列溢出。
    #[test]
    fn over_max_rate_is_rejected_before_any_amount_is_written() {
        let over = TAX_PERCENT_MAX + dec("0.01");
        let err = compute_quotation_header(dec("1000"), dec("1130"), true, over)
            .expect_err("越列域税率必须被拒绝");
        assert!(
            matches!(err, TaxDirectionError::TaxPercentOverMax { .. }),
            "必须出 TaxPercentOverMax，实际 {err:?}"
        );
        assert!(
            err.to_string().contains(&over.to_string()),
            "拒绝文案必须带实际税率 {over}，实际文案 {err}"
        );
    }

    /// 表头与明细同源，含税腿的价税合计必须逐分等于明细含税合计，不再塌成净额。
    #[test]
    fn inclusive_header_total_equals_the_sum_of_line_gross_amounts() {
        let gross_per_line = [dec("113"), dec("226"), dec("339")];
        let gross_sum: Decimal = gross_per_line.iter().copied().sum();
        let (subtotal, tax_amount, total_amount) =
            compute_quotation_header(dec("0"), gross_sum, true, dec("13"))
                .expect("多行含税合计夹具必须合法");
        assert_eq!(
            total_amount, gross_sum,
            "价税合计必须逐分等于明细含税合计 {gross_sum}，实际 {total_amount}"
        );
        assert_eq!(
            subtotal + tax_amount,
            total_amount,
            "表头三值钩稽破口，小计 {subtotal} 税额 {tax_amount} 合计 {total_amount}"
        );
        assert_eq!(
            (subtotal + tax_amount - total_amount).round_dp(tax::MONEY_SCALE),
            Decimal::ZERO,
            "钩稽差额必须为 0 分，实际小计 {subtotal} 税额 {tax_amount} 合计 {total_amount}"
        );
    }
}
