//! 报价单生命周期 impl 子模块（quotation_ops/lifecycle）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移生命周期相关方法。
//! 包含 cancel（取消报价单）+ generate_quotation_no_txn（在写入事务内生成报价单号）。

use chrono::Utc;
use sea_orm::{DatabaseTransaction, EntityTrait, QuerySelect, Set, TransactionTrait};

use crate::models::sales_quotation::{
    self, ActiveModel as QuotationActive, Entity as QuotationEntity,
};
use crate::models::status::quotation as quotation_status;
use crate::services::quotation_service::{QuotationService, ServiceError};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

impl QuotationService {
    /// 取消报价单（任意非 converted 状态可取消）
    /// 批次 26 v6 P1：状态门+update 移入 txn+lock_exclusive 串行化并发；批次 94 用 update_with_audit 记审计
    pub async fn cancel(&self, id: i64, user_id: i64) -> Result<sales_quotation::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let existing = QuotationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;
        if existing.status == "converted" {
            return Err(AppError::validation_displayable(
                "当前状态不允许此操作".to_string(),
            ));
        }
        if existing.status == quotation_status::CANCELLED {
            return Ok(existing);
        }

        let mut active: QuotationActive = existing.into();
        active.status = Set(quotation_status::CANCELLED.to_string());
        active.updated_at = Set(Utc::now());
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "quotation",
            active,
            Some(user_id as i32),
        )
        .await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 在写入事务内生成报价单号：QT + YYYYMMDD + 4 位当日流水。
    ///
    /// 走通用单号生成器的「事务内 + 按前缀+日期取 pg_advisory_xact_lock」路径：advisory 锁
    /// 持续到本事务提交，串行化并发的「计数→插入」，杜绝并发建单读到同一当日计数、
    /// 各自拼出相同 QT 号再撞 `sales_quotations_quotation_no_key` 唯一约束（表现为 500）。
    /// 必须在 `create_draft` 的插入事务中调用，使锁覆盖到插入完成。
    pub(crate) async fn generate_quotation_no_txn(
        txn: &DatabaseTransaction,
    ) -> Result<String, ServiceError> {
        DocumentNumberGenerator::generate_no_with_width_txn(
            txn,
            "QT",
            QuotationEntity,
            sales_quotation::Column::QuotationNo,
            4,
        )
        .await
        .map_err(ServiceError::App)
    }
}
