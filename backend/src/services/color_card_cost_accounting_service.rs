//! 色卡成本核算服务
//! V15 P2 类九 10.3-4：成本归集/结转/恢复/损失核算
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

#[allow(dead_code)]
pub struct ColorCardCostAccountingService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code)]
impl ColorCardCostAccountingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 制作成本归集
    pub async fn collect_production_cost(&self, _color_card_id: i32) -> Result<Decimal, AppError> {
        Ok(Decimal::ZERO)
    }

    /// 发放成本结转
    pub async fn transfer_issue_cost(&self, _issue_record_id: i32) -> Result<Decimal, AppError> {
        Ok(Decimal::ZERO)
    }

    /// 取消发放恢复成本
    pub async fn restore_cost_on_cancel(&self, _issue_record_id: i32) -> Result<(), AppError> {
        Ok(())
    }

    /// 过期损失核算
    pub async fn calculate_expiry_loss(&self, _issue_record_id: i32) -> Result<Decimal, AppError> {
        Ok(Decimal::ZERO)
    }
}
