//! 销售报价单更新 DTO
//!
//! 更新场景：仅在 draft / rejected 状态下允许修改主表字段。
//! 与 CreateQuotationDto 结构一致，但允许所有字段可选。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 更新报价单请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone, Default)]
pub struct UpdateQuotationDto {
    pub customer_id: Option<i64>,
    pub sales_user_id: Option<i64>,
    pub quotation_date: Option<NaiveDate>,
    pub valid_until: Option<NaiveDate>,

    pub currency: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub base_currency: Option<String>,

    #[validate(length(min = 1, max = 20))]
    pub price_terms: Option<String>,
    pub incoterms_version: Option<String>,
    pub incoterm_location: Option<String>,

    pub tax_inclusive: Option<bool>,
    pub tax_rate: Option<Decimal>,

    pub moq: Option<Decimal>,
    pub lead_time_days: Option<i32>,
    pub customer_level: Option<String>,

    pub notes: Option<String>,

    /// 全量替换明细时使用；为 None 时保留原明细
    pub items: Option<Vec<super::quotation_create_dto::CreateQuotationItemDto>>,
    pub terms: Option<Vec<super::quotation_create_dto::CreateQuotationTermDto>>,
}
