//! 面料多色号定价扩展 - 色号价格 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-price-extension-design.md §4

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

// ----------------------------------------------------------------------
// 请求 DTO
// ----------------------------------------------------------------------

/// 创建色号价格请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateColorPriceDto {
    /// 产品 ID
    pub product_id: i64,

    /// 色号 ID
    pub color_id: i64,

    /// 币种（CNY/USD/EUR）
    #[validate(length(min = 1, max = 10))]
    pub currency: String,

    /// 基础价
    pub base_price: Decimal,

    /// 生效日期
    pub effective_from: NaiveDate,

    /// 失效日期
    pub effective_to: Option<NaiveDate>,

    /// 客户等级（VIP/NORMAL/GOLD/SILVER）
    pub customer_level: Option<String>,

    /// 最小起订量
    pub min_quantity: Option<Decimal>,

    /// 阶梯价区间上限
    pub max_quantity: Option<Decimal>,

    /// 客户专属 ID（NULL = 通用）
    pub customer_id: Option<i64>,

    /// 季节（SS/AW/HOLIDAY）
    pub season: Option<String>,

    /// 优先级（数值大 = 优先级高）
    pub priority: Option<i32>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新色号价格请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UpdateColorPriceDto {
    pub currency: Option<String>,
    pub base_price: Option<Decimal>,
    pub effective_from: Option<NaiveDate>,
    pub effective_to: Option<NaiveDate>,
    pub customer_level: Option<String>,
    pub min_quantity: Option<Decimal>,
    pub max_quantity: Option<Decimal>,
    pub customer_id: Option<i64>,
    pub season: Option<String>,
    pub is_active: Option<bool>,
    pub priority: Option<i32>,
    pub notes: Option<String>,
}

/// 批量调价请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct BatchAdjustPriceDto {
    /// 调价明细
    #[validate(length(min = 1, max = 500))]
    pub items: Vec<BatchAdjustItem>,

    /// 调价原因
    pub change_reason: Option<String>,
}

/// 单条批量调价项
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BatchAdjustItem {
    /// 色号价格 ID
    pub price_id: i64,

    /// 调整方式：percentage（百分比）/ fixed（固定金额）
    pub adjustment_type: String,

    /// 调整值：+0.05 = 涨 5%，+1.5 = 加 1.5 元
    pub adjustment_value: Decimal,
}

/// 审批调价请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApproveColorPriceDto {
    /// 审批意见：APPROVED / REJECTED
    pub decision: String,

    /// 审批意见说明
    pub comments: Option<String>,
}

// ----------------------------------------------------------------------
// 查询 DTO
// ----------------------------------------------------------------------

/// 列表过滤查询
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListColorPricesQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i64>,
    pub color_id: Option<i64>,
    pub customer_id: Option<i64>,
    pub customer_level: Option<String>,
    pub season: Option<String>,
    pub currency: Option<String>,
    pub is_active: Option<bool>,
    pub approval_status: Option<String>,
    pub keyword: Option<String>,
}

// ----------------------------------------------------------------------
// 响应 DTO
// ----------------------------------------------------------------------

/// 色号价格列表项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorPriceListItem {
    pub id: i64,
    pub product_id: i64,
    pub color_id: i64,
    pub currency: String,
    pub base_price: Decimal,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub customer_level: Option<String>,
    pub min_quantity: Option<Decimal>,
    pub max_quantity: Option<Decimal>,
    pub customer_id: Option<i64>,
    pub season: Option<String>,
    pub is_active: bool,
    pub priority: i32,
    pub approval_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 色号价格详情
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorPriceDetail {
    pub id: i64,
    pub product_id: i64,
    pub color_id: i64,
    pub currency: String,
    pub base_price: Decimal,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub customer_level: Option<String>,
    pub min_quantity: Option<Decimal>,
    pub max_quantity: Option<Decimal>,
    pub customer_id: Option<i64>,
    pub season: Option<String>,
    pub is_active: bool,
    pub priority: i32,
    pub notes: Option<String>,
    pub created_by: Option<i64>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approval_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 批量调价结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchAdjustResult {
    /// 自动通过的调价 ID 列表
    pub auto_approved: Vec<i64>,
    /// 待审批的调价 ID 列表
    pub pending_approval: Vec<i64>,
    /// 总数
    pub total: usize,
}

/// 分页响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PagedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

// ----------------------------------------------------------------------
// 价格计算 DTO
// ----------------------------------------------------------------------

/// 价格计算请求
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PriceCalcRequest {
    pub product_id: i64,
    pub color_id: i64,
    pub customer_id: Option<i64>,
    pub customer_level: Option<String>,
    pub quantity: Decimal,
    pub season: Option<String>,
    pub product_category_id: Option<i64>,
    pub currency: String,
    pub calc_date: Option<NaiveDate>,
}

/// 价格计算结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceCalcResult {
    pub base_price: Decimal,
    pub tier_price: Option<Decimal>,
    pub level_price: Option<Decimal>,
    pub season_price: Option<Decimal>,
    pub special_price: Option<Decimal>,
    pub final_price: Decimal,
    pub currency: String,
    pub applied_rule: String,
    pub breakdown: Vec<PriceCalcStep>,
}

/// 单步计算明细
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceCalcStep {
    pub step: String,
    pub before: Decimal,
    pub after: Decimal,
    pub rule: String,
}
