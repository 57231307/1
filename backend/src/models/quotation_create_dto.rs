//! 销售报价单创建 DTO
//!
//! 包含创建报价单所需的所有字段：主表、明细、贸易条款。
//! 设计依据：2026-06-16-sales-quotation-design.md

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 创建报价单请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateQuotationDto {
    pub customer_id: i64,
    pub sales_user_id: i64,
    pub quotation_date: NaiveDate,
    pub valid_until: NaiveDate,

    /// 货币
    pub currency: String,
    pub exchange_rate: Decimal,
    pub base_currency: String,

    /// 价格条款（Incoterms 2020：FOB / CIF / EXW / DDP / DAP）
    #[validate(length(min = 1, max = 20))]
    pub price_terms: String,
    pub incoterms_version: Option<String>,
    pub incoterm_location: Option<String>,

    /// 税务
    pub tax_inclusive: bool,
    pub tax_rate: Decimal,

    /// 业务参数
    pub moq: Option<Decimal>,
    pub lead_time_days: Option<i32>,
    pub customer_level: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 明细（1-100 条）
    #[validate(length(min = 1, max = 100))]
    pub items: Vec<CreateQuotationItemDto>,

    /// 贸易条款（可选）
    pub terms: Option<Vec<CreateQuotationTermDto>>,
}

/// 创建报价单明细 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateQuotationItemDto {
    pub product_id: i64,
    pub color_id: Option<i64>,
    pub specification: Option<String>,
    #[validate(length(min = 1, max = 20))]
    pub unit: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub unit_price_with_tax: Decimal,
    pub tier_pricing: Option<serde_json::Value>,
    pub discount_rate: Option<Decimal>,
    pub notes: Option<String>,
}

/// 创建报价单贸易条款 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateQuotationTermDto {
    /// 条款类型：logistics / payment / sample / inspection
    #[validate(length(min = 1, max = 50))]
    pub term_type: String,
    #[validate(length(min = 1, max = 100))]
    pub term_key: String,
    #[validate(length(min = 1))]
    pub term_value: String,
    pub sequence: i32,
}
