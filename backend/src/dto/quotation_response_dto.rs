//! 销售报价单响应 DTO
//!
//! API 返回给前端的数据结构，扁平化嵌套字段。
//! 设计依据：2026-06-16-sales-quotation-design.md

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::sales_quotation;
use crate::models::sales_quotation_item;
use crate::models::sales_quotation_term;

/// 报价单响应 DTO
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QuotationResponseDto {
    pub id: i64,
    pub quotation_no: String,
    pub customer_id: i64,
    pub sales_user_id: i64,
    pub quotation_date: NaiveDate,
    pub valid_until: NaiveDate,

    pub currency: String,
    pub exchange_rate: Decimal,
    pub base_currency: String,

    pub price_terms: String,
    pub incoterms_version: Option<String>,
    pub incoterm_location: Option<String>,

    pub tax_inclusive: bool,
    pub tax_rate: Decimal,

    pub moq: Option<Decimal>,
    pub lead_time_days: Option<i32>,
    pub customer_level: Option<String>,

    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,

    pub status: String,
    pub approval_instance_id: Option<i64>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,

    pub converted_sales_order_id: Option<i64>,
    pub converted_at: Option<DateTime<Utc>>,

    pub notes: Option<String>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub items: Vec<QuotationItemResponseDto>,
    pub terms: Vec<QuotationTermResponseDto>,
}

impl From<sales_quotation::Model> for QuotationResponseDto {
    fn from(m: sales_quotation::Model) -> Self {
        Self {
            id: m.id,
            quotation_no: m.quotation_no,
            customer_id: m.customer_id,
            sales_user_id: m.sales_user_id,
            quotation_date: m.quotation_date,
            valid_until: m.valid_until,
            currency: m.currency,
            exchange_rate: m.exchange_rate,
            base_currency: m.base_currency,
            price_terms: m.price_terms,
            incoterms_version: m.incoterms_version,
            incoterm_location: m.incoterm_location,
            tax_inclusive: m.tax_inclusive,
            tax_rate: m.tax_rate,
            moq: m.moq,
            lead_time_days: m.lead_time_days,
            customer_level: m.customer_level,
            subtotal: m.subtotal,
            tax_amount: m.tax_amount,
            total_amount: m.total_amount,
            status: m.status,
            approval_instance_id: m.approval_instance_id,
            approved_by: m.approved_by,
            approved_at: m.approved_at,
            rejection_reason: m.rejection_reason,
            converted_sales_order_id: m.converted_sales_order_id,
            converted_at: m.converted_at,
            notes: m.notes,
            created_by: m.created_by,
            created_at: m.created_at,
            updated_at: m.updated_at,
            items: Vec::new(),
            terms: Vec::new(),
        }
    }
}

/// 报价单明细响应 DTO
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QuotationItemResponseDto {
    pub id: i64,
    pub quotation_id: i64,
    pub product_id: i64,
    pub color_id: Option<i64>,
    pub color_code: Option<String>,
    pub pantone_code: Option<String>,
    pub cncs_code: Option<String>,
    pub specification: Option<String>,
    pub unit: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub unit_price_with_tax: Decimal,
    pub amount: Decimal,
    pub amount_with_tax: Decimal,
    pub tier_pricing: Option<serde_json::Value>,
    pub discount_rate: Option<Decimal>,
    pub discount_amount: Option<Decimal>,
    pub notes: Option<String>,
    pub sequence: i32,
}

impl From<sales_quotation_item::Model> for QuotationItemResponseDto {
    fn from(m: sales_quotation_item::Model) -> Self {
        Self {
            id: m.id,
            quotation_id: m.quotation_id,
            product_id: m.product_id,
            color_id: m.color_id,
            color_code: m.color_code,
            pantone_code: m.pantone_code,
            cncs_code: m.cncs_code,
            specification: m.specification,
            unit: m.unit,
            quantity: m.quantity,
            unit_price: m.unit_price,
            unit_price_with_tax: m.unit_price_with_tax,
            amount: m.amount,
            amount_with_tax: m.amount_with_tax,
            tier_pricing: m.tier_pricing.map(|j| j.into()),
            discount_rate: m.discount_rate,
            discount_amount: m.discount_amount,
            notes: m.notes,
            sequence: m.sequence,
        }
    }
}

/// 报价单贸易条款响应 DTO
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QuotationTermResponseDto {
    pub id: i64,
    pub quotation_id: i64,
    pub term_type: String,
    pub term_key: String,
    pub term_value: String,
    pub sequence: i32,
}

impl From<sales_quotation_term::Model> for QuotationTermResponseDto {
    fn from(m: sales_quotation_term::Model) -> Self {
        Self {
            id: m.id,
            quotation_id: m.quotation_id,
            term_type: m.term_type,
            term_key: m.term_key,
            term_value: m.term_value,
            sequence: m.sequence,
        }
    }
}
