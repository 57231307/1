//! 销售报价单更新 DTO
//!
//! 销售报价单的可选字段更新 DTO。所有字段均为 `Option<T>`，未提供（`None`）的字段表示保持原值。
//! 使用 `Option<Option<T>>` 无法表达 `null`，因此采用哨兵值约定：
//! 业务层应使用 `serde_json::Value::Null` 检测显式清空，或在更新时通过专门的清空接口处理。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::quotation_create_dto::{QuotationItemCreateDto, QuotationTermCreateDto};

/// 销售报价单更新请求
///
/// 约定：
/// - 字段为 `None` 时表示"不修改"；
/// - 字段为 `Some(value)` 时表示"更新为 value"；
/// - 主表元数据（id、created_by、created_at、status、approval_instance_id、approved_*、converted_*）不允许通过此 DTO 修改，
///   对应更新需调用专用方法（submit / approve / reject / cancel / convert）。
#[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuotationUpdateDto {
    /// 客户 ID
    #[serde(default)]
    pub customer_id: Option<i32>,
    /// 销售员 ID
    #[serde(default)]
    pub sales_user_id: Option<i32>,
    /// 报价日期
    #[serde(default)]
    pub quotation_date: Option<NaiveDate>,
    /// 有效期至
    #[serde(default)]
    pub valid_until: Option<NaiveDate>,
    /// 报价货币
    #[serde(default)]
    pub currency: Option<String>,
    /// 汇率
    #[serde(default)]
    pub exchange_rate: Option<Decimal>,
    /// 本位币
    #[serde(default)]
    pub base_currency: Option<String>,
    /// 价格条款
    #[serde(default)]
    pub price_terms: Option<String>,
    /// Incoterms 版本
    #[serde(default)]
    pub incoterms_version: Option<String>,
    /// Incoterms 地点
    #[serde(default)]
    pub incoterm_location: Option<String>,
    /// 是否含税
    #[serde(default)]
    pub tax_inclusive: Option<bool>,
    /// 税率
    #[serde(default)]
    pub tax_rate: Option<Decimal>,
    /// 最小起订量
    #[serde(default)]
    pub moq: Option<Decimal>,
    /// 交货周期（天）
    #[serde(default)]
    pub lead_time_days: Option<i32>,
    /// 客户等级
    #[serde(default)]
    pub customer_level: Option<String>,
    /// 不含税小计
    #[serde(default)]
    pub subtotal: Option<Decimal>,
    /// 税额
    #[serde(default)]
    pub tax_amount: Option<Decimal>,
    /// 含税总额
    #[serde(default)]
    pub total_amount: Option<Decimal>,
    /// 备注
    #[serde(default)]
    pub notes: Option<String>,
    /// 明细行项目（None=不修改；Some(vec)=覆盖为新列表）
    #[serde(default)]
    pub items: Option<Vec<QuotationItemCreateDto>>,
    /// 贸易条款（None=不修改；Some(vec)=覆盖为新列表）
    #[serde(default)]
    pub terms: Option<Vec<QuotationTermCreateDto>>,
}
