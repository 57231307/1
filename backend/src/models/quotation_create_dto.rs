//! 销售报价单创建 DTO
//!
//! 提供创建销售报价单的请求结构：主表字段 + 明细行项目 + 贸易条款。
//! 字段语义参考销售报价单主表 / 明细 / 贸易条款三张表的 PR-1 迁移。
//!
//! # 关键约束
//! - 不依赖 `product_color_price`（test 独有模型），仅引用 main 已有的 `product` 主表 ID。
//! - 字段命名采用 main 风格：`customer_id` / `sales_user_id` / `currency` / `price_terms` 等。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 销售报价单明细行项目创建请求
///
/// 与 `sales_quotation_items` 表一一对应。色号字段均为可选（不同产品支持度不同）。
#[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationItemCreateDto {
    /// 产品 ID（必填，外键 products.id）
    pub product_id: i32,
    /// 色号 ID（可选，外键 product_colors.id）
    #[serde(default)]
    pub color_id: Option<i32>,
    /// 色号编码（业务编码，可选）
    #[serde(default)]
    pub color_code: Option<String>,
    /// 潘通色号（可选）
    #[serde(default)]
    pub pantone_code: Option<String>,
    /// CNCS 色号（可选）
    #[serde(default)]
    pub cncs_code: Option<String>,
    /// 规格（如幅宽、克重，可选）
    #[serde(default)]
    pub specification: Option<String>,
    /// 计量单位（必填，如米/件/千克）
    pub unit: String,
    /// 数量
    pub quantity: Decimal,
    /// 不含税单价
    pub unit_price: Decimal,
    /// 含税单价
    #[serde(default)]
    pub unit_price_with_tax: Option<Decimal>,
    /// 不含税金额
    #[serde(default)]
    pub amount: Option<Decimal>,
    /// 含税金额
    #[serde(default)]
    pub amount_with_tax: Option<Decimal>,
    /// 阶梯价 JSON 字符串（可选）
    #[serde(default)]
    pub tier_pricing: Option<String>,
    /// 折扣率（百分比，可选）
    #[serde(default)]
    pub discount_rate: Option<Decimal>,
    /// 折扣金额（可选）
    #[serde(default)]
    pub discount_amount: Option<Decimal>,
    /// 备注（可选）
    #[serde(default)]
    pub notes: Option<String>,
    /// 排序号
    #[serde(default)]
    pub sequence: Option<i32>,
}

/// 销售报价单贸易条款创建请求
///
/// 与 `sales_quotation_terms` 表一一对应。
/// `term_type` 取值范围：logistics / payment / sample / inspection。
#[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationTermCreateDto {
    /// 条款类型（logistics/payment/sample/inspection）
    pub term_type: String,
    /// 条款键
    pub term_key: String,
    /// 条款值
    pub term_value: String,
    /// 排序号
    #[serde(default)]
    pub sequence: Option<i32>,
}

/// 销售报价单创建请求
///
/// # 字段说明
/// - `customer_id` 必填：客户主表外键
/// - `sales_user_id` 必填：销售员（外键 users.id）
/// - `quotation_date` / `valid_until` 必填：报价日期 + 有效期
/// - `currency` / `base_currency` 默认 CNY，`exchange_rate` 默认 1.0
/// - `price_terms` 必填：Incoterms 2020 价目条款（FOB/CIF/EXW/DDP/DAP）
/// - `items` 必填：至少包含 1 条明细
/// - `terms` 可选：贸易条款（物流/付款/样品/检验）
#[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationCreateDto {
    /// 报价单号（可选；不提供时由服务层生成）
    #[serde(default)]
    pub quotation_no: Option<String>,
    /// 客户 ID
    pub customer_id: i32,
    /// 销售员 ID
    pub sales_user_id: i32,
    /// 报价日期
    pub quotation_date: NaiveDate,
    /// 有效期至
    pub valid_until: NaiveDate,
    /// 报价货币
    #[serde(default = "default_currency")]
    pub currency: String,
    /// 汇率
    #[serde(default = "default_exchange_rate")]
    pub exchange_rate: Decimal,
    /// 本位币
    #[serde(default = "default_currency")]
    pub base_currency: String,
    /// 价格条款（Incoterms 2020：FOB/CIF/EXW/DDP/DAP）
    pub price_terms: String,
    /// Incoterms 版本（默认 2020）
    #[serde(default = "default_incoterms_version")]
    pub incoterms_version: Option<String>,
    /// Incoterms 地点
    #[serde(default)]
    pub incoterm_location: Option<String>,
    /// 是否含税
    #[serde(default = "default_true")]
    pub tax_inclusive: bool,
    /// 税率（百分比）
    #[serde(default = "default_tax_rate")]
    pub tax_rate: Decimal,
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
    /// 明细行项目（至少 1 条）
    pub items: Vec<QuotationItemCreateDto>,
    /// 贸易条款
    #[serde(default)]
    pub terms: Vec<QuotationTermCreateDto>,
}

fn default_currency() -> String {
    "CNY".to_string()
}

fn default_exchange_rate() -> Decimal {
    Decimal::ONE
}

fn default_incoterms_version() -> Option<String> {
    Some("2020".to_string())
}

fn default_true() -> bool {
    true
}

fn default_tax_rate() -> Decimal {
    // 默认税率 13%（中国大陆增值税）
    Decimal::new(13, 0)
}
