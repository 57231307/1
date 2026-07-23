//! 应收对账 - 客户确认/争议（ar/vfy_ops/confirm）
//!
//! 批次 490 D10-4b 拆分自原 `ar/vfy.rs` 的 `customer_confirm` / `customer_dispute` 方法。
//! 职责：带状态机校验（lock_exclusive 串行化并发）的客户对账单确认与争议提交，
//! 审计日志经 `AuditLogService::update_with_audit` 写入。
//! 本模块扩展 `ArReconciliationService` 的 `customer_confirm` 与 `customer_dispute` 公开方法。

use chrono::Utc;
use sea_orm::{EntityTrait, QuerySelect, Set, TransactionTrait};
use tracing::info;

use crate::models::ar_reconciliation::{ActiveModel, Entity as ReconciliationEntity};
use crate::models::status::ar as ar_status;
use crate::utils::error::AppError;

use super::super::ArReconciliationService;

impl ArReconciliationService {
    /// 客户确认对账单（带状态校验）
    ///
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    /// 原实现完全无 txn 无 lock，两并发 customer_confirm 同时通过状态门后基于过期状态写入，
    /// 导致 confirmed_by/confirmed_at 被覆盖、审计日志完全丢失（未走 update_with_audit）。
    pub async fn customer_confirm(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<crate::models::ar_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let status = model.reconciliation_status.as_deref().unwrap_or(ar_status::RECONCILIATION_DRAFT);
        if status == ar_status::RECONCILIATION_CONFIRMED {
            return Err(AppError::business("对账单已确认，不可重复确认".to_string()));
        }
        if status == ar_status::RECONCILIATION_DISPUTED {
            return Err(AppError::business(
                "对账单存在争议，请先解决争议后再确认".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(ar_status::RECONCILIATION_CONFIRMED.to_string()));
        active_model.confirmed_by_customer = Set(Some(true));
        active_model.confirmed_by = Set(Some(user_id));
        active_model.confirmed_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("客户确认对账单成功：id={}", id);
        Ok(updated)
    }

    /// 客户提出争议（带状态校验）
    ///
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    /// 原实现完全无 txn 无 lock，两并发 customer_dispute 同时通过状态门后基于过期状态写入，
    /// 导致 dispute_reason 被覆盖、审计日志完全丢失（未走 update_with_audit）。
    pub async fn customer_dispute(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<crate::models::ar_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let status = model.reconciliation_status.as_deref().unwrap_or(ar_status::RECONCILIATION_DRAFT);
        if status == ar_status::RECONCILIATION_CONFIRMED {
            return Err(AppError::business("对账单已确认，不可提出争议".to_string()));
        }
        if status == ar_status::RECONCILIATION_CLOSED {
            return Err(AppError::business("对账单已关闭，不可提出争议".to_string()));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(ar_status::RECONCILIATION_DISPUTED.to_string()));
        active_model.dispute_reason = Set(Some(reason.clone()));
        active_model.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("客户对账单提出争议：id={}, reason={}", id, reason);
        Ok(updated)
    }
}
