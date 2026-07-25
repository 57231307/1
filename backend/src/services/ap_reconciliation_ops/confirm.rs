//! 供应商对账状态流转 impl 子模块（ap_reconciliation_ops/confirm）
//!
//! D10-5 拆分：从原 `ap_reconciliation_service.rs` 迁移。
//! 包含 ApReconciliationService 的 2 个状态机方法：
//! - confirm_reconciliation（PENDING → CONFIRMED，lock_exclusive 串行化，确认后生成对账确认凭证）
//! - dispute（非 CONFIRMED → DISPUTED，lock_exclusive 串行化）
//!
//! 并发状态变更通过 lock_exclusive 串行化（批次 27 v7 P0 修复）。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{EntityTrait, QuerySelect, Set, TransactionTrait};

use crate::models::ap_reconciliation;
use crate::models::status::ap_reconciliation as reconciliation_status;
use crate::services::ap_reconciliation_service::ApReconciliationService;
use crate::utils::error::AppError;

impl ApReconciliationService {
    /// 确认对账单
    ///
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    /// 原实现已有 txn 但状态门查询未加 lock_exclusive，两并发 confirm_reconciliation 同时通过
    /// PENDING 检查后基于过期状态写入，导致 confirmed_by/confirmed_at 被覆盖。
    pub async fn confirm_reconciliation(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询对账单（加 lock_exclusive 串行化并发 confirm_reconciliation）
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("对账单 {}", id)))?;

        // 2. 检查状态
        if reconciliation.reconciliation_status != reconciliation_status::PENDING {
            return Err(AppError::business(format!(
                "对账单状态为{}，不可确认",
                reconciliation.reconciliation_status
            )));
        }

        // 3. 确认对账单（带审计）
        let reconciliation = Self::apply_confirmed_status(&txn, reconciliation, user_id).await?;
        txn.commit().await?;

        // 4. 生成对账确认凭证（失败仅 warn 不阻断主流程）
        self.try_create_confirmation_voucher(&reconciliation, user_id)
            .await;

        Ok(reconciliation)
    }

    /// 写入 CONFIRMED 状态并附审计日志
    async fn apply_confirmed_status(
        txn: &sea_orm::DatabaseTransaction,
        reconciliation: ap_reconciliation::Model,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let now = Utc::now();
        let mut reconciliation_active: ap_reconciliation::ActiveModel = reconciliation.into();
        reconciliation_active.reconciliation_status = Set(reconciliation_status::CONFIRMED.to_string());
        reconciliation_active.confirmed_by = Set(Some(user_id));
        reconciliation_active.confirmed_at = Set(Some(now));

        Ok(crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            reconciliation_active,
            Some(user_id),
        )
        .await?)
    }

    /// 生成对账确认凭证（失败仅 warn 不阻断主流程）
    ///
    /// F-P2-4 修复（批次 387 v13 复审）：commit 成功后生成转账凭证
    ///（借贷均为应付账款，金额=期末余额），作为对账确认的审计凭证，不改变账面净余额。
    async fn try_create_confirmation_voucher(
        &self,
        reconciliation: &ap_reconciliation::Model,
        user_id: i32,
    ) {
        let voucher_req = Self::build_confirmation_voucher_request(reconciliation);
        let voucher_service = crate::services::voucher_service::VoucherService::new(self.db.clone());
        if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
            tracing::warn!(
                "对账单 {} 确认成功，但生成对账确认凭证失败：{}",
                reconciliation.reconciliation_no,
                e
            );
        }
    }

    /// 构建对账确认凭证请求
    fn build_confirmation_voucher_request(
        reconciliation: &ap_reconciliation::Model,
    ) -> crate::services::voucher_service::CreateVoucherRequest {
        let summary = format!("对账确认-{}", reconciliation.reconciliation_no);
        crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "转".to_string(),
            voucher_date: reconciliation.end_date,
            source_type: Some("AP_RECONCILIATION".to_string()),
            source_module: Some("ap".to_string()),
            source_bill_id: Some(reconciliation.id),
            source_bill_no: Some(reconciliation.reconciliation_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                Self::build_voucher_item(1, reconciliation.closing_balance, Decimal::ZERO, &summary, reconciliation.supplier_id),
                Self::build_voucher_item(2, Decimal::ZERO, reconciliation.closing_balance, &summary, reconciliation.supplier_id),
            ],
        }
    }

    /// 构建对账确认凭证分录
    fn build_voucher_item(
        line_no: i32,
        debit: Decimal,
        credit: Decimal,
        summary: &str,
        supplier_id: i32,
    ) -> crate::services::voucher_service::VoucherItemRequest {
        crate::services::voucher_service::VoucherItemRequest {
            line_no: Some(line_no),
            subject_code: Some("2202".to_string()),
            subject_name: Some("应付账款".to_string()),
            debit,
            credit,
            summary: Some(summary.to_string()),
            assist_customer_id: None,
            assist_supplier_id: Some(supplier_id),
            assist_department_id: None,
            assist_employee_id: None,
            assist_project_id: None,
            assist_batch_id: None,
            assist_color_no_id: None,
            assist_dye_lot_id: None,
            assist_grade: None,
            assist_workshop_id: None,
            quantity_meters: None,
            quantity_kg: None,
            unit_price: None,
        }
    }

    /// 提出争议
    ///
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    /// 原实现已有 txn 但状态门查询未加 lock_exclusive，两并发 dispute 同时通过门控后
    /// 基于过期状态写入，导致 disputed_reason 被覆盖。
    pub async fn dispute(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询对账单（加 lock_exclusive 串行化并发 dispute）
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("对账单 {}", id)))?;

        // 2. 检查状态
        if reconciliation.reconciliation_status == reconciliation_status::CONFIRMED {
            return Err(AppError::business("对账单已确认，不可提出争议".to_string()));
        }

        // 3. 提出争议
        let now = Utc::now();
        let mut reconciliation_active: ap_reconciliation::ActiveModel = reconciliation.into();
        reconciliation_active.reconciliation_status = Set(reconciliation_status::DISPUTED.to_string());
        reconciliation_active.disputed_by = Set(Some(user_id));
        reconciliation_active.disputed_at = Set(Some(now));
        reconciliation_active.disputed_reason = Set(Some(reason));

        let reconciliation =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                reconciliation_active,
                Some(user_id),
            )
            .await?;

        txn.commit().await?;

        Ok(reconciliation)
    }
}
