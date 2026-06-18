//! 销售报价单响应 DTO
//!
//! 完整响应 DTO：主表 + 明细行项目 + 贸易条款。
//! 实现 `From<(Model, Vec<Item>, Vec<Term>)>` 以便 Service 层从查询结果直接转换。

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::sales_quotation::Model as QuotationModel;
use crate::models::sales_quotation_item::Model as QuotationItemModel;
use crate::models::sales_quotation_term::Model as QuotationTermModel;

/// 销售报价单明细行项目响应
#[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationItemResponseDto {
    /// 明细 ID
    pub id: i32,
    /// 报价单 ID
    pub quotation_id: i32,
    /// 产品 ID
    pub product_id: i32,
    /// 色号 ID
    pub color_id: Option<i32>,
    /// 色号编码
    pub color_code: Option<String>,
    /// 潘通色号
    pub pantone_code: Option<String>,
    /// CNCS 色号
    pub cncs_code: Option<String>,
    /// 规格
    pub specification: Option<String>,
    /// 计量单位
    pub unit: String,
    /// 数量
    pub quantity: Decimal,
    /// 不含税单价
    pub unit_price: Decimal,
    /// 含税单价
    pub unit_price_with_tax: Decimal,
    /// 不含税金额
    pub amount: Decimal,
    /// 含税金额
    pub amount_with_tax: Decimal,
    /// 阶梯价 JSON 数据
    pub tier_pricing: Option<serde_json::Value>,
    /// 折扣率（百分比）
    pub discount_rate: Option<Decimal>,
    /// 折扣金额
    pub discount_amount: Option<Decimal>,
    /// 备注
    pub notes: Option<String>,
    /// 排序号
    pub sequence: i32,
}

impl From<QuotationItemModel> for QuotationItemResponseDto {
    #[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
    fn from(model: QuotationItemModel) -> Self {
        // sea_orm 2.0 的 Json 是 serde_json::Value 的别名，直接转换即可
        let tier_pricing: Option<serde_json::Value> = model.tier_pricing;
        Self {
            id: model.id,
            quotation_id: model.quotation_id,
            product_id: model.product_id,
            color_id: model.color_id,
            color_code: model.color_code,
            pantone_code: model.pantone_code,
            cncs_code: model.cncs_code,
            specification: model.specification,
            unit: model.unit,
            quantity: model.quantity,
            unit_price: model.unit_price,
            unit_price_with_tax: model.unit_price_with_tax,
            amount: model.amount,
            amount_with_tax: model.amount_with_tax,
            tier_pricing,
            discount_rate: model.discount_rate,
            discount_amount: model.discount_amount,
            notes: model.notes,
            sequence: model.sequence,
        }
    }
}

/// 销售报价单贸易条款响应
#[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationTermResponseDto {
    /// 条款 ID
    pub id: i32,
    /// 报价单 ID
    pub quotation_id: i32,
    /// 条款类型
    pub term_type: String,
    /// 条款键
    pub term_key: String,
    /// 条款值
    pub term_value: String,
    /// 排序号
    pub sequence: i32,
}

impl From<QuotationTermModel> for QuotationTermResponseDto {
    #[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
    fn from(model: QuotationTermModel) -> Self {
        Self {
            id: model.id,
            quotation_id: model.quotation_id,
            term_type: model.term_type,
            term_key: model.term_key,
            term_value: model.term_value,
            sequence: model.sequence,
        }
    }
}

/// 销售报价单完整响应 DTO
///
/// 字段类型与 main `sales_quotation` 模型保持一致；时间字段序列化为 RFC3339 字符串。
#[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationResponseDto {
    /// 报价单 ID
    pub id: i32,
    /// 报价单号
    pub quotation_no: String,
    /// 客户 ID
    pub customer_id: i32,
    /// 销售员 ID
    pub sales_user_id: i32,
    /// 报价日期
    pub quotation_date: NaiveDate,
    /// 有效期至
    pub valid_until: NaiveDate,
    /// 报价货币
    pub currency: String,
    /// 汇率
    pub exchange_rate: Decimal,
    /// 本位币
    pub base_currency: String,
    /// 价格条款
    pub price_terms: String,
    /// Incoterms 版本
    pub incoterms_version: Option<String>,
    /// Incoterms 地点
    pub incoterm_location: Option<String>,
    /// 是否含税
    pub tax_inclusive: bool,
    /// 税率（百分比）
    pub tax_rate: Decimal,
    /// 最小起订量
    pub moq: Option<Decimal>,
    /// 交货周期（天）
    pub lead_time_days: Option<i32>,
    /// 客户等级
    pub customer_level: Option<String>,
    /// 不含税小计
    pub subtotal: Decimal,
    /// 税额
    pub tax_amount: Decimal,
    /// 含税总额
    pub total_amount: Decimal,
    /// 状态
    pub status: String,
    /// BPM 审批实例 ID
    pub approval_instance_id: Option<i32>,
    /// 审批人
    pub approved_by: Option<i32>,
    /// 审批时间
    pub approved_at: Option<DateTime<Utc>>,
    /// 拒绝原因
    pub rejection_reason: Option<String>,
    /// 转换后的销售订单 ID
    pub converted_sales_order_id: Option<i32>,
    /// 转换时间
    pub converted_at: Option<DateTime<Utc>>,
    /// 备注
    pub notes: Option<String>,
    /// 创建人
    pub created_by: i32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 明细行项目
    pub items: Vec<QuotationItemResponseDto>,
    /// 贸易条款
    pub terms: Vec<QuotationTermResponseDto>,
}

impl
    From<(
        QuotationModel,
        Vec<QuotationItemModel>,
        Vec<QuotationTermModel>,
    )> for QuotationResponseDto
{
    #[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
    fn from(
        input: (
            QuotationModel,
            Vec<QuotationItemModel>,
            Vec<QuotationTermModel>,
        ),
    ) -> Self {
        let (m, items, terms) = input;
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
            items: items
                .into_iter()
                .map(QuotationItemResponseDto::from)
                .collect(),
            terms: terms
                .into_iter()
                .map(QuotationTermResponseDto::from)
                .collect(),
        }
    }
}

/// 销售报价单列表查询参数
#[allow(dead_code)] // TODO(tech-debt): 等待 PR-3 handler 接入后移除
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuotationQueryParams {
    /// 客户 ID
    #[serde(default)]
    pub customer_id: Option<i32>,
    /// 销售员 ID
    #[serde(default)]
    pub sales_user_id: Option<i32>,
    /// 状态
    #[serde(default)]
    pub status: Option<String>,
    /// 关键字（报价单号模糊）
    #[serde(default)]
    pub keyword: Option<String>,
    /// 页码（从 1 开始）
    #[serde(default = "default_page")]
    pub page: u64,
    /// 每页数量
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    20
}
