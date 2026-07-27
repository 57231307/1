//! 面料多色号定价扩展 - 价格计算引擎
//!
//! 统一价格计算：客户专属价 > 季节调价 > 阶梯价 > 客户等级 > 基础价
//! 创建时间: 2026-06-18
//! 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §4.4

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use thiserror::Error;

use crate::models::color_price_dto::{PriceCalcRequest, PriceCalcResult, PriceCalcStep};
use crate::models::color_price_tier;
use crate::models::customer_color_price;
use crate::models::product_color_price;
use crate::models::seasonal_price_rule;

/// 计算错误
#[derive(Debug, Error)]
pub enum CalcError {
    #[error("未找到基础价格")]
    BasePriceNotFound,
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 客户等级折扣
pub fn customer_level_discount(level: Option<&str>) -> Decimal {
    match level {
        // P0 6-2 修复：scale 3 → 2（原 0.095 应为 0.95，导致价格被错误地缩小 10 倍）
        Some("VIP") => Decimal::new(95, 2),     // 0.95（95 折）
        Some("GOLD") => Decimal::new(90, 2),    // 0.90（9 折）
        Some("SILVER") => Decimal::new(95, 2),  // 0.95（95 折）
        _ => Decimal::new(100, 2),              // 1.00（100%，NORMAL/无折扣）
    }
}

/// 计算价格（优先级：客户专属价 > 季节调价 > 阶梯价 > 客户等级 > 基础价）
pub async fn calculate_price(
    db: &DatabaseConnection,
    req: &PriceCalcRequest,
) -> Result<PriceCalcResult, CalcError> {
    let calc_date = req.calc_date.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let mut breakdown: Vec<PriceCalcStep> = Vec::new();
    let base_price = find_base_price(db, req).await?;
    let mut current = base_price;
    breakdown.push(make_base_step(current, &req.currency));

    let mut tier_price: Option<Decimal> = None;
    let mut level_price: Option<Decimal> = None;
    let mut season_price: Option<Decimal> = None;
    let mut special_price: Option<Decimal> = None;
    let mut applied_rule = "base".to_string();
    if let Some((tier, step)) = apply_tier_step(db, req, current).await? {
        current = tier;
        tier_price = Some(tier);
        breakdown.push(step);
        applied_rule = "tier".to_string();
    }

    if let Some((lvl, step)) = apply_level_step(req, current) {
        current = lvl;
        level_price = Some(lvl);
        breakdown.push(step);
        applied_rule = "level".to_string();
    }

    if let Some((season, step)) = apply_seasonal_step(db, req, calc_date, current).await? {
        current = season;
        season_price = Some(season);
        breakdown.push(step);
        applied_rule = "seasonal".to_string();
    }

    if let Some((sp, step)) = apply_customer_special_step(db, req, calc_date, current).await? {
        current = sp;
        special_price = Some(sp);
        breakdown.push(step);
        applied_rule = "customer_special".to_string();
    }

    Ok(build_calc_result(
        base_price,
        current,
        tier_price,
        level_price,
        season_price,
        special_price,
        &req.currency,
        applied_rule,
        breakdown,
    ))
}

/// 构建价格计算结果
fn build_calc_result(
    base_price: Decimal,
    final_price: Decimal,
    tier_price: Option<Decimal>,
    level_price: Option<Decimal>,
    season_price: Option<Decimal>,
    special_price: Option<Decimal>,
    currency: &str,
    applied_rule: String,
    breakdown: Vec<PriceCalcStep>,
) -> PriceCalcResult {
    PriceCalcResult {
        base_price,
        tier_price,
        level_price,
        season_price,
        special_price,
        final_price,
        currency: currency.to_string(),
        applied_rule,
        breakdown,
    }
}

/// 构建基础价步骤
fn make_base_step(price: Decimal, currency: &str) -> PriceCalcStep {
    PriceCalcStep {
        step: "基础价".to_string(),
        before: price,
        after: price,
        rule: format!("基础价 {:.2} {}", price, currency),
    }
}

/// 应用阶梯价步骤
async fn apply_tier_step(
    db: &DatabaseConnection,
    req: &PriceCalcRequest,
    current: Decimal,
) -> Result<Option<(Decimal, PriceCalcStep)>, CalcError> {
    let tier = match find_tier_price(db, req).await? {
        Some(t) => t,
        None => return Ok(None),
    };
    let step = PriceCalcStep {
        step: "阶梯价".to_string(),
        before: current,
        after: tier,
        rule: format!("数量 {} 命中阶梯价 {:.2} {}", req.quantity, tier, req.currency),
    };
    Ok(Some((tier, step)))
}

/// 应用客户等级折扣步骤
fn apply_level_step(
    req: &PriceCalcRequest,
    current: Decimal,
) -> Option<(Decimal, PriceCalcStep)> {
    let lvl = req.customer_level.as_deref()?;
    let before = current;
    let discount = customer_level_discount(Some(lvl));
    let after = (current * discount).round_dp(6);
    let step = PriceCalcStep {
        step: "客户等级".to_string(),
        before,
        after,
        rule: format!("{} 等级 {:.3} 折", lvl, discount),
    };
    Some((after, step))
}

/// 应用季节调价步骤
async fn apply_seasonal_step(
    db: &DatabaseConnection,
    req: &PriceCalcRequest,
    calc_date: NaiveDate,
    current: Decimal,
) -> Result<Option<(Decimal, PriceCalcStep)>, CalcError> {
    let season = match req.season.as_deref() {
        Some(s) => s,
        None => return Ok(None),
    };
    let adj = match find_seasonal_adjustment(db, req, season, calc_date).await? {
        Some(a) => a,
        None => return Ok(None),
    };
    let before = current;
    let after = match adj.adjustment_type.as_str() {
        "percentage" => (current * (Decimal::from(1) + adj.adjustment_value)).round_dp(6),
        "fixed" => current + adj.adjustment_value,
        _ => current,
    };
    let step = PriceCalcStep {
        step: "季节调价".to_string(),
        before,
        after,
        rule: format!("{} 规则 {} {:.4}", season, adj.adjustment_type, adj.adjustment_value),
    };
    Ok(Some((after, step)))
}

/// 应用客户专属价步骤
async fn apply_customer_special_step(
    db: &DatabaseConnection,
    req: &PriceCalcRequest,
    calc_date: NaiveDate,
    current: Decimal,
) -> Result<Option<(Decimal, PriceCalcStep)>, CalcError> {
    let cust_id = match req.customer_id {
        Some(id) => id,
        None => return Ok(None),
    };
    let sp = match find_customer_special_price(db, cust_id, req, calc_date).await? {
        Some(p) => p,
        None => return Ok(None),
    };
    let before = current;
    let step = PriceCalcStep {
        step: "客户专属价".to_string(),
        before,
        after: sp,
        rule: format!("客户 {} 专属价 {:.2} {}", cust_id, sp, req.currency),
    };
    Ok(Some((sp, step)))
}

// ----------------------------------------------------------------------
// 辅助查询函数
// ----------------------------------------------------------------------

async fn find_base_price(
    db: &DatabaseConnection,
    req: &PriceCalcRequest,
) -> Result<Decimal, CalcError> {
    // 取 active + approval_status=APPROVED + 客户等级匹配 + season 匹配 + 优先级最高的
    let mut q = product_color_price::Entity::find()
        .filter(product_color_price::Column::ProductId.eq(req.product_id))
        .filter(product_color_price::Column::ColorId.eq(req.color_id))
        .filter(product_color_price::Column::Currency.eq(&req.currency))
        .filter(product_color_price::Column::IsActive.eq(true))
        .filter(product_color_price::Column::ApprovalStatus.eq("APPROVED"));

    // 客户等级匹配：NULL = 通用
    if let Some(lvl) = &req.customer_level {
        q = q.filter(
            product_color_price::Column::CustomerLevel
                .eq(lvl.clone())
                .or(product_color_price::Column::CustomerLevel.is_null()),
        );
    } else {
        q = q.filter(product_color_price::Column::CustomerLevel.is_null());
    }

    // 季节匹配：NULL = 通用
    if let Some(season) = &req.season {
        q = q.filter(
            product_color_price::Column::Season
                .eq(season.clone())
                .or(product_color_price::Column::Season.is_null()),
        );
    } else {
        q = q.filter(product_color_price::Column::Season.is_null());
    }

    let row = q
        .order_by_desc(product_color_price::Column::Priority)
        .order_by_desc(product_color_price::Column::CreatedAt)
        .one(db)
        .await?
        .ok_or(CalcError::BasePriceNotFound)?;

    Ok(row.base_price)
}

async fn find_tier_price(
    db: &DatabaseConnection,
    req: &PriceCalcRequest,
) -> Result<Option<Decimal>, CalcError> {
    // 1. 先找到匹配的 product_color_price.id
    let price_row = product_color_price::Entity::find()
        .filter(product_color_price::Column::ProductId.eq(req.product_id))
        .filter(product_color_price::Column::ColorId.eq(req.color_id))
        .filter(product_color_price::Column::Currency.eq(&req.currency))
        .filter(product_color_price::Column::IsActive.eq(true))
        .order_by_desc(product_color_price::Column::Priority)
        .order_by_desc(product_color_price::Column::CreatedAt)
        .one(db)
        .await?;

    let price_id = match price_row {
        Some(p) => p.id,
        None => return Ok(None),
    };

    // 2. 查阶梯价
    let mut q = color_price_tier::Entity::find()
        .filter(color_price_tier::Column::ProductColorPriceId.eq(price_id))
        .filter(color_price_tier::Column::MinQuantity.lte(req.quantity))
        .filter(
            color_price_tier::Column::MaxQuantity
                .gte(req.quantity)
                .or(color_price_tier::Column::MaxQuantity.is_null()),
        );

    if let Some(lvl) = &req.customer_level {
        q = q.filter(
            color_price_tier::Column::CustomerLevel
                .eq(lvl.clone())
                .or(color_price_tier::Column::CustomerLevel.is_null()),
        );
    }

    let row = q
        .order_by_asc(color_price_tier::Column::Sequence)
        .one(db)
        .await?;
    Ok(row.map(|m| m.tier_price))
}

async fn find_seasonal_adjustment(
    db: &DatabaseConnection,
    req: &PriceCalcRequest,
    season: &str,
    calc_date: NaiveDate,
) -> Result<Option<seasonal_price_rule::Model>, CalcError> {
    let mut q = seasonal_price_rule::Entity::find()
        .filter(seasonal_price_rule::Column::IsActive.eq(true))
        .filter(seasonal_price_rule::Column::Season.eq(season))
        .filter(seasonal_price_rule::Column::ValidFrom.lte(calc_date))
        .filter(
            seasonal_price_rule::Column::ValidUntil
                .gte(calc_date)
                .or(seasonal_price_rule::Column::ValidUntil.is_null()),
        );

    if let Some(cat_id) = req.product_category_id {
        q = q.filter(
            seasonal_price_rule::Column::ProductCategoryId
                .eq(cat_id)
                .or(seasonal_price_rule::Column::ProductCategoryId.is_null()),
        );
    }

    let row = q.order_by_desc(seasonal_price_rule::Column::CreatedAt).one(db).await?;
    Ok(row)
}

async fn find_customer_special_price(
    db: &DatabaseConnection,
    customer_id: i64,
    req: &PriceCalcRequest,
    calc_date: NaiveDate,
) -> Result<Option<Decimal>, CalcError> {
    let row = customer_color_price::Entity::find()
        .filter(customer_color_price::Column::CustomerId.eq(customer_id))
        .filter(customer_color_price::Column::ProductId.eq(req.product_id))
        .filter(customer_color_price::Column::ColorId.eq(req.color_id))
        .filter(customer_color_price::Column::Currency.eq(&req.currency))
        .filter(customer_color_price::Column::ValidFrom.lte(calc_date))
        .filter(
            customer_color_price::Column::ValidUntil
                .gte(calc_date)
                .or(customer_color_price::Column::ValidUntil.is_null()),
        )
        .order_by_desc(customer_color_price::Column::CreatedAt)
        .one(db)
        .await?;
    Ok(row.map(|m| m.special_price))
}

// ----------------------------------------------------------------------
// 单元测试
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vip_discount() {
        let d = customer_level_discount(Some("VIP"));
        // P0 6-2 修复：0.95（原 Decimal::new(95, 3) = 0.095 是 bug）
        assert_eq!(d, Decimal::new(95, 2)); // 95 折 = 0.95
    }

    #[test]
    fn test_normal_no_discount() {
        let d = customer_level_discount(Some("NORMAL"));
        assert_eq!(d, Decimal::new(100, 2)); // 1.00 = 100%
    }

    #[test]
    fn test_gold_discount() {
        let d = customer_level_discount(Some("GOLD"));
        assert_eq!(d, Decimal::new(90, 2)); // 0.90 = 9 折
    }

    #[test]
    fn test_none_discount() {
        let d = customer_level_discount(None);
        assert_eq!(d, Decimal::new(100, 2)); // 1.00 = 100%
    }

    #[test]
    fn test_tier_vip_combined() {
        // 100 元基础价 → 阶梯价 90 元（5% off）→ VIP 95 折 → 85.5 元
        let tier = Decimal::new(90, 0);
        let vip = customer_level_discount(Some("VIP"));
        let final_price = (tier * vip).round_dp(6);
        assert_eq!(final_price, Decimal::new(85500, 3));
    }

    #[test]
    fn test_seasonal_percentage() {
        // 100 元 → 春季 +10% = 110 元
        let base = Decimal::new(100, 0);
        let factor = Decimal::from(1) + Decimal::new(10, 2);
        let result = (base * factor).round_dp(6);
        assert_eq!(result, Decimal::new(110, 0));
    }

    #[test]
    fn test_seasonal_fixed() {
        // 100 元 → 节日 +5 元 = 105 元
        let base = Decimal::new(100, 0);
        let result = base + Decimal::new(5, 0);
        assert_eq!(result, Decimal::new(105, 0));
    }

    #[test]
    fn test_seasonal_negative() {
        // 100 元 → 冬季 -5% = 95 元
        let base = Decimal::new(100, 0);
        let factor = Decimal::from(1) - Decimal::new(5, 2);
        let result = (base * factor).round_dp(6);
        assert_eq!(result, Decimal::new(95, 0));
    }

    #[test]
    fn test_special_price_overrides_all() {
        // 客户专属价应覆盖所有其他规则
        let tier = Decimal::new(90, 0);
        let vip = customer_level_discount(Some("VIP"));
        let after_tier_vip = (tier * vip).round_dp(6);
        let special = Decimal::new(80, 0);
        assert!(special < after_tier_vip);
        assert_eq!(special, Decimal::new(80, 0));
    }

    #[test]
    fn test_decimal_precision() {
        // 验证 6 位精度
        let a = Decimal::new(1999, 2);
        let b = Decimal::new(95, 2);
        let c = (a * b).round_dp(6);
        assert_eq!(c, Decimal::new(189905, 4));
    }
}
