//! 面料多色号定价扩展 - 阶梯价 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-price-extension-design.md §4

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

// ----------------------------------------------------------------------
// 请求 DTO
// ----------------------------------------------------------------------

/// 创建阶梯价请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreatePriceTierDto {
    pub product_color_price_id: i64,

    pub min_quantity: Decimal,

    pub max_quantity: Option<Decimal>,

    pub tier_price: Decimal,

    /// 客户等级（NULL = 通用）
    pub customer_level: Option<String>,

    pub sequence: Option<i32>,
}

// ----------------------------------------------------------------------
// 响应 DTO
// ----------------------------------------------------------------------

/// 阶梯价响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceTierInfo {
    pub id: i64,
    pub product_color_price_id: i64,
    pub min_quantity: Decimal,
    pub max_quantity: Option<Decimal>,
    pub tier_price: Decimal,
    pub customer_level: Option<String>,
    pub sequence: i32,
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
