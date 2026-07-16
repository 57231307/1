// 批次 101 v6 复审 P2 修复：approve_invoice 补全状态门，仅 "pending" 状态可审批，
// 防止已审批/已核销等状态被重复审批（P2-7）。

use crate::models::finance_invoice::Model as InvoiceModel;
use crate::models::finance_invoice::{self, ActiveModel, Entity as FinanceInvoice};
use crate::models::status::finance_invoice as invoice_status;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

pub struct FinanceInvoiceService {
    db: Arc<DatabaseConnection>,
}

impl FinanceInvoiceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list_invoices(
        &self,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Vec<InvoiceModel>, AppError> {
        let mut query = FinanceInvoice::find();
        // V15 P0-S01：行级数据权限过滤
        // finance_invoice 表无 department_id，Dept 退化为 Self；
        // finance_invoice.created_by 是 Option<i32>（create_invoice 当前未显式设置，恒为 None，
        // Self 范围将拒绝访问；admin 范围 All 不受影响）。
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                finance_invoice::Column::CreatedBy,
                finance_invoice::Column::CreatedBy, // 无 department_id，Dept 退化为 Self，复用 created_by
            );
        }
        query
            .order_by(finance_invoice::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    pub async fn get_invoice(
        &self,
        id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Option<InvoiceModel>, AppError> {
        let invoice = FinanceInvoice::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::from)?;
        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // finance_invoice 表无 department_id，Dept 退化为 Self；
        // finance_invoice.created_by 当前恒为 None（create_invoice 未显式设置）。
        if let (Some(ctx), Some(inv)) = (data_scope, &invoice) {
            if !check_resource_owner(ctx, inv.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问发票 {}（数据范围限制）", id
                )));
            }
        }
        Ok(invoice)
    }

    pub async fn create_invoice(
        &self,
        invoice_no: String,
        amount: Decimal,
        tax_amount: Decimal,
        total_amount: Decimal,
    ) -> Result<InvoiceModel, AppError> {
        let active_model = ActiveModel {
            id: NotSet,
            invoice_no: Set(invoice_no),
            order_id: Set(None),
            invoice_date: Set(Utc::now()),
            amount: Set(amount),
            tax_amount: Set(tax_amount),
            total_amount: Set(total_amount),
            status: Set(invoice_status::PENDING.to_string()),
            paid_date: Set(None),
            payment_method: Set(None),
            notes: Set(None),
            created_by: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        active_model
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    pub async fn update_invoice(
        &self,
        id: i32,
        payload: serde_json::Value,
    ) -> Result<Option<InvoiceModel>, AppError> {
        let invoice = FinanceInvoice::find_by_id(id).one(self.db.as_ref()).await?;

        if let Some(invoice) = invoice {
            let mut active_model: ActiveModel = invoice.into();

            if let Some(status) = payload.get("status").and_then(|v| v.as_str()) {
                active_model.status = Set(status.to_string());
            }
            if let Some(notes) = payload.get("notes").and_then(|v| v.as_str()) {
                active_model.notes = Set(Some(notes.to_string()));
            }

            active_model.updated_at = Set(Utc::now());

            let updated = active_model.update(self.db.as_ref()).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_invoice(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            FinanceInvoice,
            _,
        >(self.db.as_ref(), "finance_invoice", id, Some(user_id))
        .await
    }

    pub async fn approve_invoice(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<Option<InvoiceModel>, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现状态门无 txn 无 lock，两并发 approve 均基于过期状态写入，
        // 导致状态机被绕过、审计日志缺失。
        //
        // 批次 101 v6 复审 P2-7 修复：原实现缺少状态门，已审批/已核销/已取消的发票
        // 仍可被重复审批。新增状态门：仅 "pending" 状态可审批（finance_invoice 状态值
        // 为小写，与 ar_invoice 的大写 "DRAFT" 不同，已通过 grep 确认）。
        let txn = (*self.db).begin().await?;

        let invoice = FinanceInvoice::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?;

        let result = if let Some(invoice) = invoice {
            // 状态门：仅 "pending" 状态可审批，防止已审批/已核销等状态被重复审批
            if invoice.status != invoice_status::PENDING {
                return Err(AppError::bad_request(format!(
                    "只能审批待审批状态的财务发票，当前状态：{}",
                    invoice.status
                )));
            }

            let mut active_model: ActiveModel = invoice.into();
            active_model.status = Set(invoice_status::APPROVED.to_string());
            active_model.updated_at = Set(Utc::now());

            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                active_model,
                Some(user_id),
            )
            .await?;
            Some(updated)
        } else {
            None
        };

        txn.commit().await?;

        Ok(result)
    }

    pub async fn verify_invoice(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<Option<InvoiceModel>, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现状态门无 txn 无 lock，两并发 verify 均基于过期状态写入，
        // 导致状态机被绕过、审计日志缺失。
        let txn = (*self.db).begin().await?;

        let invoice = FinanceInvoice::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?;

        let result = if let Some(invoice) = invoice {
            let mut active_model: ActiveModel = invoice.into();
            active_model.status = Set("verified".to_string());
            active_model.paid_date = Set(Some(Utc::now()));
            active_model.updated_at = Set(Utc::now());

            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                active_model,
                Some(user_id),
            )
            .await?;
            Some(updated)
        } else {
            None
        };

        txn.commit().await?;

        Ok(result)
    }
}
