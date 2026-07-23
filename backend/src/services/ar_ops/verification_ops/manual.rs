//! 应收账款-手动核销与取消核销（ar_ops/verification_ops/manual）
//!
//! D10 拆分自原 `ar_ops/verification.rs`，包含手动核销与取消核销流程：
//! - `manual_verify`（公开 API）指定一张发票 + 一张收款单 + 金额，创建核销记录
//! - `cancel_verification`（公开 API）状态门 COMPLETED → CANCELLED，恢复发票金额与状态
//!
//! 手动核销内部辅助（7 个，私有）：
//! - `validate_verify_amount`        核销金额前置校验（金额>0 + 精度≤2 位小数）
//! - `lock_and_validate_invoice`     锁定发票并校验（未取消 + 未收金额充足）
//! - `lock_and_validate_payment`     锁定收款单并校验（已确认 + 客户一致）
//! - `check_payment_available_balance` 校验收款单可用余额
//! - `create_reconciliation_record`  创建核销单主记录（VER 单号）
//! - `create_reconciliation_items`   创建核销明细（INVOICE + RECEIPT）
//! - `update_invoice_after_verify`   核销后更新发票状态 + 审计
//!
//! 业务规则：
//! - 手动核销：单张发票 + 单张收款单，金额校验 round_dp(2)
//! - 取消核销：状态门仅 CLOSED 可取消，恢复发票 received_amount/unpaid_amount/status
//! - 批次 389 P2-2：所有校验失败场景记录 warn 审计日志

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
// 批次 389 P2-2：补充 warn/error 日志宏，关键操作失败场景补审计日志
use tracing::{info, warn};

use crate::models::{ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item};
use crate::utils::error::AppError;

use super::super::json_helpers::reconciliation_to_json;
use super::super::types::ReconciliationItemContext;
use crate::services::ar_service::ArService;

impl ArService {
    /// 手动核销
    /// 指定一张发票 + 一张收款单 + 金额，创建核销记录
    pub async fn manual_verify(
        &self,
        invoice_id: i32,
        payment_id: i32,
        amount: Decimal,
        remark: Option<String>,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        Self::validate_verify_amount(invoice_id, payment_id, amount, user_id)?;
        let txn = (*self.db).begin().await?;
        let invoice = self
            .lock_and_validate_invoice(invoice_id, amount, user_id, &txn)
            .await?;
        let payment = self
            .lock_and_validate_payment(&invoice, payment_id, user_id, &txn)
            .await?;
        self.check_payment_available_balance(payment_id, &payment, amount, user_id, &txn)
            .await?;
        let now = Utc::now();
        let reconciliation = self
            .create_reconciliation_record(&invoice, amount, user_id, now, &txn)
            .await?;
        let ctx = ReconciliationItemContext {
            reconciliation: &reconciliation,
            invoice: &invoice,
            payment: &payment,
            amount,
            remark,
            now,
        };
        self.create_reconciliation_items(ctx, &txn).await?;
        let (new_status, updated_invoice) = self
            .update_invoice_after_verify(&invoice, amount, user_id, now, &txn)
            .await?;
        txn.commit().await?;
        info!(
            "AR 手动核销成功：reconciliation_id={}, invoice={}, payment={}, amount={}, 新状态={}",
            reconciliation.id, invoice_id, payment_id, amount, new_status
        );
        Ok(json!({
            "id": reconciliation.id,
            "reconciliation_no": reconciliation.reconciliation_no,
            "invoice_id": invoice_id,
            "payment_id": payment_id,
            "amount": amount.to_string(),
            "status": crate::models::status::ar::RECONCILIATION_CLOSED,
            "verified_by": user_id,
            "verified_at": now,
            "invoice_status": new_status,
            "invoice_unpaid_amount": updated_invoice.unpaid_amount.to_string(),
        }))
    }

    /// 核销金额前置校验：金额>0 + 精度≤2 位小数
    fn validate_verify_amount(
        invoice_id: i32,
        payment_id: i32,
        amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 金额校验
        if amount <= Decimal::ZERO {
            // 批次 389 P2-2：金额校验失败记录 warn 日志，便于审计异常核销行为
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INVALID_AMOUNT",
                invoice_id = invoice_id,
                payment_id = payment_id,
                amount = %amount,
                operator = user_id,
                "AR 手动核销被拒：金额必须大于零"
            );
            return Err(AppError::validation("核销金额必须大于零"));
        }
        if amount.round_dp(2) != amount {
            // 批次 389 P2-2：精度校验失败记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INVALID_PRECISION",
                invoice_id = invoice_id,
                payment_id = payment_id,
                amount = %amount,
                operator = user_id,
                "AR 手动核销被拒：精度超过 2 位小数"
            );
            return Err(AppError::validation("核销金额精度不能超过 2 位小数"));
        }
        Ok(())
    }

    /// 锁定发票并校验：未取消 + 未收金额充足
    async fn lock_and_validate_invoice(
        &self,
        invoice_id: i32,
        amount: Decimal,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ar_invoice::Model, AppError> {
        // 锁定发票
        let invoice = ar_invoice::Entity::find_by_id(invoice_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应收单 {}", invoice_id)))?;
        if invoice.status == crate::models::status::common::STATUS_CANCELLED {
            // 批次 389 P2-2：应收单已取消拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INVOICE_CANCELLED",
                invoice_id = invoice_id,
                invoice_no = %invoice.invoice_no,
                operator = user_id,
                "AR 手动核销被拒：应收单已取消"
            );
            return Err(AppError::bad_request("应收单已取消，无法核销"));
        }
        if invoice.unpaid_amount < amount {
            // 批次 389 P2-2：未收金额不足拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INSUFFICIENT_UNPAID",
                invoice_id = invoice_id,
                invoice_no = %invoice.invoice_no,
                unpaid_amount = %invoice.unpaid_amount,
                verify_amount = %amount,
                operator = user_id,
                "AR 手动核销被拒：未收金额不足"
            );
            return Err(AppError::business(format!(
                "应收单 {} 未收金额 {} 小于核销金额 {}",
                invoice.invoice_no, invoice.unpaid_amount, amount
            )));
        }
        Ok(invoice)
    }

    /// 锁定收款单并校验：已确认 + 客户与发票一致
    async fn lock_and_validate_payment(
        &self,
        invoice: &ar_invoice::Model,
        payment_id: i32,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ar_collection::Model, AppError> {
        // 锁定收款单
        let payment = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {}", payment_id)))?;
        if payment.status != crate::models::status::ar::COLLECTION_CONFIRMED {
            // 批次 389 P2-2：收款未确认拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_PAYMENT_NOT_CONFIRMED",
                payment_id = payment_id,
                payment_no = %payment.collection_no,
                status = %payment.status,
                operator = user_id,
                "AR 手动核销被拒：收款单未确认"
            );
            return Err(AppError::business(format!(
                "收款单 {} 状态为 {}，未确认不可核销",
                payment.collection_no, payment.status
            )));
        }
        if invoice.customer_id != payment.customer_id {
            // 批次 389 P2-2：客户不一致拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_CUSTOMER_MISMATCH",
                invoice_id = invoice.id,
                payment_id = payment_id,
                invoice_customer_id = invoice.customer_id,
                payment_customer_id = payment.customer_id,
                operator = user_id,
                "AR 手动核销被拒：发票客户与收款客户不一致"
            );
            return Err(AppError::business("发票客户与收款客户不一致，不可核销"));
        }
        Ok(payment)
    }

    /// 校验收款单可用余额：已核销金额 + 本次核销金额 ≤ 收款金额
    async fn check_payment_available_balance(
        &self,
        payment_id: i32,
        payment: &ar_collection::Model,
        amount: Decimal,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 查询该收款单已核销金额
        let existing_verified: Decimal = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
            .filter(ar_reconciliation_item::Column::DocumentId.eq(payment_id))
            .all(txn)
            .await?
            .into_iter()
            .map(|i| i.amount.abs())
            .sum();
        let available = payment.collection_amount - existing_verified;
        if amount > available {
            // 批次 389 P2-2：余额不足拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INSUFFICIENT_BALANCE",
                payment_id = payment_id,
                payment_no = %payment.collection_no,
                available = %available,
                verify_amount = %amount,
                operator = user_id,
                "AR 手动核销被拒：收款单可用余额不足"
            );
            return Err(AppError::business(format!(
                "收款单 {} 可用余额 {} 小于核销金额 {}",
                payment.collection_no, available, amount
            )));
        }
        Ok(())
    }

    /// 创建核销单主记录（生成 VER 单号 + 初始化金额）
    async fn create_reconciliation_record(
        &self,
        invoice: &ar_invoice::Model,
        amount: Decimal,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ar_reconciliation::Model, AppError> {
        // 创建核销单
        let verify_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no(
            txn,
            "VER",
            ar_reconciliation::Entity,
            ar_reconciliation::Column::ReconciliationNo,
        )
        .await?;
        let today = now.date_naive();
        let reconciliation = ar_reconciliation::ActiveModel {
            reconciliation_no: Set(verify_no),
            reconciliation_date: Set(today),
            period_start: Set(today),
            period_end: Set(today),
            customer_id: Set(invoice.customer_id),
            customer_name: Set(invoice.customer_name.clone()),
            opening_balance: Set(Decimal::ZERO),
            total_invoices: Set(amount),
            total_collections: Set(amount),
            closing_balance: Set(Decimal::ZERO),
            reconciliation_status: Set(Some(
                crate::models::status::ar::RECONCILIATION_CLOSED.to_string(),
            )),
            confirmed_by: Set(Some(user_id)),
            confirmed_at: Set(Some(now)),
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        Ok(reconciliation)
    }

    /// 创建核销明细：INVOICE 明细 + RECEIPT 明细
    async fn create_reconciliation_items(
        &self,
        ctx: ReconciliationItemContext<'_>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // INVOICE 明细
        ar_reconciliation_item::ActiveModel {
            reconciliation_id: Set(ctx.reconciliation.id),
            item_type: Set("INVOICE".to_string()),
            document_type: Set(Some("SALES_INVOICE".to_string())),
            document_id: Set(Some(ctx.invoice.id)),
            document_no: Set(Some(ctx.invoice.invoice_no.clone())),
            document_date: Set(Some(ctx.invoice.invoice_date)),
            amount: Set(ctx.amount),
            matched_amount: Set(Some(ctx.amount)),
            match_status: Set(crate::models::status::ar::MATCH_MATCHED.to_string()),
            matched_item_id: Set(None),
            remarks: Set(ctx.remark.clone()),
            created_at: Set(ctx.now),
            updated_at: Set(ctx.now),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        // RECEIPT 明细
        ar_reconciliation_item::ActiveModel {
            reconciliation_id: Set(ctx.reconciliation.id),
            item_type: Set("RECEIPT".to_string()),
            document_type: Set(Some("AR_COLLECTION".to_string())),
            document_id: Set(Some(ctx.payment.id)),
            document_no: Set(Some(ctx.payment.collection_no.clone())),
            document_date: Set(Some(ctx.payment.collection_date)),
            amount: Set(-ctx.amount),
            matched_amount: Set(Some(ctx.amount)),
            match_status: Set(crate::models::status::ar::MATCH_MATCHED.to_string()),
            matched_item_id: Set(None),
            remarks: Set(ctx.remark),
            created_at: Set(ctx.now),
            updated_at: Set(ctx.now),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        Ok(())
    }

    /// 核销后更新发票状态（received_amount/unpaid_amount/status + 审计）
    async fn update_invoice_after_verify(
        &self,
        invoice: &ar_invoice::Model,
        amount: Decimal,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(String, ar_invoice::Model), AppError> {
        // 更新发票
        let mut inv_active: ar_invoice::ActiveModel = invoice.clone().into();
        let new_received = invoice.received_amount + amount;
        let new_unpaid = (invoice.invoice_amount - new_received).max(Decimal::ZERO);
        let new_status = if new_unpaid == Decimal::ZERO {
            crate::models::status::payment::PAYMENT_PAID.to_string()
        } else {
            crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string()
        };
        inv_active.received_amount = Set(new_received);
        inv_active.unpaid_amount = Set(new_unpaid);
        inv_active.status = Set(new_status.clone());
        inv_active.updated_at = Set(now);
        let updated_invoice =
            crate::services::audit_log_service::AuditLogService::update_with_audit::<
                ar_invoice::Entity,
                _,
                _,
            >(txn, "ar_invoice", inv_active, Some(user_id))
            .await?;
        Ok((new_status, updated_invoice))
    }

    /// 取消核销
    /// 状态门：COMPLETED → CANCELLED，恢复发票 received_amount/unpaid_amount/status
    pub async fn cancel_verification(
        &self,
        verification_id: i32,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;

        let reconciliation = ar_reconciliation::Entity::find_by_id(verification_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("核销单 {} 不存在", verification_id)))?;

        if reconciliation.reconciliation_status.as_deref() != Some(crate::models::status::ar::RECONCILIATION_CLOSED) {
            // 批次 389 P2-2：状态门拒绝记录 warn 日志，便于审计非法状态变更
            warn!(
                target: "business_audit",
                event = "AR_VERIFICATION_CANCEL_REJECTED",
                verification_id = verification_id,
                status = ?reconciliation.reconciliation_status,
                operator = user_id,
                "AR 核销取消被拒：状态非 closed"
            );
            return Err(AppError::bad_request(format!(
                "核销单状态为 {:?}，仅 closed 状态可取消",
                reconciliation.reconciliation_status
            )));
        }

        // 查询所有 INVOICE 明细，按 invoice_id 汇总应回滚金额
        let items = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ReconciliationId.eq(verification_id))
            .filter(ar_reconciliation_item::Column::ItemType.eq("INVOICE"))
            .all(&txn)
            .await?;

        // 批量查询并锁定所有相关发票
        let inv_ids: Vec<i32> = items
            .iter()
            .filter_map(|i| i.document_id)
            .collect();
        let mut inv_map: std::collections::HashMap<i32, ar_invoice::Model> = if inv_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            ar_invoice::Entity::find()
                .filter(ar_invoice::Column::Id.is_in(inv_ids))
                .lock_exclusive()
                .all(&txn)
                .await?
                .into_iter()
                .map(|inv| (inv.id, inv))
                .collect()
        };

        let now = Utc::now();

        for item in &items {
            let inv_id = item.document_id.ok_or_else(|| {
                AppError::business("核销明细缺少 document_id".to_string())
            })?;
            let invoice = inv_map.get_mut(&inv_id).ok_or_else(|| {
                AppError::not_found(format!("应收单 {}", inv_id))
            })?;
            invoice.received_amount -= item.amount;
            invoice.unpaid_amount =
                (invoice.invoice_amount - invoice.received_amount).max(Decimal::ZERO);
            // 状态恢复
            if invoice.received_amount >= invoice.invoice_amount {
                invoice.status = crate::models::status::payment::PAYMENT_PAID.to_string();
            } else if invoice.received_amount > Decimal::ZERO {
                invoice.status = crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string();
            } else {
                invoice.status = crate::models::status::common::STATUS_APPROVED.to_string();
            }
            let inv_active: ar_invoice::ActiveModel = invoice.clone().into();
            crate::services::audit_log_service::AuditLogService::update_with_audit::<
                ar_invoice::Entity,
                _,
                _,
            >(&txn, "ar_invoice", inv_active, Some(user_id))
            .await?;
        }

        // 更新核销单状态
        let mut rec_active: ar_reconciliation::ActiveModel = reconciliation.into();
        rec_active.reconciliation_status = Set(Some(crate::models::status::ar::RECONCILIATION_CANCELLED.to_string()));
        rec_active.confirmed_by = Set(None);
        rec_active.confirmed_at = Set(None);
        rec_active.updated_at = Set(now);
        let updated =
            crate::services::audit_log_service::AuditLogService::update_with_audit::<
                ar_reconciliation::Entity,
                _,
                _,
            >(&txn, "ar_reconciliation", rec_active, Some(user_id))
            .await?;

        txn.commit().await?;

        info!("AR 核销取消成功：verification_id={}", verification_id);

        Ok(reconciliation_to_json(updated))
    }
}
