//! 应付核销 Service
//!
//! 应付核销服务层，负责应付核销的核心业务逻辑
//! 包含自动核销、手工核销、取消核销等管理

use crate::models::{ap_invoice, ap_payment, ap_verification, ap_verification_item};
use crate::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// 应付核销服务
pub struct ApVerificationService {
    db: Arc<DatabaseConnection>,
}

impl ApVerificationService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成核销单号
    // 格式：VER + 年月日 + 三位序号（VER20260315001）
    crate::impl_generate_no!(
        generate_verification_no,
        "VER",
        ap_verification::Entity,
        ap_verification::Column::VerificationNo
    );

    /// 自动核销（按到期日优先匹配）
    pub async fn auto_verify(
        &self,
        supplier_id: i32,
        user_id: i32,
    ) -> Result<ap_verification::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询该供应商未核销的应付单（按到期日排序）
        let invoices = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(supplier_id))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .order_by(ap_invoice::Column::DueDate, Order::Asc)
            .all(&txn)
            .await?;

        // 2. 查询该供应商已确认未核销的付款单
        let payments = ap_payment::Entity::find()
            .filter(ap_payment::Column::SupplierId.eq(supplier_id))
            .filter(ap_payment::Column::PaymentStatus.eq("CONFIRMED"))
            .all(&txn)
            .await?;

        // 3. 查询已核销记录，排除已核销的付款单
        let payment_ids: Vec<i32> = payments.iter().map(|p| p.id).collect();
        let existing_verification_payments = ap_verification_item::Entity::find()
            .filter(ap_verification_item::Column::PaymentId.is_in(payment_ids))
            .all(&txn)
            .await?;

        let verified_payment_ids: std::collections::HashSet<i32> = existing_verification_payments
            .iter()
            .map(|item| item.payment_id)
            .collect();

        let available_payments: Vec<ap_payment::Model> = payments
            .into_iter()
            .filter(|p| !verified_payment_ids.contains(&p.id))
            .collect();

        // 4. 逐个匹配核销
        let mut verification_items = Vec::new();
        let mut total_amount = Decimal::ZERO;
        let mut invoice_remaining: std::collections::HashMap<i32, Decimal> = invoices
            .iter()
            .map(|inv| (inv.id, inv.unpaid_amount))
            .collect();

        for payment in available_payments.iter() {
            let mut remaining = payment.payment_amount;

            // 查询该付款单已核销金额
            let verified_amount: Decimal = ap_verification_item::Entity::find()
                .filter(ap_verification_item::Column::PaymentId.eq(payment.id))
                .all(&txn)
                .await?
                .iter()
                .map(|item| item.verify_amount)
                .sum();

            remaining -= verified_amount;

            if remaining <= Decimal::ZERO {
                continue;
            }

            for invoice in invoices.iter() {
                if remaining <= Decimal::ZERO {
                    break;
                }

                let unpaid = invoice_remaining
                    .get(&invoice.id)
                    .copied()
                    .unwrap_or(Decimal::ZERO);
                if unpaid > Decimal::ZERO {
                    let verify_amount = remaining.min(unpaid);

                    verification_items.push(ApVerificationItemDto {
                        invoice_id: invoice.id,
                        payment_id: payment.id,
                        verify_amount,
                        notes: None,
                    });

                    remaining -= verify_amount;
                    total_amount += verify_amount;
                    invoice_remaining.insert(invoice.id, unpaid - verify_amount);
                }
            }
        }

        if verification_items.is_empty() {
            return Err(AppError::business("没有可核销的应付单和付款单".to_string()));
        }

        // 5. 创建核销单
        let verification_no = self.generate_verification_no().await?;
        let verification_date = Utc::now().naive_utc().date();

        let verification = ap_verification::ActiveModel {
            verification_no: Set(verification_no),
            verification_date: Set(verification_date),
            supplier_id: Set(supplier_id),
            verification_type: Set("AUTO".to_string()),
            total_amount: Set(total_amount),
            verification_status: Set("COMPLETED".to_string()),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 6. 创建核销明细并更新应付单和付款单
        for item_dto in verification_items {
            let _item = ap_verification_item::ActiveModel {
                verification_id: Set(verification.id),
                invoice_id: Set(item_dto.invoice_id),
                payment_id: Set(item_dto.payment_id),
                verify_amount: Set(item_dto.verify_amount),
                notes: Set(item_dto.notes),
                ..Default::default()
            }
            .insert(&txn)
            .await?;

            // 更新应付单
            // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发核销导致 paid_amount 丢失更新
            let mut invoice = ap_invoice::Entity::find_by_id(item_dto.invoice_id)
                .lock_exclusive()
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("应付单 {}", item_dto.invoice_id)))?;

            invoice.paid_amount += item_dto.verify_amount;
            invoice.unpaid_amount = invoice.amount - invoice.paid_amount;

            if invoice.unpaid_amount <= Decimal::ZERO {
                invoice.invoice_status = "PAID".to_string();
            } else {
                invoice.invoice_status = "PARTIAL_PAID".to_string();
            }

            let invoice_active: ap_invoice::ActiveModel = invoice.into();
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                invoice_active,
                Some(0),
            )
            .await?;
        }

        txn.commit().await?;

        Ok(verification)
    }

    /// 手工核销（指定核销关系）
    pub async fn manual_verify(
        &self,
        req: ManualVerifyRequest,
        user_id: i32,
    ) -> Result<ap_verification::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 验证所有应付单和付款单
        let mut total_amount = Decimal::ZERO;
        // v10 P1-4 修复：保存第一次循环已查询（带行锁）的 invoice，供第二次循环复用，
        // 避免循环内重复 find_by_id（N+1 查询）。行锁在事务内持续到 commit，复用安全。
        let mut invoice_map: std::collections::HashMap<i32, ap_invoice::Model> =
            std::collections::HashMap::new();

        for item in &req.items {
            // 验证应付单
            // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发核销导致 paid_amount 丢失更新
            let invoice = ap_invoice::Entity::find_by_id(item.invoice_id)
                .lock_exclusive()
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("应付单 {}", item.invoice_id)))?;

            if invoice.unpaid_amount < item.verify_amount {
                return Err(AppError::business(format!(
                    "应付单{}未付金额{}小于核销金额{}",
                    invoice.invoice_no, invoice.unpaid_amount, item.verify_amount
                )));
            }

            // 验证付款单
            // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发核销复用同一付款单导致超额核销
            let payment = ap_payment::Entity::find_by_id(item.payment_id)
                .lock_exclusive()
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("付款单 ID: {}", item.payment_id)))?;

            if payment.payment_status != "CONFIRMED" {
                return Err(AppError::business(format!(
                    "付款单{}状态为{}，未确认不可核销",
                    payment.payment_no, payment.payment_status
                )));
            }

            total_amount += item.verify_amount;

            // v10 P1-4 修复：保存到 map 供第二次循环复用（若 invoice_id 重复，后一次覆盖前一次，验证均已通过）
            invoice_map.insert(item.invoice_id, invoice);
        }

        // 2. 创建核销单
        let verification_no = self.generate_verification_no().await?;
        let verification_date = Utc::now().naive_utc().date();

        let verification = ap_verification::ActiveModel {
            verification_no: Set(verification_no),
            verification_date: Set(verification_date),
            supplier_id: Set(req.supplier_id),
            verification_type: Set("MANUAL".to_string()),
            total_amount: Set(total_amount),
            verification_status: Set("COMPLETED".to_string()),
            notes: Set(req.notes),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 3. 创建核销明细并更新应付单和付款单
        for item in req.items {
            let _item = ap_verification_item::ActiveModel {
                verification_id: Set(verification.id),
                invoice_id: Set(item.invoice_id),
                payment_id: Set(item.payment_id),
                verify_amount: Set(item.verify_amount),
                notes: Set(item.notes),
                ..Default::default()
            }
            .insert(&txn)
            .await?;

            // 更新应付单
            // v10 P1-4 修复：优先复用第一次循环已查询（带行锁）的 invoice，避免重复 find_by_id
            // 若 invoice_id 重复（map 已 remove），回退到 find_by_id 查最新值（与原逻辑一致，保留累加正确性）
            let mut invoice = match invoice_map.remove(&item.invoice_id) {
                Some(inv) => inv,
                None => ap_invoice::Entity::find_by_id(item.invoice_id)
                    .one(&txn)
                    .await?
                    .ok_or_else(|| AppError::not_found(format!("应付单 {}", item.invoice_id)))?,
            };

            invoice.paid_amount += item.verify_amount;
            invoice.unpaid_amount = invoice.amount - invoice.paid_amount;

            if invoice.unpaid_amount <= Decimal::ZERO {
                invoice.invoice_status = "PAID".to_string();
            } else {
                invoice.invoice_status = "PARTIAL_PAID".to_string();
            }

            let invoice_active: ap_invoice::ActiveModel = invoice.into();
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                invoice_active,
                Some(0),
            )
            .await?;
        }

        txn.commit().await?;

        Ok(verification)
    }

    /// 取消核销
    pub async fn cancel(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<ap_verification::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询核销单（批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更）
        let verification = ap_verification::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("核销单 ID: {}", id)))?;

        // 2. 检查状态
        if verification.verification_status == "CANCELLED" {
            return Err(AppError::business("核销单已取消"));
        }

        // 3. 查询核销明细
        let items = ap_verification_item::Entity::find()
            .filter(ap_verification_item::Column::VerificationId.eq(id))
            .all(&txn)
            .await?;

        // 4. 恢复应付单状态
        for item in items {
            // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发取消核销导致 paid_amount 丢失更新
            let mut invoice = ap_invoice::Entity::find_by_id(item.invoice_id)
                .lock_exclusive()
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("应付单 {}", item.invoice_id)))?;

            invoice.paid_amount -= item.verify_amount;
            invoice.unpaid_amount = invoice.amount - invoice.paid_amount;

            // 恢复应付状态
            if invoice.paid_amount <= Decimal::ZERO {
                invoice.invoice_status = "AUDITED".to_string();
            } else {
                invoice.invoice_status = "PARTIAL_PAID".to_string();
            }

            let invoice_active: ap_invoice::ActiveModel = invoice.into();
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                invoice_active,
                Some(0),
            )
            .await?;
        }

        // 5. 取消核销单
        let now = Utc::now();
        let mut verification_active: ap_verification::ActiveModel = verification.into();
        verification_active.verification_status = Set("CANCELLED".to_string());
        verification_active.cancelled_by = Set(Some(user_id));
        verification_active.cancelled_at = Set(Some(now));
        verification_active.cancelled_reason = Set(Some(reason));

        let verification = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            verification_active,
            Some(0),
        )
        .await?;

        txn.commit().await?;

        Ok(verification)
    }

    /// 获取核销单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_verification::Model, AppError> {
        let verification = ap_verification::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("核销单 {}", id)))?;

        Ok(verification)
    }

    /// 获取核销单列表
    pub async fn get_list(
        &self,
        supplier_id: Option<i32>,
        verification_type: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ap_verification::Model>, u64), AppError> {
        let mut query = ap_verification::Entity::find();

        // 筛选条件
        if let Some(sid) = supplier_id {
            query = query.filter(ap_verification::Column::SupplierId.eq(sid));
        }
        if let Some(vtype) = verification_type {
            query = query.filter(ap_verification::Column::VerificationType.eq(vtype));
        }
        if let Some(sd) = start_date {
            query = query.filter(ap_verification::Column::VerificationDate.gte(sd));
        }
        if let Some(ed) = end_date {
            query = query.filter(ap_verification::Column::VerificationDate.lte(ed));
        }

        // 分页
        let paginator = query
            .order_by(ap_verification::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page).await?;

        Ok((items, total))
    }

    /// 获取未核销应付单列表
    pub async fn get_unverified_invoices(
        &self,
        supplier_id: i32,
    ) -> Result<Vec<ap_invoice::Model>, AppError> {
        let invoices = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(supplier_id))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .order_by(ap_invoice::Column::DueDate, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(invoices)
    }

    /// 获取未核销付款单列表
    pub async fn get_unverified_payments(
        &self,
        supplier_id: i32,
    ) -> Result<Vec<ap_payment::Model>, AppError> {
        let payments = ap_payment::Entity::find()
            .filter(ap_payment::Column::SupplierId.eq(supplier_id))
            .filter(ap_payment::Column::PaymentStatus.eq("CONFIRMED"))
            .order_by(ap_payment::Column::PaymentDate, Order::Asc)
            .all(&*self.db)
            .await?;

        // v11 批次 37 修复：批量查询所有付款单的核销明细，按 payment_id 分组求和，避免循环内逐个查询（N+1）
        let payment_ids: Vec<i32> = payments.iter().map(|p| p.id).collect();
        let verified_items = if payment_ids.is_empty() {
            Vec::new()
        } else {
            ap_verification_item::Entity::find()
                .filter(ap_verification_item::Column::PaymentId.is_in(payment_ids))
                .all(&*self.db)
                .await?
        };
        let mut verified_map: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        for item in &verified_items {
            *verified_map.entry(item.payment_id).or_insert(Decimal::ZERO) += item.verify_amount;
        }

        // 过滤掉已核销的付款单
        let result = payments
            .into_iter()
            .filter(|p| {
                verified_map.get(&p.id).copied().unwrap_or(Decimal::ZERO) < p.payment_amount
            })
            .collect();

        Ok(result)
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 核销明细 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApVerificationItemDto {
    /// 应付单 ID
    pub invoice_id: i32,

    /// 付款单 ID
    pub payment_id: i32,

    /// 核销金额
    pub verify_amount: Decimal,

    /// 备注
    pub notes: Option<String>,
}

/// 手工核销请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct ManualVerifyRequest {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 核销明细
    pub items: Vec<ApVerificationItemDto>,

    /// 备注
    pub notes: Option<String>,
}
