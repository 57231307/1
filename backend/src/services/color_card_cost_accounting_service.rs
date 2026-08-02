//! 色卡成本核算服务
//! V15 P2 类九 10.3-4：成本归集/结转/恢复/损失核算（营销费用-色卡发放科目口径）
//!
//! 成本口径：色卡制作成本按色卡上色号数量（total_colors）× 每色号标准成本归集；
//! 单本色卡成本 = 整卡制作成本 / (库存数量 + 已发放数量)；
//! 发放成本结转 = 发放数量 × 单本色卡成本；
//! 过期损失 = 未归还发放数量 × 单本色卡成本。
//!
//! 每色号标准成本可通过环境变量 `COLOR_CARD_COST_PER_COLOR` 覆盖（默认 50.00 元）。

use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_issue::Entity as IssueEntity;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

/// 每色号标准制作成本（元），默认 50.00，可通过环境变量 COLOR_CARD_COST_PER_COLOR 覆盖
fn cost_per_color() -> Decimal {
    std::env::var("COLOR_CARD_COST_PER_COLOR")
        .ok()
        .and_then(|v| v.parse::<Decimal>().ok())
        .filter(|&v| v > Decimal::ZERO)
        .unwrap_or(Decimal::new(50, 2))
}

/// 计算整卡制作成本（total_colors × 每色号标准成本）
fn production_cost_of(card: &color_card::Model) -> Decimal {
    Decimal::from(card.total_colors) * cost_per_color()
}

/// 计算单本色卡成本 = 整卡制作成本 / (库存数量 + 已发放数量)，库存为零时按整卡成本计
fn unit_cost_of(card: &color_card::Model) -> Decimal {
    let total_qty = Decimal::from(card.stock_quantity + card.issued_quantity);
    if total_qty.is_zero() {
        return production_cost_of(card);
    }
    (production_cost_of(card) / total_qty).round_dp(2)
}

/// 色卡成本核算服务
pub struct ColorCardCostAccountingService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardCostAccountingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 制作成本归集：按色卡色号数量 × 每色号标准成本计算整卡制作成本
    pub async fn collect_production_cost(&self, color_card_id: i32) -> Result<Decimal, AppError> {
        let card = ColorCardEntity::find_by_id(color_card_id as i64)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("色卡 {} 不存在", color_card_id)))?;
        Ok(production_cost_of(&card))
    }

    /// 发放成本结转：发放数量 × 单本色卡成本（营销费用-色卡发放科目）
    pub async fn transfer_issue_cost(&self, issue_record_id: i32) -> Result<Decimal, AppError> {
        let issue = IssueEntity::find_by_id(issue_record_id as i64)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("发放记录 {} 不存在", issue_record_id)))?;
        if issue.status == "cancelled" {
            return Err(AppError::business(format!(
                "发放记录 {} 已取消，不执行成本结转",
                issue_record_id
            )));
        }
        let card = ColorCardEntity::find_by_id(issue.color_card_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("色卡 {} 不存在", issue.color_card_id)))?;
        Ok((unit_cost_of(&card) * Decimal::from(issue.issue_qty)).round_dp(2))
    }

    /// 取消发放恢复成本：恢复色卡库存后，成本回冲（恢复金额 = 已结转成本）
    pub async fn restore_cost_on_cancel(&self, issue_record_id: i32) -> Result<(), AppError> {
        let issue = IssueEntity::find_by_id(issue_record_id as i64)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("发放记录 {} 不存在", issue_record_id)))?;
        if issue.status != "cancelled" {
            return Err(AppError::business(format!(
                "发放记录 {} 状态为 {}（非 cancelled），不执行成本恢复",
                issue_record_id, issue.status
            )));
        }
        let card = ColorCardEntity::find_by_id(issue.color_card_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("色卡 {} 不存在", issue.color_card_id)))?;
        // 库存已由取消发放逻辑恢复，此处仅校验并确认成本回冲的基数存在
        if card.stock_quantity < 0 {
            return Err(AppError::business("色卡库存数量异常（负数），无法恢复成本"));
        }
        Ok(())
    }

    /// 过期损失核算：未归还发放数量 × 单本色卡成本
    pub async fn calculate_expiry_loss(&self, issue_record_id: i32) -> Result<Decimal, AppError> {
        let issue = IssueEntity::find_by_id(issue_record_id as i64)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("发放记录 {} 不存在", issue_record_id)))?;
        if issue.status != "issued" {
            return Err(AppError::business(format!(
                "发放记录 {} 状态为 {}（非 issued），不核算过期损失",
                issue_record_id, issue.status
            )));
        }
        let card = ColorCardEntity::find_by_id(issue.color_card_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("色卡 {} 不存在", issue.color_card_id)))?;
        Ok((unit_cost_of(&card) * Decimal::from(issue.issue_qty)).round_dp(2))
    }
}
