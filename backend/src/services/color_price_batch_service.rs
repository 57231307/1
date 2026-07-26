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

use crate::models::color_price_dto::{
    ApproveColorPriceDto, BatchAdjustItem, BatchAdjustPriceDto, BatchAdjustResult,
};
use crate::models::color_price_history::{self, ActiveModel as HistoryActive};
use crate::models::product_color_price::{
    self, ActiveModel as ColorPriceActive, Entity as ColorPriceEntity,
};
// 批次 158 v11 真实接入：审批状态常量替代字符串字面量
use crate::models::status::approval;

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

// v11 批次 147 P2-B：移除失效的 dead_code 标注（被 handlers/color_price_handler.rs:211,232 真实调用）
impl ColorPriceBatchService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &crate::utils::app_state::AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 批量调价（批量查询 + 逐条计算 + 批量插入历史 + 逐条更新）
    pub async fn batch_adjust(
        &self,
        dto: BatchAdjustPriceDto,
        operated_by: i64,
    ) -> Result<BatchAdjustResult, BatchError> {
        let total = dto.items.len();
        let mut price_map = self.fetch_prices_map(&dto.items).await?;

        let mut history_models: Vec<HistoryActive> = Vec::with_capacity(total);
        let mut auto_update_models: Vec<(ColorPriceActive, i64)> = Vec::new();
        let mut pending_update_models: Vec<(ColorPriceActive, i64)> = Vec::new();

        for item in dto.items.iter() {
            let existing = price_map
                .remove(&item.price_id)
                .ok_or(BatchError::PriceNotFound(item.price_id))?;
            let (history, active, need_approval) =
                Self::build_adjustment_models(existing, item, &dto.change_reason, operated_by)?;
            history_models.push(history);
            if need_approval {
                pending_update_models.push((active, item.price_id));
            } else {
                auto_update_models.push((active, item.price_id));
            }
        }

        self.insert_history_records(history_models).await?;
        let (auto_approved, pending_approval) = self
            .apply_price_updates(auto_update_models, pending_update_models)
            .await?;

        Ok(BatchAdjustResult {
            auto_approved,
            pending_approval,
            total,
        })
    }

    /// 批量查询色号价格并构建 id→Model 映射（1 次查询替代 N 次 find_by_id）
    async fn fetch_prices_map(
        &self,
        items: &[BatchAdjustItem],
    ) -> Result<std::collections::HashMap<i64, product_color_price::Model>, BatchError> {
        let price_ids: Vec<i64> = items.iter().map(|i| i.price_id).collect();
        let existing = ColorPriceEntity::find()
            .filter(product_color_price::Column::Id.is_in(price_ids))
            .all(&*self.db)
            .await?;
        Ok(existing.into_iter().map(|p| (p.id, p)).collect())
    }

    /// 构建单条调价的历史记录与 ActiveModel（含审批阈值判断）
    fn build_adjustment_models(
        existing: product_color_price::Model,
        item: &BatchAdjustItem,
        change_reason: &Option<String>,
        operated_by: i64,
    ) -> Result<(HistoryActive, ColorPriceActive, bool), BatchError> {
        let new_price = calculate_new_price(
            existing.base_price,
            &item.adjustment_type,
            item.adjustment_value,
        )?;
        let change_percent = if existing.base_price.is_zero() {
            Decimal::ZERO
        } else {
            ((new_price - existing.base_price) / existing.base_price).round_dp(4)
        };
        let need_approval =
            change_percent.abs() > Decimal::new(APPROVAL_THRESHOLD as i64 * 10000, 4);

        let history = HistoryActive {
            id: Default::default(),
            product_color_price_id: Set(existing.id),
            old_price: Set(existing.base_price),
            new_price: Set(new_price),
            currency: Set(existing.currency.clone()),
            change_type: Set("batch".to_string()),
            change_reason: Set(change_reason.clone()),
            change_percent: Set(Some(change_percent)),
            quantity: Set(None),
            operated_by: Set(operated_by),
            operated_at: Set(Utc::now()),
            approved_by: Set(None),
            approved_at: Set(None),
        };

        let mut active: ColorPriceActive = existing.into();
        if need_approval {
            active.approval_status = Set(approval::PENDING.to_string());
        } else {
            active.base_price = Set(new_price);
            active.approved_by = Set(Some(operated_by));
            active.approved_at = Set(Some(Utc::now()));
            active.approval_status = Set(approval::APPROVED.to_string());
        }
        active.updated_at = Set(Utc::now());
        Ok((history, active, need_approval))
    }

    /// 批量插入调价历史记录（空集合跳过）
    async fn insert_history_records(
        &self,
        history_models: Vec<HistoryActive>,
    ) -> Result<(), BatchError> {
        if history_models.is_empty() {
            return Ok(());
        }
        color_price_history::Entity::insert_many(history_models)
            .exec(&*self.db)
            .await?;
        Ok(())
    }

    /// 逐条更新色号价格并收集自动通过/待审批 ID 列表
    async fn apply_price_updates(
        &self,
        auto_update_models: Vec<(ColorPriceActive, i64)>,
        pending_update_models: Vec<(ColorPriceActive, i64)>,
    ) -> Result<(Vec<i64>, Vec<i64>), BatchError> {
        let mut auto_approved: Vec<i64> = Vec::new();
        let mut pending_approval: Vec<i64> = Vec::new();
        for (active, price_id) in auto_update_models {
            active.update(&*self.db).await?;
            auto_approved.push(price_id);
        }
        for (active, price_id) in pending_update_models {
            active.update(&*self.db).await?;
            pending_approval.push(price_id);
        }
        Ok((auto_approved, pending_approval))
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

        if existing.approval_status != approval::PENDING {
            return Err(BatchError::Validation(format!(
                "价格不处于待审批状态（当前: {}）",
                existing.approval_status
            )));
        }

        let new_status = match dto.decision.as_str() {
            approval::APPROVED => approval::APPROVED,
            approval::REJECTED => approval::REJECTED,
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

        if new_status == approval::APPROVED {
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

