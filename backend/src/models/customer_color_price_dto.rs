//! 面料多色号定价扩展 - 客户专属价 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-price-extension-design.md §4

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

// ----------------------------------------------------------------------
// 请求 DTO
// ----------------------------------------------------------------------

/// 创建客户专属价请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateCustomerColorPriceDto {
    pub customer_id: i64,
    pub product_id: i64,
    pub color_id: i64,
    pub special_price: Decimal,
    pub discount_percent: Option<Decimal>,
    pub currency: String,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub notes: Option<String>,
}

// ----------------------------------------------------------------------
// 查询 DTO
// ----------------------------------------------------------------------

/// 客户专属价查询
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListCustomerColorPricesQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub customer_id: Option<i64>,
    pub product_id: Option<i64>,
    pub color_id: Option<i64>,
    pub active_only: Option<bool>,
}

// ----------------------------------------------------------------------
// 响应 DTO
// ----------------------------------------------------------------------

/// 客户专属价响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomerColorPriceInfo {
    pub id: i64,
    pub customer_id: i64,
    pub product_id: i64,
    pub color_id: i64,
    pub special_price: Decimal,
    pub discount_percent: Option<Decimal>,
    pub currency: String,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub notes: Option<String>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
