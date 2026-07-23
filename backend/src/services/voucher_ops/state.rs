//! 凭证服务-状态流转子模块（voucher_ops/state）
//!
//! 批次 488 D10-4 拆分：从原 `voucher_service.rs` L523-678 迁移。
//! 包含 3 个状态流转方法：
//! - submit（draft → submitted）
//! - review（submitted → reviewed）
//! - post（reviewed → posted，核心过账逻辑，调用 balance/assist 子模块）
//!
//! 跨子模块调用（pub(crate) 方法）：
//! - submit/review → balance::validate_voucher
//! - post → balance::validate_voucher_in_transaction / update_account_balances
//! - post → assist::write_assist_accounting_records_txn

use chrono::Datelike;
use sea_orm::{EntityTrait, IntoActiveModel, QuerySelect, TransactionTrait};
use tracing::info;

use crate::models::voucher;
use crate::utils::error::AppError;

use crate::services::voucher_service::VoucherService;

impl VoucherService {
    /// 提交凭证
    // 批次 24 v6 P1-4 修复：将 _user_id 改为 user_id，传入审计日志追溯操作人。
    // 原签名用下划线前缀表示未使用，导致审计日志 user_id 硬编码为 0 无法追溯。
    pub async fn submit(&self, id: i32, user_id: i32) -> Result<voucher::Model, AppError> {
        info!("提交凭证 ID: {}", id);

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_by_id 裸查询，未加行锁，并发提交可能双写状态。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更。
        let txn = (*self.db).begin().await?;
        let voucher = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        if voucher.status != crate::models::status::voucher::VOUCHER_DRAFT {
            return Err(AppError::bad_request(
                "只有草稿状态的凭证可以提交".to_string(),
            ));
        }

        // 提交前验证借贷平衡
        self.validate_voucher(id).await?;

        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set(crate::models::status::voucher::VOUCHER_SUBMITTED.to_string());

        // 批次 11（2026-06-28）：事务包裹"凭证状态更新 + 审计日志"，保证原子性
        // 原 update_with_audit(&*self.db, ...) 内部 2 次独立写入非原子，
        // 审计插入失败会导致"凭证已提交但审计缺失"
        // 批次 24 v6 P1-4 修复：user_id 从 0 改为传入的真实值
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
        )
        .await?;
        txn.commit().await?;

        info!("凭证提交成功：no={}", updated.voucher_no);
        Ok(updated)
    }

    /// 审核凭证
    pub async fn review(&self, id: i32, user_id: i32) -> Result<voucher::Model, AppError> {
        info!("审核凭证 ID: {}", id);

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_by_id 裸查询，未加行锁，并发审核可能双写状态。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更。
        let txn = (*self.db).begin().await?;
        let voucher = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        if voucher.status != crate::models::status::voucher::VOUCHER_SUBMITTED {
            return Err(AppError::bad_request("只有已提交的凭证可以审核"));
        }

        // 验证借贷平衡
        self.validate_voucher(id).await?;

        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set(crate::models::status::voucher::VOUCHER_REVIEWED.to_string());
        active_model.reviewed_by = sea_orm::Set(Some(user_id));
        active_model.reviewed_at = sea_orm::Set(Some(chrono::Utc::now()));

        // 批次 11（2026-06-28）：事务包裹"凭证审核状态更新 + 审计日志"，保证原子性
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;
        txn.commit().await?;

        info!("凭证审核成功：no={}", updated.voucher_no);
        Ok(updated)
    }

    /// 凭证过账（核心功能）
    pub async fn post(&self, id: i32, user_id: i32) -> Result<voucher::Model, AppError> {
        info!("凭证过账 ID: {}", id);

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_by_id 裸查询，未加行锁，并发过账可能双写状态。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更。
        let txn = (*self.db).begin().await?;
        let voucher = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        if voucher.status != crate::models::status::voucher::VOUCHER_REVIEWED {
            return Err(AppError::bad_request("只有已审核的凭证可以过账"));
        }

        // 检查期间锁定
        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        period_svc.check_date_locked(voucher.voucher_date).await?;

        // 1. 验证凭证
        self.validate_voucher_in_transaction(id, &txn).await?;

        // 2. 更新科目余额
        // 批次 94 P2-10：传入 user_id 用于余额变更审计日志
        self.update_account_balances(id, user_id, &txn).await?;

        // 2.5 F-P1-3 修复（批次 359 v13 复审）：写入辅助核算记录
        // 原实现仅更新科目余额（account_balance），未写入 assist_accounting_record 表，
        // 导致辅助核算明细账与汇总表查询无数据。仅对包含辅助核算维度的分录写入。
        self.write_assist_accounting_records_txn(id, user_id, &txn)
            .await?;

        // 3. 更新凭证状态
        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set(crate::models::status::voucher::VOUCHER_POSTED.to_string());
        active_model.posted_by = sea_orm::Set(Some(user_id));
        active_model.posted_at = sea_orm::Set(Some(chrono::Utc::now()));
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 提交事务
        txn.commit().await?;

        info!("凭证过账成功：no={}", updated.voucher_no);

        // 触发财务指标更新事件
        let period = format!(
            "{:04}-{:02}",
            updated.voucher_date.year(),
            updated.voucher_date.month()
        );
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::FinancialIndicatorUpdate {
                period,
                trigger_source: format!("voucher_posted:{}", updated.voucher_no),
            },
        );

        Ok(updated)
    }
}
