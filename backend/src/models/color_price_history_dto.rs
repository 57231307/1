//! 面料多色号定价扩展 - 价格历史 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-price-extension-design.md §4

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ----------------------------------------------------------------------
// 响应 DTO
// ----------------------------------------------------------------------

/// 价格历史项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceHistoryItem {
    pub id: i64,
    pub product_color_price_id: i64,
    pub old_price: Decimal,
    pub new_price: Decimal,
    pub currency: String,
    pub change_type: String,
    pub change_reason: Option<String>,
    pub change_percent: Option<Decimal>,
    pub quantity: Option<Decimal>,
    pub operated_by: i64,
    pub operated_at: DateTime<Utc>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
}

/// 价格历史查询请求
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PriceHistoryQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i64>,
    pub color_id: Option<i64>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub change_type: Option<String>,
}
