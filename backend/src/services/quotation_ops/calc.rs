//! 报价单计算与校验 impl 子模块（quotation_ops/calc）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移计算与校验相关方法。
//! 包含 calculate_totals（金额计算）/ validate_create（业务校验）/ validate_price_terms（贸易术语校验）。

use rust_decimal::Decimal;

use crate::models::quotation_create_dto::{CreateQuotationDto, CreateQuotationItemDto};
use crate::services::quotation_service::{QuotationService, ServiceError};

impl QuotationService {
    /// 计算小计/税额/总金额
    pub(crate) fn calculate_totals(
        &self,
        dto: &CreateQuotationDto,
    ) -> Result<(Decimal, Decimal, Decimal), ServiceError> {
        // 批次 87：金额计算补 round_dp(2) 精度归一化
        let subtotal: Decimal = dto
            .items
            .iter()
            .map(|i: &CreateQuotationItemDto| (i.quantity * i.unit_price).round_dp(2))
            .sum::<Decimal>()
            .round_dp(2);

        let tax_amount = if dto.tax_inclusive {
            // 含税：报价单小计已含税，差额为 0
            Decimal::ZERO
        } else {
            // 不含税：税额 = 小计 * 税率
            (subtotal * dto.tax_rate / Decimal::from(100)).round_dp(2)
        };

        let total_amount = (subtotal + tax_amount).round_dp(2);
        Ok((subtotal, tax_amount, total_amount))
    }

    /// 业务校验
    pub(crate) fn validate_create(&self, dto: &CreateQuotationDto) -> Result<(), ServiceError> {
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
    pub(crate) fn validate_price_terms(
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
