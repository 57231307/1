#![allow(dead_code)]
// TODO(tech-debt): 待 P13+ port product_color_price 后移除本占位文件；届时接入
// 阶梯价（tier pricing）、色号专属折扣、协议价等业务规则。当前 PR-2 阶段
// 不依赖 product_color_price 模型，所有定价方法返回 0 避免阻断主流程。

//! 销售报价单定价服务（占位）
//!
//! P0 port 销售报价单 PR-2 阶段的 stub。完整定价逻辑（阶梯价 / 协议折扣 / 色号价目表）依赖
//! `test` 分支独有的 `product_color_price` 模型，需在 P13+ 单独 port 后接入。
//!
//! # 设计原则
//! - 不引入 `product_color_price` 模型依赖
//! - 提供与最终实现同名的方法签名（`calculate`），handler 层可安全引用
//! - 临时返回值：固定 `Decimal::ZERO`，业务层应在此基础上由人工核算填充金额
//!
//! # 替换计划
//! - P13 port `product_color_price` 模型
//! - 替换 `QuotationPricingService::calculate` 实现：按 product_id + color_id 查表 + 协议价叠加
//! - 移除本文件级 `#![allow(dead_code)]` 改为项级注释

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::utils::error::AppError;

/// 报价单定价上下文（stub 占位；与最终接口形态保持一致）
///
/// # 字段说明
/// - `product_id`: 产品主表 ID
/// - `color_id`: 色号 ID（可选）
/// - `quantity`: 数量
/// - `unit_price`: 主表上的报价（不含税）
/// - `customer_id`: 客户 ID（用于客户级别折扣）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingContext {
    pub product_id: i32,
    pub color_id: Option<i32>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub customer_id: i32,
}

/// 报价单定价服务（占位实现）
#[allow(dead_code)] // TODO(tech-debt): 待 P13+ port product_color_price 后移除
pub struct QuotationPricingService;

impl QuotationPricingService {
    /// 计算报价单明细行项目金额（stub）
    ///
    /// # 当前行为
    /// 直接返回 `Decimal::ZERO`，不读取任何外部数据；handler 层应在收到此结果后
    /// 走人工复核 / 兜底逻辑。
    ///
    /// # 后续实现
    /// ```text
    /// 1. 根据 product_id + color_id 查 product_color_price
    /// 2. 叠加 customer_id 对应的客户级别折扣
    /// 3. 按 quantity 命中阶梯价表
    /// 4. 返回 Decimal 总价
    /// ```
    pub async fn calculate(_ctx: PricingContext) -> Result<Decimal, AppError> {
        // TODO: 集成 product_color_price 后实现阶梯价/折扣
        Ok(Decimal::ZERO)
    }

    /// 校验定价上下文（stub）
    ///
    /// 完整实现应检查：产品存在、色号存在、阶梯价表配置存在。
    /// 当前 stub 仅做非空校验。
    #[allow(dead_code)] // TODO(tech-debt): 待 P13+ port 后扩展为完整校验
    pub fn validate(ctx: &PricingContext) -> Result<(), AppError> {
        if ctx.unit_price < Decimal::ZERO {
            return Err(AppError::validation("定价单价不能为负数"));
        }
        if ctx.quantity <= Decimal::ZERO {
            return Err(AppError::validation("定价数量必须大于 0"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).expect("测试金额格式错误")
    }

    #[tokio::test]
    async fn calculate_returns_zero_in_stub_mode() {
        let ctx = PricingContext {
            product_id: 1,
            color_id: None,
            quantity: dec("10"),
            unit_price: dec("50.00"),
            customer_id: 1001,
        };
        let result = QuotationPricingService::calculate(ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::ZERO);
    }

    #[test]
    fn validate_rejects_negative_unit_price() {
        let ctx = PricingContext {
            product_id: 1,
            color_id: None,
            quantity: dec("10"),
            unit_price: dec("-1.00"),
            customer_id: 1001,
        };
        assert!(QuotationPricingService::validate(&ctx).is_err());
    }

    #[test]
    fn validate_rejects_zero_quantity() {
        let ctx = PricingContext {
            product_id: 1,
            color_id: None,
            quantity: dec("0"),
            unit_price: dec("10.00"),
            customer_id: 1001,
        };
        assert!(QuotationPricingService::validate(&ctx).is_err());
    }

    #[test]
    fn validate_accepts_valid_context() {
        let ctx = PricingContext {
            product_id: 1,
            color_id: Some(5),
            quantity: dec("100"),
            unit_price: dec("25.50"),
            customer_id: 2002,
        };
        assert!(QuotationPricingService::validate(&ctx).is_ok());
    }
}
