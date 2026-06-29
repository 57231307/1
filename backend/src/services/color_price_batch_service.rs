//! 面料多色号定价扩展 - 批量调价 Service
//!
//! 批量调价 + 审批（>10% 涨跌幅需经理审批）
//! 创建时间: 2026-06-18
//! 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §6.1

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_price_dto::{ApproveColorPriceDto, BatchAdjustPriceDto, BatchAdjustResult};
use crate::models::color_price_history::{self, ActiveModel as HistoryActive};
use crate::models::product_color_price::{
    self, ActiveModel as ColorPriceActive, Entity as ColorPriceEntity,
};

/// 业务错误
#[derive(Debug, Error)]
pub enum BatchError {
    #[error("色号价格不存在: id={0}")]
    PriceNotFound(i64),
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 调价审批阈值（涨跌幅 > 10% 需经理审批）
pub const APPROVAL_THRESHOLD: f64 = 0.10;

/// 批量调价服务
pub struct ColorPriceBatchService {
    db: Arc<DatabaseConnection>,
}

impl ColorPriceBatchService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &crate::utils::app_state::AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 批量调价
    pub async fn batch_adjust(
        &self,
        dto: BatchAdjustPriceDto,
        operated_by: i64,
    ) -> Result<BatchAdjustResult, BatchError> {
        let mut auto_approved: Vec<i64> = Vec::new();
        let mut pending_approval: Vec<i64> = Vec::new();
        let total = dto.items.len();

        for item in dto.items.iter() {
            // 1. 查找色号价格
            let existing = ColorPriceEntity::find_by_id(item.price_id)
                .one(&*self.db)
                .await?
                .ok_or(BatchError::PriceNotFound(item.price_id))?;

            // 2. 计算新价
            let new_price = calculate_new_price(existing.base_price, &item.adjustment_type, item.adjustment_value)?;
            let change_percent = if existing.base_price.is_zero() {
                Decimal::ZERO
            } else {
                ((new_price - existing.base_price) / existing.base_price).round_dp(4)
            };

            // 3. 判断是否需审批
            let need_approval = change_percent.abs() > Decimal::new(APPROVAL_THRESHOLD as i64 * 10000, 4);

            // 4. 写历史
            let history = HistoryActive {
                id: Default::default(),
                product_color_price_id: Set(existing.id),
                old_price: Set(existing.base_price),
                new_price: Set(new_price),
                currency: Set(existing.currency.clone()),
                change_type: Set("batch".to_string()),
                change_reason: Set(dto.change_reason.clone()),
                change_percent: Set(Some(change_percent)),
                quantity: Set(None),
                operated_by: Set(operated_by),
                operated_at: Set(Utc::now()),
                approved_by: Set(None),
                approved_at: Set(None),
            };
            history.insert(&*self.db).await?;

            if need_approval {
                // 标记 PENDING，不更新 base_price
                let mut active: ColorPriceActive = existing.into();
                active.approval_status = Set("PENDING".to_string());
                active.updated_at = Set(Utc::now());
                active.update(&*self.db).await?;
                pending_approval.push(item.price_id);
            } else {
                // 自动通过，直接更新 base_price
                let mut active: ColorPriceActive = existing.into();
                active.base_price = Set(new_price);
                active.approved_by = Set(Some(operated_by));
                active.approved_at = Set(Some(Utc::now()));
                active.approval_status = Set("APPROVED".to_string());
                active.updated_at = Set(Utc::now());
                active.update(&*self.db).await?;
                auto_approved.push(item.price_id);
            }
        }

        Ok(BatchAdjustResult {
            auto_approved,
            pending_approval,
            total,
        })
    }

    /// 审批
    pub async fn approve(
        &self,
        id: i64,
        approved_by: i64,
        dto: ApproveColorPriceDto,
    ) -> Result<product_color_price::Model, BatchError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;

        let existing = ColorPriceEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BatchError::PriceNotFound(id))?;

        if existing.approval_status != "PENDING" {
            return Err(BatchError::Validation(format!(
                "价格不处于待审批状态（当前: {}）",
                existing.approval_status
            )));
        }

        let new_status = match dto.decision.as_str() {
            "APPROVED" => "APPROVED",
            "REJECTED" => "REJECTED",
            _ => return Err(BatchError::Validation(format!(
                "无效的审批决定: {}（允许: APPROVED / REJECTED）",
                dto.decision
            ))),
        };

        // 找到最近一次历史（在事务内查询以避免脏读）
        let last_history = color_price_history::Entity::find()
            .filter(color_price_history::Column::ProductColorPriceId.eq(id))
            .order_by_desc(color_price_history::Column::OperatedAt)
            .one(&txn)
            .await?;

        let mut active: ColorPriceActive = existing.clone().into();
        active.approval_status = Set(new_status.to_string());
        active.approved_by = Set(Some(approved_by));
        active.approved_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());

        if new_status == "APPROVED" {
            if let Some(h) = last_history.as_ref() {
                active.base_price = Set(h.new_price);
            }
        }
        // 使用 update_with_audit 在事务内同步写入审计日志
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(approved_by as i32),
        )
        .await
        .map_err(|e| BatchError::Validation(e.to_string()))?;

        // 更新历史记录的 approved_by（在事务内）
        if let Some(h) = last_history.as_ref() {
            let mut history_active: HistoryActive = h.clone().into();
            history_active.approved_by = Set(Some(approved_by));
            history_active.approved_at = Set(Some(Utc::now()));
            history_active.update(&txn).await?;
        }

        txn.commit().await?;

        Ok(result)
    }
}

/// 计算新价格
fn calculate_new_price(
    base: Decimal,
    adjustment_type: &str,
    adjustment_value: Decimal,
) -> Result<Decimal, BatchError> {
    match adjustment_type {
        "percentage" => {
            let factor = Decimal::from(1) + adjustment_value;
            Ok((base * factor).round_dp(6))
        }
        "fixed" => Ok(base + adjustment_value),
        _ => Err(BatchError::Validation(format!(
            "无效的调整方式: {}（允许: percentage / fixed）",
            adjustment_type
        ))),
    }
}

