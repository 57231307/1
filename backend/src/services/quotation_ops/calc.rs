//! 报价单计算与校验 impl 子模块（quotation_ops/calc）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移计算与校验相关方法。
//! 包含 quotation_line_amounts（行价税分离）/ calculate_totals（金额计算）/
//! validate_create（业务校验）/ validate_price_terms（贸易术语校验）。
//!
//! 计算方向口径（与 utils tax 内核同源，本模块不另写一套）：
//! 报价行的**含税单价是录入事实**，净额一律由 `含税 ÷ (1 + 税率% ÷ 100)` 反算，
//! 税额一律由 `含税 − 净额` 取差额，恒等式 `净额 + 税额 == 含税` 由构造保证。
//! 本模块不存在 `× (1 + 税率)` 或 `× 税率` 的正向公式，也不按 tax_inclusive
//! 分叉出第二条计算腿（含税与免税走同一枚除法，0 税率退化为等值）。

use rust_decimal::Decimal;

use crate::models::quotation_create_dto::{CreateQuotationDto, CreateQuotationItemDto};
use crate::services::quotation_service::{QuotationService, ServiceError};
use crate::utils::tax;

/// 报价行价税分离结果。
///
/// 出参逐字段的落库去向是 sales_quotation_items 的同名列
/// （unit_price、unit_price_with_tax、amount、amount_with_tax、discount_amount），
/// 其中 unit_price 与 amount 是反算派生值，unit_price_with_tax 与 amount_with_tax
/// 是录入事实与事实聚合，税额只随主表 tax_amount 列（表头汇总）落库，行上不留第三份。
pub(crate) struct QuotationLineAmounts {
    /// 不含税单价（由含税单价反算，派生只读）
    pub unit_price: Decimal,
    /// 含税单价（录入事实，原样保留）
    pub unit_price_with_tax: Decimal,
    /// 不含税金额（由成交含税额反算）
    pub amount: Decimal,
    /// 成交含税额（标价含税额扣掉行折扣，落 amount_with_tax 列）
    pub amount_with_tax: Decimal,
    /// 行折扣额（含税口径，落 discount_amount 列）
    pub discount_amount: Decimal,
}

impl QuotationService {
    /// 报价行的价税分离单源算法。
    ///
    /// 入参，item 是提交上来的报价行（含税单价是事实，quantity 是数量，
    /// discount_rate 是行折扣百分数，可空表示不打折）、tax_percent 是本张报价单的税率
    /// （百分数口径，来自单据自身，缺来源由 DTO 反序列化阶段拒绝，本函数不做任何代填）。
    /// 传给谁：create_draft 的行插入腿、update 的行重建腿与表头 calculate_totals 汇总腿，
    /// 三处共用同一枚函数，禁止第二套公式（否则同一张报价单在保存与改单时算出两个数）。
    /// 存什么：返回的四个金额/单价字段分别写入 sales_quotation_items 的四列。
    pub(crate) fn quotation_line_amounts(
        item: &CreateQuotationItemDto,
        tax_percent: Decimal,
    ) -> QuotationLineAmounts {
        // 标价含税额 = 录入含税单价 × 数量（这是事实的聚合，税率不进算式）
        let list_inclusive = tax::inclusive_amount(item.unit_price_with_tax, item.quantity);
        // 行折扣按含税成交额打折（纺织内贸报价口径），折扣百分数可空 = 不打折
        let discount_amount = match item.discount_rate {
            Some(rate) => (list_inclusive * rate / Decimal::from(100)).round_dp(2),
            None => Decimal::ZERO,
        };
        // 成交含税额才是本行的含税事实（折扣后的应开票金额），净额与税额都由它反算
        let amount_with_tax = (list_inclusive - discount_amount).round_dp(2);
        let amount = tax::net_from_inclusive(amount_with_tax, tax_percent);
        QuotationLineAmounts {
            unit_price: tax::net_from_inclusive(item.unit_price_with_tax, tax_percent),
            unit_price_with_tax: item.unit_price_with_tax,
            amount,
            amount_with_tax,
            discount_amount,
        }
    }

    /// 计算报价单表头的 净额小计/税额/含税总额。
    ///
    /// 入参 dto 是报价单创建或整体更新载荷，逐行走 [`quotation_line_amounts`] 单源算法。
    /// 出参三元组写入 sales_quotations 的 subtotal（不含税）、tax_amount（税额）、
    /// total_amount（含税）三列，其中税额取「含税合计 − 净额合计」的差额，
    /// 因此 subtotal + tax_amount 逐分等于 total_amount，不出现两处各自舍入的 1 分差。
    /// tax_inclusive 只声明本单是否含税计价，不再决定计算方向（0 税率行与免税行走
    /// 同一枚除法反算），本函数内不存在任何乘税率造税额的正向公式。
    pub fn calculate_totals(
        &self,
        dto: &CreateQuotationDto,
    ) -> Result<(Decimal, Decimal, Decimal), ServiceError> {
        let mut subtotal = Decimal::ZERO;
        let mut total_amount = Decimal::ZERO;
        for item in &dto.items {
            let line = Self::quotation_line_amounts(item, dto.tax_rate);
            subtotal += line.amount;
            total_amount += line.amount_with_tax;
        }
        let subtotal = subtotal.round_dp(2);
        let total_amount = total_amount.round_dp(2);
        // 税额取差额，不重算：合计减合计，逐分钩稽
        let tax_amount = (total_amount - subtotal).round_dp(2);
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
