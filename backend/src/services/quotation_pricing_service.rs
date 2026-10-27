//! 销售报价单定价服务
//!
//! 业务功能：
//! - 阶梯价匹配（min_quantity 档位）
//! - 客户等级折扣（VIP 95 折）
//! - 增值税含税/不含税转换
//! - 价格来源标记
//!
//! Week 2 任务 6 - 销售报价单模块
//! 创建时间: 2026-06-16
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 6

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::container::AppState;
use crate::models::product_color_price;
use crate::utils::error::AppError;

/// 客户等级（影响折扣）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CustomerLevel {
    /// VIP 客户（95 折）
    Vip,
    /// 普通客户（无折扣）
    Normal,
}

impl CustomerLevel {
    /// 折扣率（0.05 = 95 折）
    pub fn discount_rate(&self) -> Decimal {
        match self {
            CustomerLevel::Vip => Decimal::new(5, 2),
            CustomerLevel::Normal => Decimal::ZERO,
        }
    }

    /// 解析
    pub fn from_code(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "VIP" => CustomerLevel::Vip,
            _ => CustomerLevel::Normal,
        }
    }
}

/// 定价上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingContext {
    pub customer_id: i64,
    pub customer_level: CustomerLevel,
    pub product_id: i64,
    pub color_id: Option<i64>,
    pub quantity: Decimal,
    pub currency: String,
    pub quotation_date: chrono::NaiveDate,
    /// 本张报价单的适用税率，**百分数口径**（13 即 13%，免税行传 0），
    /// 由调用方从报价单自身的税率（人工录入或上游单据派生）带入，与服务端
    /// sales_quotations.tax_rate 同单位。必填、无默认档位，缺字段由入参反序列化
    /// 直接拒绝，服务端不会用任何税率补算。
    pub tax_rate: Decimal,
}

/// 单档阶梯价
#[derive(Debug, Clone, Serialize)]
pub struct TierPrice {
    pub min_quantity: Decimal,
    pub max_quantity: Option<Decimal>,
    pub unit_price: Decimal,
}

/// 定价结果
#[derive(Debug, Clone, Serialize)]
pub struct PricingResult {
    /// 不含税单价（由含税单价经税额内核除法反算派生，不是录入事实，只读展示与核对用）
    pub unit_price: Decimal,
    /// 含税单价（色号价格表带入的录入事实，折扣已扣）
    pub unit_price_with_tax: Decimal,
    /// 匹配的阶梯价
    pub tier_breakdown: Vec<TierPrice>,
    /// 折扣金额（每单位，含税口径）
    pub discount_applied: Decimal,
    /// 最终金额（不含税，由含税总额价税分离反算得到）
    pub final_amount: Decimal,
    /// 价格来源
    pub price_source: PriceSource,
}

/// 价格来源
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    /// 来自色号价格表
    ColorPrice,
    /// 来自产品基础价
    ProductPrice,
    /// 来自促销
    Promotion,
}

/// 定价服务
pub struct QuotationPricingService {
    db: Arc<DatabaseConnection>,
}

impl QuotationPricingService {
    /// 从数据库连接直接构造
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造便捷方法
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 执行价格计算
    pub async fn calculate(&self, ctx: PricingContext) -> Result<PricingResult, AppError> {
        // 1. 查询色号价格
        let color_price = if let Some(color_id) = ctx.color_id {
            product_color_price::Entity::find()
                .filter(product_color_price::Column::ProductId.eq(ctx.product_id))
                .filter(product_color_price::Column::ColorId.eq(color_id))
                .filter(product_color_price::Column::Currency.eq(&ctx.currency))
                .filter(product_color_price::Column::EffectiveFrom.lte(ctx.quotation_date))
                .all(&*self.db)
                .await?
        } else {
            Vec::new()
        };

        // 过滤匹配的客户等级记录；若无匹配则取 NULL 等级（通用价）
        let matched = if let Some(level) = ctx.customer_level_opt() {
            color_price
                .iter()
                .find(|cp| {
                    cp.customer_level
                        .as_deref()
                        .map(|l| l.eq_ignore_ascii_case(level))
                        .unwrap_or(false)
                })
                .cloned()
        } else {
            None
        };
        let selected_price = matched.or_else(|| {
            // 回退到无客户等级限制的记录
            color_price
                .iter()
                .find(|cp| cp.customer_level.is_none())
                .cloned()
        });

        let cp = selected_price.ok_or_else(|| {
            AppError::not_found(format!(
                "色号价格未配置（product_id={}, color_id={}, currency={}）",
                ctx.product_id,
                ctx.color_id.unwrap_or(0),
                ctx.currency
            ))
        })?;

        let base_price = cp.base_price;

        // 2. 阶梯价匹配（按 min_quantity 阈值）
        let tier = Self::match_tier(base_price, ctx.quantity, cp.min_quantity);

        // 3. 客户等级折扣：折扣作用在采价时定妥的含税价上（每单位），折扣额与折后价都是含税口径
        //    金额计算 round_dp(2) 精度归一化
        let discount_rate = ctx.customer_level.discount_rate();
        let discount_applied = (tier.unit_price * discount_rate).round_dp(2);
        let unit_price_with_tax = (tier.unit_price - discount_applied).round_dp(2);

        // 4. 价税分离：色号价格表的 base_price 是采价录入事实（该表只有一个价格列、
        //    无税率位，采到的就是对外报价用的含税价），因此含税单价是带入事实，
        //    不含税单价经 utils tax 内核由除法反算得到。本服务不乘 (1 + 税率)，
        //    也不持有、不回落任何税率档位，税率取本张报价单自身的 tax_rate（百分数口径）。
        let unit_price = crate::utils::tax::net_from_inclusive(unit_price_with_tax, ctx.tax_rate);

        // 5. 最终金额：先按含税单价与数量聚合出含税事实，再做价税分离，
        //    final_amount 出的是反算净额（不含税金额），税额与含税额同时由 split 带出，
        //    三者天然满足「净额 + 税额 = 含税额」，供前端预览与落库核对同源。
        let inclusive_amount =
            crate::utils::tax::inclusive_amount(unit_price_with_tax, ctx.quantity);
        let split = crate::utils::tax::split_from_inclusive(inclusive_amount, ctx.tax_rate);
        let final_amount = split.net;

        Ok(PricingResult {
            unit_price,
            unit_price_with_tax,
            tier_breakdown: vec![tier],
            discount_applied,
            final_amount,
            price_source: PriceSource::ColorPrice,
        })
    }

    /// 阶梯价匹配：（若 `min_quantity` 存在且 `<= quantity`，则应用基础价作为阶梯起点；否则按基础价无阶梯）
    fn match_tier(
        base_price: Decimal,
        quantity: Decimal,
        min_quantity: Option<Decimal>,
    ) -> TierPrice {
        match min_quantity {
            Some(min_q) if min_q <= quantity => TierPrice {
                min_quantity: min_q,
                max_quantity: None,
                unit_price: base_price,
            },
            _ => TierPrice {
                min_quantity: Decimal::ONE,
                max_quantity: None,
                unit_price: base_price,
            },
        }
    }

    /// 单元测试用阶梯价匹配（pub 暴露给 tests/ 集成测试）
    pub fn match_tier_for_unit_test(
        base_price: Decimal,
        quantity: Decimal,
        min_quantity: Option<Decimal>,
    ) -> TierPrice {
        Self::match_tier(base_price, quantity, min_quantity)
    }
}

impl PricingContext {
    /// 客户等级的字符串表示（用于查询匹配）
    fn customer_level_opt(&self) -> Option<&'static str> {
        match self.customer_level {
            CustomerLevel::Vip => Some("VIP"),
            CustomerLevel::Normal => None,
        }
    }
}
