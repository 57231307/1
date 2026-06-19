//! 定制订单创建 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 创建定制订单请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateCustomOrderDto {
    pub customer_id: i64,
    pub product_id: i64,
    pub color_id: Option<i64>,

    /// 规格
    #[validate(length(min = 1, max = 200))]
    pub spec: String,

    /// 数量（必须 > 0）
    pub quantity: Decimal,

    /// 单位（m / kg / pcs）
    #[validate(length(min = 1, max = 20))]
    pub unit: String,

    /// 客户定制要求（JSONB）
    pub custom_requirements: Option<serde_json::Value>,

    /// 纱线规格
    pub yarn_spec: Option<String>,

    /// 染色方法
    pub dye_method: Option<String>,

    /// 后整理方法
    pub finishing_method: Option<String>,

    /// 期望交付日期
    pub expected_delivery_date: Option<NaiveDate>,

    /// 关联销售订单 ID（从销售订单转定制订单时使用）
    pub sales_order_id: Option<i64>,

    /// 金额
    pub total_amount: Option<Decimal>,

    /// 币种
    pub currency: Option<String>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新定制订单 DTO（仅草稿状态可更新）
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UpdateCustomOrderDto {
    pub spec: Option<String>,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub custom_requirements: Option<serde_json::Value>,
    pub yarn_spec: Option<String>,
    pub dye_method: Option<String>,
    pub finishing_method: Option<String>,
    pub expected_delivery_date: Option<NaiveDate>,
    pub total_amount: Option<Decimal>,
    pub notes: Option<String>,
}

/// 状态推进 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdvanceStatusDto {
    /// 目标状态
    pub target_status: String,
    /// 操作人 ID
    pub operator_id: i64,
    /// 备注
    pub notes: Option<String>,
}

/// 取消定制订单 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CancelCustomOrderDto {
    /// 取消原因
    pub reason: String,
}
