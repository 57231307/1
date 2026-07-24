//! 应收对账单 - 状态机操作（ar/recon_ops/lifecycle）
//!
//! 批次 D10 拆分自原 `ar/recon.rs` 的 delete / send / close / update_status 方法。
//! 职责：对账单状态流转（draft → sent → confirmed/disputed → closed），
//! 含状态门控、lock_exclusive 串行化、审计日志与关闭后凭证生成。
//! 本模块扩展 `ArReconciliationService` 的状态机公开方法。
//!
//! 批次 108 P1-6 修复：delete/send/close 方法已通过 /ar-reconciliations 路由接入业务
//! （ar_reconciliation_handler.rs + routes/finance.rs），移除 dead_code 标注。

use chrono::Utc;
use sea_orm::{EntityTrait, QuerySelect, Set, TransactionTrait};

use crate::models::ar_reconciliation::{
    ActiveModel, Entity as ReconciliationEntity, Model as ReconciliationModel,
};
use crate::models::status::ar as ar_status;
use crate::utils::error::AppError;

use super::super::ArReconciliationService;

impl ArReconciliationService {
    /// 删除对账单
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 93 P1-2 修复：状态门 + delete 移入同一事务，补 lock_exclusive 串行化并发
        // 原实现 find_by_id 在 self.db → 状态门 → delete_with_audit 在 self.db，
        // 状态门与 delete 跨事务边界，并发 delete + send 会竞态绕过 draft 状态门控。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        // 只有草稿状态的对账单可以删除（状态门在 txn 内，基于 lock_exclusive 读出的 model）
        if model.reconciliation_status.as_deref() != Some(ar_status::RECONCILIATION_DRAFT) {
            return Err(AppError::business(
                "只有草稿状态的对账单可以删除".to_string(),
            ));
        }

        // P0 8-3 修复：delete 操作补审计日志
        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            ReconciliationEntity,
            _,
        >(&txn, "ar_reconciliation", id, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 发送对账单
    pub async fn send(&self, id: i32, user_id: i32) -> Result<ReconciliationModel, AppError> {
        // P1 3-3 修复（批次 61）：状态机 lock_exclusive 补全，串行化并发发送
        // 原实现无 txn 无 lock，状态门在事务外，并发 send 会竞态绕过 draft 状态门控。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        if model.reconciliation_status.as_deref() != Some(ar_status::RECONCILIATION_DRAFT) {
            return Err(AppError::business(
                "只有草稿状态的对账单可以发送".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(ar_status::RECONCILIATION_SENT.to_string()));
        active_model.updated_at = Set(Utc::now());

        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        let updated =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                active_model,
                Some(user_id),
            )
            .await?;

        txn.commit().await?;

        Ok(updated)
    }

    /// 关闭对账单
    pub async fn close(&self, id: i32, user_id: i32) -> Result<ReconciliationModel, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现状态变更无 txn 无 lock，状态门控（confirmed/disputed → closed）在并发场景下
        // 会被竞态绕过：两并发 close 同时通过门控后基于过期状态写入。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let status = model.reconciliation_status.as_deref().unwrap_or(ar_status::RECONCILIATION_DRAFT);
        if status != ar_status::RECONCILIATION_CONFIRMED && status != ar_status::RECONCILIATION_DISPUTED {
            return Err(AppError::business(
                "只有已确认或有争议的对账单可以关闭".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(ar_status::RECONCILIATION_CLOSED.to_string()));
        active_model.updated_at = Set(Utc::now());

        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // F-P2-4 修复（批次 387 v13 复审）：AR 对账单关闭后生成对账确认凭证
        // 原实现 close 仅更新对账单状态，不生成凭证，
        // 导致对账确认结果无法在凭证体系中追溯。
        // 修复：commit 成功后生成转账凭证（借贷均为应收账款，金额=期末余额），
        // 作为对账确认的审计凭证，不改变账面净余额。失败时仅 warn 不阻断主流程。
        let voucher_req = crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "转".to_string(),
            voucher_date: result.period_end,
            source_type: Some("AR_RECONCILIATION".to_string()),
            source_module: Some("ar".to_string()),
            source_bill_id: Some(result.id),
            source_bill_no: Some(result.reconciliation_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: Some(1),
                    subject_code: Some("1131".to_string()),
                    subject_name: Some("应收账款".to_string()),
                    debit: result.closing_balance,
                    credit: rust_decimal::Decimal::ZERO,
                    summary: Some(format!("对账确认-{}", result.reconciliation_no)),
                    assist_customer_id: Some(result.customer_id),
                    assist_supplier_id: None,
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
                },
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: Some(2),
                    subject_code: Some("1131".to_string()),
                    subject_name: Some("应收账款".to_string()),
                    debit: rust_decimal::Decimal::ZERO,
                    credit: result.closing_balance,
                    summary: Some(format!("对账确认-{}", result.reconciliation_no)),
                    assist_customer_id: Some(result.customer_id),
                    assist_supplier_id: None,
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
                },
            ],
        };
        let voucher_service = crate::services::voucher_service::VoucherService::new(self.db.clone());
        if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
            tracing::warn!(
                "AR 对账单 {} 关闭成功，但生成对账确认凭证失败：{}",
                result.reconciliation_no,
                e
            );
        }

        Ok(result)
    }

    /// 更新对账单状态（通用）
    ///
    /// 批次 109 P3：新增 remark 参数，若提供则同步写入 notes 字段
    /// （原 UpdateConfirmationStatusRequest.remark 标注 dead_code 未使用）
    pub async fn update_status(
        &self,
        id: i32,
        status: &str,
        user_id: i32,
        remark: Option<String>,
    ) -> Result<ReconciliationModel, AppError> {
        // P1 3-2 修复（批次 61）：状态机 lock_exclusive 补全 + 状态白名单
        // 原实现无 txn 无 lock，且无状态白名单，任意字符串都能写入 reconciliation_status，
        // 可能导致状态机被非法值破坏。改为 txn + lock_exclusive + 白名单校验。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        // 状态白名单：仅允许合法的状态值
        let allowed_statuses = [
            ar_status::RECONCILIATION_DRAFT,
            ar_status::RECONCILIATION_SENT,
            ar_status::RECONCILIATION_CONFIRMED,
            ar_status::RECONCILIATION_DISPUTED,
            ar_status::RECONCILIATION_CLOSED,
            ar_status::RECONCILIATION_CANCELLED,
        ];
        if !allowed_statuses.contains(&status) {
            return Err(AppError::business(format!(
                "非法的对账单状态：{}，允许的状态：{:?}",
                status, allowed_statuses
            )));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(status.to_string()));
        active_model.updated_at = Set(Utc::now());

        // 批次 109 P3：remark 写入 notes 字段（原 UpdateConfirmationStatusRequest.remark 未使用）
        if let Some(remark) = remark {
            active_model.notes = Set(Some(remark));
        }

        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        let updated =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                active_model,
                Some(user_id),
            )
            .await?;

        txn.commit().await?;

        Ok(updated)
    }
}
