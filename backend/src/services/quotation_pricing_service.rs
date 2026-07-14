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

use crate::models::product_color_price;
use crate::utils::app_state::AppState;
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
    /// 不含税单价
    pub unit_price: Decimal,
    /// 含税单价
    pub unit_price_with_tax: Decimal,
    /// 匹配的阶梯价
    pub tier_breakdown: Vec<TierPrice>,
    /// 折扣金额（每单位）
    pub discount_applied: Decimal,
    /// 最终金额（不含税）
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

        // 3. 客户等级折扣
        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let discount_rate = ctx.customer_level.discount_rate();
        let discount_amount = (tier.unit_price * discount_rate).round_dp(2);
        let unit_price = (tier.unit_price - discount_amount).round_dp(2);

        // 4. 含税计算（默认 13% 增值税）
        let tax_rate = Decimal::new(13, 2);
        let unit_price_with_tax =
            (unit_price * (Decimal::ONE + tax_rate / Decimal::from(100))).round_dp(2);

        // 5. 最终金额
        let final_amount = (unit_price * ctx.quantity).round_dp(2);

        Ok(PricingResult {
            unit_price,
            unit_price_with_tax,
            tier_breakdown: vec![tier],
            discount_applied: discount_amount,
            final_amount,
            price_source: PriceSource::ColorPrice,
        })
    }

    /// 阶梯价匹配：
    /// - 若 `min_quantity` 存在且 `<= quantity`，则应用基础价作为阶梯起点
    /// - 否则按基础价无阶梯
    fn match_tier(base_price: Decimal, quantity: Decimal, min_quantity: Option<Decimal>) -> TierPrice {
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

#[cfg(test)]
mod tests {
    //! 销售报价单定价服务单元测试（批次 409 P2-8 补测）
    //!
    //! 覆盖目标：
    //! - CustomerLevel 折扣率与编码解析（VIP/Normal/大小写/空串/含空格）
    //! - match_tier 阶梯价匹配（数量>min_q/边界相等/小于/None/零值）
    //! - PricingContext::customer_level_opt VIP/Normal 分支

    use super::*;

    // 工厂函数：构造测试基础价 10.00 元，避免散落硬编码
    fn create_base_price() -> Decimal {
        Decimal::new(1000, 2)
    }

    // 工厂函数：构造测试定价上下文，统一字段避免重复
    fn create_test_pricing_context(level: CustomerLevel) -> PricingContext {
        PricingContext {
            customer_id: 1,
            customer_level: level,
            product_id: 100,
            color_id: Some(200),
            quantity: Decimal::new(500, 0),
            currency: "CNY".to_string(),
            quotation_date: chrono::NaiveDate::from_ymd_opt(2026, 7, 14)
                .expect("测试夹具：构造测试日期失败"),
        }
    }

    /// VIP 折扣率必须为 0.05（95 折），calculate 中 discount_amount 依赖此值
    #[test]
    fn test_客户等级折扣率_VIP应为0_05() {
        let rate = CustomerLevel::Vip.discount_rate();
        assert_eq!(rate, Decimal::new(5, 2));
    }

    /// 普通客户无折扣，折扣率必须为 0，避免金额被错误折减
    #[test]
    fn test_客户等级折扣率_普通客户应为0() {
        let rate = CustomerLevel::Normal.discount_rate();
        assert_eq!(rate, Decimal::ZERO);
    }

    /// from_code "VIP" 大写应解析为 Vip，色号价格查询依赖此映射
    #[test]
    fn test_from_code_VIP大写解析为Vip() {
        assert_eq!(CustomerLevel::from_code("VIP"), CustomerLevel::Vip);
    }

    /// from_code 应大小写不敏感，前端可能传入任意大小写组合
    #[test]
    fn test_from_code_大小写不敏感解析为Vip() {
        assert_eq!(CustomerLevel::from_code("vip"), CustomerLevel::Vip);
        assert_eq!(CustomerLevel::from_code("Vip"), CustomerLevel::Vip);
        assert_eq!(CustomerLevel::from_code("vIp"), CustomerLevel::Vip);
    }

    /// 非客户等级字符串识别为 Normal，避免误授权 VIP 折扣
    #[test]
    fn test_from_code_非VIP字符串解析为Normal() {
        assert_eq!(CustomerLevel::from_code("NORMAL"), CustomerLevel::Normal);
        assert_eq!(CustomerLevel::from_code("MEMBER"), CustomerLevel::Normal);
    }

    /// 空字符串安全降级为 Normal，防止解析异常中断业务
    #[test]
    fn test_from_code_空字符串解析为Normal() {
        assert_eq!(CustomerLevel::from_code(""), CustomerLevel::Normal);
    }

    /// 含空格的 "VIP" 不应被识别为 Vip（from_code 未做 trim，避免掩盖输入异常）
    #[test]
    fn test_from_code_含空格的VIP解析为Normal() {
        assert_eq!(CustomerLevel::from_code(" VIP "), CustomerLevel::Normal);
    }

    /// 数量 > min_quantity 时应使用 min_quantity 作为阶梯起点，业务上确认档位匹配
    #[test]
    fn test_match_tier_数量大于最小数量_应用阶梯价() {
        let base = create_base_price();
        let min_q = Some(Decimal::new(100, 0));
        let quantity = Decimal::new(500, 0);

        let tier = QuotationPricingService::match_tier_for_unit_test(base, quantity, min_q);

        assert_eq!(tier.min_quantity, Decimal::new(100, 0));
        assert_eq!(tier.max_quantity, None);
        assert_eq!(tier.unit_price, base);
    }

    /// 边界：数量 == min_quantity 时仍应匹配阶梯价（条件为 min_q <= quantity）
    #[test]
    fn test_match_tier_数量等于最小数量_边界匹配阶梯价() {
        let base = create_base_price();
        let min_q = Some(Decimal::new(100, 0));
        let quantity = Decimal::new(100, 0);

        let tier = QuotationPricingService::match_tier_for_unit_test(base, quantity, min_q);

        assert_eq!(tier.min_quantity, Decimal::new(100, 0));
        assert_eq!(tier.unit_price, base);
    }

    /// 数量 < min_quantity 时回退到默认档位（min_quantity=1），避免误用未达档位价格
    #[test]
    fn test_match_tier_数量小于最小数量_回退默认档() {
        let base = create_base_price();
        let min_q = Some(Decimal::new(500, 0));
        let quantity = Decimal::new(100, 0);

        let tier = QuotationPricingService::match_tier_for_unit_test(base, quantity, min_q);

        assert_eq!(tier.min_quantity, Decimal::ONE);
        assert_eq!(tier.max_quantity, None);
        assert_eq!(tier.unit_price, base);
    }

    /// min_quantity=None 时回退到默认档位（min_quantity=1），色号价格表未配置档位时的兜底
    #[test]
    fn test_match_tier_min_quantity为None_回退默认档() {
        let base = create_base_price();
        let quantity = Decimal::new(500, 0);

        let tier = QuotationPricingService::match_tier_for_unit_test(base, quantity, None);

        assert_eq!(tier.min_quantity, Decimal::ONE);
        assert_eq!(tier.max_quantity, None);
        assert_eq!(tier.unit_price, base);
    }

    /// 数量=0 且 min_quantity=None 时仍回退默认档，避免空档位导致业务异常
    #[test]
    fn test_match_tier_零数量None最小数量_回退默认档() {
        let base = create_base_price();

        let tier = QuotationPricingService::match_tier_for_unit_test(base, Decimal::ZERO, None);

        assert_eq!(tier.min_quantity, Decimal::ONE);
        assert_eq!(tier.unit_price, base);
    }

    /// 数量=0 且 min_quantity=0 时 0<=0 成立，应匹配档位（min_quantity 字段为 0）
    #[test]
    fn test_match_tier_零数量零最小数量_匹配档位() {
        let base = create_base_price();

        let tier = QuotationPricingService::match_tier_for_unit_test(
            base,
            Decimal::ZERO,
            Some(Decimal::ZERO),
        );

        assert_eq!(tier.min_quantity, Decimal::ZERO);
        assert_eq!(tier.unit_price, base);
    }

    /// VIP 客户 customer_level_opt 必须返回 "VIP"，色号价格表 customer_level 字段按此匹配
    #[test]
    fn test_PricingContext_VIP客户_level_opt返回VIP() {
        let ctx = create_test_pricing_context(CustomerLevel::Vip);
        assert_eq!(ctx.customer_level_opt(), Some("VIP"));
    }

    /// Normal 客户 customer_level_opt 返回 None，触发色号价格表回退到通用价
    #[test]
    fn test_PricingContext_普通客户_level_opt返回None() {
        let ctx = create_test_pricing_context(CustomerLevel::Normal);
        assert_eq!(ctx.customer_level_opt(), None);
    }
}
