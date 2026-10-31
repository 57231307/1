//! 报价单计算与校验 impl 子模块（quotation_ops/calc）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移计算与校验相关方法。
//! 包含 calculate_totals（金额计算）/ validate_create（业务校验）/ validate_price_terms（贸易术语校验），
//! 以及报价行与表头价税三值的唯一换算入口 compute_quotation_line 与 compute_quotation_header。

use rust_decimal::Decimal;

use crate::models::quotation_create_dto::CreateQuotationDto;
use crate::services::quotation_service::{QuotationService, ServiceError};
use crate::utils::tax::{self, TaxDirectionError};

/// 报价单单行的价税折算结果，两枚单价与三枚金额一次成套产出。
///
/// 功能，把一行报价的含税事实、净额事实与税额绑成一个不可拆的整体，
/// 行内只舍到位一次，调用方拿不到可以各自舍入的两个数。
/// 调用方，本模块 compute_quotation_line（出参）、
/// quotation_ops/crud.rs 的 insert_new_quotation_items（建单行写库）、
/// quotation_ops/update.rs 的 items_to_active_models 与
/// recalculate_totals_and_update（改单行写库与切换口径重算）。
/// 入参，由 compute_quotation_line 按单据含税标志与单据自身税率填入。
/// 传给谁，行结果分别写 sales_quotation_items 的 unit_price、unit_price_with_tax、
/// amount、amount_with_tax、discount_amount 列；表头合计由本模块再逐行累加。
/// 存什么存哪里，本结构体不落库，字段与上述列一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuotationLineAmounts {
    /// 不含税单价（含税报价下是除法反算的展示值，不含税报价下是录入事实）
    pub unit_price: Decimal,
    /// 含税单价（含税报价下是录入事实，不含税报价下是正算的展示值）
    pub unit_price_with_tax: Decimal,
    /// 行不含税净额
    pub net_amount: Decimal,
    /// 行税额
    pub tax_amount: Decimal,
    /// 行含税金额（价税合计）
    pub gross_amount: Decimal,
    /// 行折扣额，未录折扣率时为 None（列可空，空即未采集，不折成 0）
    pub discount_amount: Option<Decimal>,
}

/// 报价行价税折算的唯一入口，方向由该单的含税标志决定。
///
/// 功能，含税报价以含税单价为录入事实，行含税金额取 数量 乘 含税单价（两枚事实相乘），
/// 净额按 含税 除以 (1 加 税率) 反算、税额取差额，不含税单价同样除法反算；
/// 不含税报价以净单价为录入事实，行净额取 数量 乘 净单价，税额与含税由内核
/// split_from_net 一次构造，含税单价同样取该次构造的含税侧。
/// 两条腿都只经 crate::utils::tax 的入口出，本函数内不写任何税率公式。
/// 禁止的形态，含税单上用净价乘 (1 加 税率) 回算含税、不含税单上用含税除以因子再乘因子
/// 反复舍入、以及把税额写成净额乘税率后又独立舍入造成 净额 加 税额 不等于 含税。
/// 调用方，本模块 calculate_totals（建单表头的逐行基数）、
/// quotation_ops/crud.rs 的 insert_new_quotation_items、
/// quotation_ops/update.rs 的 items_to_active_models 与 recalculate_totals_and_update。
/// 入参，quantity 数量、unit_price 净单价、unit_price_with_tax 含税单价、
/// discount_rate 可选折扣率百分数、tax_inclusive 该单含税标志、
/// tax_percent 该单税率（百分数口径可选项，缺键点名拒绝，绝不退化成 0 也不取档位）。
/// 传给谁，出参 QuotationLineAmounts 由调用方逐列写 sales_quotation_items。
/// 存什么存哪里，本函数不落库。
///
/// 舍入口径，行金额侧只做一次收敛到分（含税腿先按 数量 乘 含税单价 收敛再除法反算，
/// 不含税腿先按 数量 乘 净单价 收敛再正算税额），单价侧另按自身一次收敛，
/// 金额三值之间不再有任何独立舍入，行内恒等式 净额 加 税额 等于 含税 逐分成立。
/// 单价与金额的朴素乘算关系（金额 等于 数量 乘 单价）在反算侧可能差 1 分，
/// 记账口径一律取行金额，单价只是展示口径，这是逐行舍入的必然结果而非缺陷。
pub(crate) fn compute_quotation_line(
    quantity: Decimal,
    unit_price: Decimal,
    unit_price_with_tax: Decimal,
    discount_rate: Option<Decimal>,
    tax_inclusive: bool,
    tax_percent: Option<Decimal>,
) -> Result<QuotationLineAmounts, TaxDirectionError> {
    // 先拦两侧单价自相矛盾的载荷，含税单价低于净单价在税率非负时不可能成立
    if tax_inclusive && unit_price > unit_price_with_tax {
        return Err(TaxDirectionError::InclusiveBelowNet {
            net: unit_price,
            inclusive: unit_price_with_tax,
        });
    }

    // 行金额腿，事实侧只乘一次（数量 乘 单价），另一侧一律交内核换算
    let split = if tax_inclusive {
        let gross_line = (quantity * unit_price_with_tax).round_dp(tax::MONEY_SCALE);
        tax::split_from_inclusive(gross_line, tax_percent)?
    } else {
        let net_line = (quantity * unit_price).round_dp(tax::MONEY_SCALE);
        tax::split_from_net(net_line, tax_percent)?
    };

    // 单价腿，与行金额同向同入口，展示单价不得另起一套公式
    let (unit_net, unit_gross) = if tax_inclusive {
        // 含税单价是录入事实，不含税单价由除法反算，禁止再乘因子
        let net = tax::net_from_inclusive(unit_price_with_tax, tax_percent)?;
        (net, unit_price_with_tax)
    } else {
        // 净单价是录入事实，含税单价取内核正算的含税侧
        (
            unit_price,
            tax::split_from_net(unit_price, tax_percent)?.inclusive,
        )
    };

    let discount_amount = match discount_rate {
        Some(rate) => Some(tax::percent_of(split.net, rate).round_dp(tax::MONEY_SCALE)),
        None => None,
    };

    Ok(QuotationLineAmounts {
        unit_price: unit_net,
        unit_price_with_tax: unit_gross,
        net_amount: split.net,
        tax_amount: split.tax,
        gross_amount: split.inclusive,
        discount_amount,
    })
}

/// 报价表头三值的钩稽判定，把逐行累加出的三个合计交内核核对。
///
/// 功能，表头小计与税额与价税合计一律取 明细行分别换算后的求和，不再对合计做一次
/// 独立换算；面料单常有多行，逐行舍入再求和与合计直接换算可以差 1 分，
/// 单据只有以行为准才能让明细与表头逐分对上，本函数就是这条口径的守门人。
/// 调用方，本模块 calculate_totals（建单腿）与 quotation_ops/update.rs 的
/// recalculate_totals_and_update（改单重算腿，入参取已落库的明细列合计）。
/// 入参，net_sum 行净额合计、tax_sum 行税额合计、gross_sum 行含税合计。
/// 传给谁，成功时出参三元组写 sales_quotations 的 subtotal、tax_amount、total_amount 列；
/// 失败时由调用方把 Display 原文外显（存量明细两列互矛盾时点名拒绝，不静默改哪一侧）。
/// 存什么存哪里，本函数不落库。
pub(crate) fn compute_quotation_header(
    net_sum: Decimal,
    tax_sum: Decimal,
    gross_sum: Decimal,
) -> Result<(Decimal, Decimal, Decimal), TaxDirectionError> {
    let split = tax::assert_three_way_reconcile(net_sum, tax_sum, gross_sum)?;
    Ok((split.net, split.tax, split.inclusive))
}

impl QuotationService {
    /// 计算小计/税额/总金额
    ///
    /// 调用方是建单腿 create（出参进 build_new_quotation_active 写 sales_quotations 的
    /// subtotal、tax_amount、total_amount 三列）。入参 dto 的明细行经
    /// compute_quotation_line 逐行折算，与 quotation_ops/crud.rs 写 sales_quotation_item
    /// 的 amount 与 amount_with_tax 两列共用同一入口，表头与明细不会因舍入次序不同而分裂。
    /// 出参三元组直接进主表三列，换算方向与钩稽判据见本模块 compute_quotation_line
    /// 与 compute_quotation_header。
    pub fn calculate_totals(
        &self,
        dto: &CreateQuotationDto,
    ) -> Result<(Decimal, Decimal, Decimal), ServiceError> {
        let mut net_sum = Decimal::ZERO;
        let mut tax_sum = Decimal::ZERO;
        let mut gross_sum = Decimal::ZERO;
        for item in &dto.items {
            let line = compute_quotation_line(
                item.quantity,
                item.unit_price,
                item.unit_price_with_tax,
                item.discount_rate,
                dto.tax_inclusive,
                Some(dto.tax_rate),
            )
            .map_err(|e| ServiceError::Validation(e.to_string()))?;
            net_sum += line.net_amount;
            tax_sum += line.tax_amount;
            gross_sum += line.gross_amount;
        }

        let (subtotal, tax_amount, total_amount) =
            compute_quotation_header(net_sum, tax_sum, gross_sum)
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

    /// 单行三值的公共断言，容差取存储精度（分位精确相等），不用浮点 epsilon。
    fn assert_line(
        line: &QuotationLineAmounts,
        expected_net_unit: &str,
        expected_gross_unit: &str,
        expected_net: &str,
        expected_tax: &str,
        expected_gross: &str,
    ) {
        assert_eq!(
            line.unit_price,
            dec(expected_net_unit),
            "不含税单价期望 {expected_net_unit}，实际 {}",
            line.unit_price
        );
        assert_eq!(
            line.unit_price_with_tax,
            dec(expected_gross_unit),
            "含税单价期望 {expected_gross_unit}，实际 {}",
            line.unit_price_with_tax
        );
        assert_eq!(
            line.net_amount,
            dec(expected_net),
            "行净额期望 {expected_net}，实际 {}",
            line.net_amount
        );
        assert_eq!(
            line.tax_amount,
            dec(expected_tax),
            "行税额期望 {expected_tax}，实际 {}",
            line.tax_amount
        );
        assert_eq!(
            line.gross_amount,
            dec(expected_gross),
            "行含税金额期望 {expected_gross}，实际 {}",
            line.gross_amount
        );
        assert_eq!(
            line.net_amount + line.tax_amount,
            line.gross_amount,
            "行三值钩稽破口，净额 {} 加税额 {} 不等于含税 {}",
            line.net_amount,
            line.tax_amount,
            line.gross_amount
        );
    }

    /// 正向，含税 100 米每米 11.30 元按 13 个点，行含税 1130 反算得净额 1000 与税额 130。
    #[test]
    fn inclusive_line_divides_the_recorded_gross_and_derives_the_net_unit() {
        let line = compute_quotation_line(
            dec("100"),
            dec("10"),
            dec("11.3"),
            None,
            true,
            Some(dec("13")),
        )
        .expect("含税报价行夹具不该被拒绝");
        assert_line(&line, "10", "11.3", "1000", "130", "1130");
    }

    /// 正向，不含税 100 米每米 10 元按 13 个点，行净额 1000 正算得税额 130 与含税 1130。
    #[test]
    fn exclusive_line_multiplies_only_the_net_fact_and_builds_gross_once() {
        let line = compute_quotation_line(
            dec("100"),
            dec("10"),
            dec("0"),
            None,
            false,
            Some(dec("13")),
        )
        .expect("不含税报价行夹具不该被拒绝");
        assert_line(&line, "10", "11.3", "1000", "130", "1130");
    }

    /// 切换口径不得改事实总额，同一张单把含税标志从假翻到真时，
    /// 含税单价一侧才是录入事实，净额必须重新除法反算而不是沿用旧乘法值。
    #[test]
    fn flipping_the_flag_rederives_net_from_the_inclusive_fact() {
        let exclusive = compute_quotation_line(
            dec("3"),
            dec("7.777"),
            dec("0"),
            None,
            false,
            Some(dec("13")),
        )
        .expect("不含税腿夹具应通过");
        assert_line(&exclusive, "7.78", "8.79", "23.33", "3.03", "26.36");

        let inclusive =
            compute_quotation_line(dec("3"), dec("0"), dec("8.79"), None, true, Some(dec("13")))
                .expect("含税腿夹具应通过");
        // 行含税 26.37 是事实，净额除法反算 23.34，税额取差额 3.03
        assert_line(&inclusive, "7.78", "8.79", "23.34", "3.03", "26.37");
        assert_eq!(
            inclusive.net_amount + inclusive.tax_amount,
            inclusive.gross_amount,
            "切换口径后行三值仍必须逐分钩稽"
        );
    }

    /// 税率 0 与 100 两条腿都走同一条公式，函数内不持有任何档位。
    #[test]
    fn lines_handle_zero_and_hundred_percent_rates_on_both_legs() {
        assert_line(
            &compute_quotation_line(dec("10"), dec("10"), dec("10"), None, true, Some(dec("0")))
                .expect("0 税率是合法声明值"),
            "10",
            "10",
            "100",
            "0",
            "100",
        );
        assert_line(
            &compute_quotation_line(dec("10"), dec("10"), dec("0"), None, false, Some(dec("0")))
                .expect("0 税率正向腿应通过"),
            "10",
            "10",
            "100",
            "0",
            "100",
        );
        assert_line(
            &compute_quotation_line(dec("2"), dec("0"), dec("600"), None, true, Some(dec("100")))
                .expect("100 税率含税腿应通过"),
            "300",
            "600",
            "600",
            "600",
            "1200",
        );
        assert_line(
            &compute_quotation_line(
                dec("2"),
                dec("300"),
                dec("0"),
                None,
                false,
                Some(dec("100")),
            )
            .expect("100 税率正向腿应通过"),
            "300",
            "600",
            "600",
            "600",
            "1200",
        );
    }

    /// 空税率点名拒绝，四枚入参组合（含税腿与不含税腿、行合计与单价）都不得退化成 0 税率。
    #[test]
    fn missing_rate_is_rejected_per_leg() {
        assert_eq!(
            compute_quotation_line(dec("100"), dec("10"), dec("11.3"), None, true, None),
            Err(TaxDirectionError::MissingTaxPercent),
            "含税腿缺税率必须出 MissingTaxPercent，不能按 0 税率把净额算成含税"
        );
        assert_eq!(
            compute_quotation_line(dec("100"), dec("10"), dec("0"), None, false, None),
            Err(TaxDirectionError::MissingTaxPercent),
            "不含税腿缺税率同样必须点名拒绝"
        );
        assert!(
            TaxDirectionError::MissingTaxPercent
                .to_string()
                .contains("tax_percent"),
            "拒绝文案必须带字段名，前端才可能定位到录入项"
        );
    }

    /// 含税单价低于净单价是价税矛盾（折扣不得写成单价），逐行点名拒绝并回显两侧。
    #[test]
    fn inclusive_unit_below_net_unit_is_rejected_at_the_line() {
        assert_eq!(
            compute_quotation_line(
                dec("10"),
                dec("12"),
                dec("11.3"),
                None,
                true,
                Some(dec("13"))
            ),
            Err(TaxDirectionError::InclusiveBelowNet {
                net: dec("12"),
                inclusive: dec("11.3"),
            }),
            "含税单价低于净单价必须出 InclusiveBelowNet，不得静默把净额当含税"
        );
    }

    /// 行含税金额为负与行净额为负分别点名，红冲须由单据的负向数量表达而不是负单价。
    #[test]
    fn negative_line_amounts_are_rejected_by_name() {
        assert_eq!(
            compute_quotation_line(
                dec("-1"),
                dec("10"),
                dec("11.3"),
                None,
                true,
                Some(dec("13"))
            ),
            Err(TaxDirectionError::NegativeInclusiveAmount {
                inclusive: dec("-11.3")
            }),
            "数量为负使行含税为负，必须出 NegativeInclusiveAmount"
        );
        assert_eq!(
            compute_quotation_line(
                dec("10"),
                dec("-11.3"),
                dec("0"),
                None,
                false,
                Some(dec("13"))
            ),
            Err(TaxDirectionError::NegativeNetAmount { net: dec("-113") }),
            "净单价为负必须出 NegativeNetAmount"
        );
    }

    /// 税率越出落库列域在算行金额之前拒绝，避免把非法载荷留到落库炸成列溢出。
    #[test]
    fn over_max_rate_is_rejected_before_any_line_amount() {
        let over = TAX_PERCENT_MAX + dec("0.01");
        let err =
            compute_quotation_line(dec("100"), dec("10"), dec("11.3"), None, true, Some(over))
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

    /// 折扣额取行净额为基，与含税标志无关，未录折扣率的行留 None 不折成 0。
    #[test]
    fn discount_is_taken_from_the_line_net_and_stays_none_when_absent() {
        let with_discount = compute_quotation_line(
            dec("100"),
            dec("10"),
            dec("11.3"),
            Some(dec("5")),
            true,
            Some(dec("13")),
        )
        .expect("带折扣率的含税行应通过");
        assert_eq!(
            with_discount.discount_amount,
            Some(dec("50")),
            "折扣额必须是行净额 1000 的 5 个点，实际 {:?}",
            with_discount.discount_amount
        );
        let without = compute_quotation_line(
            dec("100"),
            dec("10"),
            dec("11.3"),
            None,
            true,
            Some(dec("13")),
        )
        .expect("无折扣行应通过");
        assert_eq!(
            without.discount_amount, None,
            "未录折扣率必须留空，不得折成 0 与合法零折扣混淆"
        );
    }

    /// 表头取逐行换算后的求和，1 分尾差留在明细而不是被合计直接换算抹掉。
    #[test]
    fn header_sums_line_conversions_instead_of_reconverting_the_total() {
        // 两行各含税 1000 元按 13 个点，逐行反算净额 884.96 与税额 115.04
        let mut net_sum = Decimal::ZERO;
        let mut tax_sum = Decimal::ZERO;
        let mut gross_sum = Decimal::ZERO;
        for _ in 0..2 {
            let line = compute_quotation_line(
                dec("1"),
                dec("0"),
                dec("1000"),
                None,
                true,
                Some(dec("13")),
            )
            .expect("单行含税夹具应通过");
            assert_line(&line, "884.96", "1000", "884.96", "115.04", "1000");
            net_sum += line.net_amount;
            tax_sum += line.tax_amount;
            gross_sum += line.gross_amount;
        }
        let (subtotal, tax_amount, total_amount) =
            compute_quotation_header(net_sum, tax_sum, gross_sum).expect("逐行合计必须钩稽");
        assert_eq!(
            subtotal,
            dec("1769.92"),
            "表头小计取两行反算值之和 1769.92，不得改成合计直接除法的 1769.91"
        );
        assert_eq!(tax_amount, dec("230.08"), "表头税额取两行差额之和");
        assert_eq!(
            total_amount,
            dec("2000"),
            "表头价税合计必须逐分等于两行含税合计"
        );
        assert_eq!(
            subtotal + tax_amount,
            total_amount,
            "表头三值钩稽破口，小计 {subtotal} 加税额 {tax_amount} 不等于合计 {total_amount}"
        );
        // 对照，合计直接换算会得 1769.91 与 230.09，与明细行求和差 1 分，钉住两口径不同值
        let direct = tax::split_from_inclusive(gross_sum, Some(dec("13"))).expect("合计换算应通过");
        assert_ne!(
            direct.net, subtotal,
            "合计直接换算 与 逐行换算后求和 本就该差 1 分，二者相等说明夹具失效"
        );
        assert_eq!(
            direct.net,
            dec("1769.91"),
            "对照值必须钉死，实际 {}",
            direct.net
        );
    }

    /// 表头守门判定，存量明细两列互矛盾时点名拒绝而不是择一侧静默改写。
    #[test]
    fn header_gate_rejects_inconsistent_stored_line_sums() {
        assert_eq!(
            compute_quotation_header(dec("1000"), dec("130"), dec("1131")),
            Err(TaxDirectionError::NotReconciled {
                net: dec("1000"),
                tax: dec("130"),
                inclusive: dec("1131"),
                gap: dec("-1"),
            }),
            "明细净额加税额与含税差 1 分必须被表头守门拒绝"
        );
        assert_eq!(
            compute_quotation_header(dec("1000"), dec("130"), dec("999")),
            Err(TaxDirectionError::InclusiveBelowNet {
                net: dec("1000"),
                inclusive: dec("999"),
            }),
            "合计含税低于净额必须出 InclusiveBelowNet，不得静默把净额当含税"
        );
    }

    /// 只舍一次，行含税原始尾差在收敛到分之后才进除法。
    #[test]
    fn line_amounts_round_once_at_money_scale() {
        let line = compute_quotation_line(
            dec("3"),
            dec("0"),
            dec("376.667"),
            None,
            true,
            Some(dec("13")),
        )
        .expect("尾差夹具应通过");
        // 3 乘 376.667 等于 1130.001，先收敛到 1130.00 再除法反算；
        // 展示单价按自身收敛到分后反算得 333.34，与行净额 1000 保持同一方向
        assert_line(&line, "333.34", "376.667", "1000", "130", "1130");
        assert_eq!(
            tax::MONEY_SCALE,
            2,
            "存储口径位数必须是 2 分，改动位数要同步全部单据列"
        );
    }
}
