//! 面料多色号定价扩展 - 季节调价规则 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-price-extension-design.md §4

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

// ----------------------------------------------------------------------
// 请求 DTO
// ----------------------------------------------------------------------

/// 创建季节调价规则请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateSeasonalRuleDto {
    #[validate(length(min = 1, max = 100))]
    pub rule_name: String,

    /// SS / AW / HOLIDAY
    #[validate(length(min = 1, max = 10))]
    pub season: String,

    pub product_category_id: Option<i64>,

    /// percentage / fixed
    #[validate(length(min = 1, max = 20))]
    pub adjustment_type: String,

    pub adjustment_value: Decimal,

    pub valid_from: NaiveDate,

    pub valid_until: Option<NaiveDate>,

    pub description: Option<String>,
}

/// 更新季节调价规则请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UpdateSeasonalRuleDto {
    pub rule_name: Option<String>,
    pub season: Option<String>,
    pub product_category_id: Option<i64>,
    pub adjustment_type: Option<String>,
    pub adjustment_value: Option<Decimal>,
    pub valid_from: Option<NaiveDate>,
    pub valid_until: Option<NaiveDate>,
    pub is_active: Option<bool>,
    pub description: Option<String>,
}

// ----------------------------------------------------------------------
// 查询 DTO
// ----------------------------------------------------------------------

/// 季节规则查询
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListSeasonalRulesQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub season: Option<String>,
    pub is_active: Option<bool>,
    pub product_category_id: Option<i64>,
}

// ----------------------------------------------------------------------
// 响应 DTO
// ----------------------------------------------------------------------

/// 季节调价规则响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeasonalRuleInfo {
    pub id: i64,
    pub rule_name: String,
    pub season: String,
    pub product_category_id: Option<i64>,
    pub adjustment_type: String,
    pub adjustment_value: Decimal,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub is_active: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
