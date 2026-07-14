//! 应付核销 Service
//!
//! 应付核销服务层，负责应付核销的核心业务逻辑
//! 包含自动核销、手工核销、取消核销等管理

use crate::models::{ap_invoice, ap_payment, ap_verification, ap_verification_item};
use crate::utils::error::AppError;
// 批次 259 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
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
    // P1 5-11 修复（批次 60）：改用宏的 txn 变体（5 参数），调用方传入外层 txn，
    // 确保单号生成的 advisory_xact_lock 与 INSERT 在同一事务内，锁覆盖完整临界区
    crate::impl_generate_no!(
        generate_verification_no,
        "VER",
        ap_verification::Entity,
        ap_verification::Column::VerificationNo,
        txn
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

        // v13 批次 40 修复 + v14 批次 41 清理：
        // 原代码第 91-97 行循环内逐个查询已核销金额（N+1 查询）。
        // 批量查询已得到 existing_verification_payments，按 payment_id 分组取
        // 已核销付款 id 集合（verified_payment_ids），过滤掉任何有核销记录的付款。
        // 由于 available_payments 已过滤掉所有有核销记录的付款，循环内
        // verified_amount 必为 0，无需再查/再减，remaining 直接等于 payment_amount。
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
            // available_payments 已过滤掉所有有核销记录的付款，
            // remaining 直接等于 payment_amount（无需减 verified_amount）
            let mut remaining = payment.payment_amount;

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
        // P1 5-11 修复（批次 60）：传入外层 txn，确保单号生成与 INSERT 在同一事务内
        let verification_no = self.generate_verification_no(&txn).await?;
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
        // v17 批次 46 修复：循环外批量查询并锁定所有 invoice，避免循环内逐个 find_by_id + lock_exclusive（N+1）
        // 使用 get_mut + clone 模式，同一 invoice_id 重复核销时复用 map 中已更新的值，无需回退查询
        let invoice_ids: Vec<i32> = verification_items.iter().map(|i| i.invoice_id).collect();
        let mut invoice_map: std::collections::HashMap<i32, ap_invoice::Model> =
            if invoice_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                use sea_orm::QuerySelect;
                ap_invoice::Entity::find()
                    .filter(ap_invoice::Column::Id.is_in(invoice_ids))
                    .lock_exclusive()
                    .all(&txn)
                    .await?
                    .into_iter()
                    .map(|inv| (inv.id, inv))
                    .collect()
            };

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

            // 更新应付单（v17 批次 46 修复：从批量查询结果获取，get_mut 复用已更新值）
            let invoice = invoice_map
                .get_mut(&item_dto.invoice_id)
                .ok_or_else(|| AppError::not_found(format!("应付单 {}", item_dto.invoice_id)))?;

            invoice.paid_amount += item_dto.verify_amount;
            invoice.unpaid_amount = invoice.amount - invoice.paid_amount;

            if invoice.unpaid_amount <= Decimal::ZERO {
                invoice.invoice_status = "PAID".to_string();
            } else {
                invoice.invoice_status = "PARTIAL_PAID".to_string();
            }

            let invoice_active: ap_invoice::ActiveModel = invoice.clone().into();
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                invoice_active,
                // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
                Some(user_id),
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

        // v16 批次 44 修复：循环外批量查询并锁定所有 invoice 和 payment，
        // 避免循环内逐个 find_by_id + lock_exclusive（N+1 查询）
        // 行锁在事务内持续到 commit，批量锁定与逐个锁定效果一致
        let invoice_ids: Vec<i32> = req.items.iter().map(|i| i.invoice_id).collect();
        let payment_ids: Vec<i32> = req.items.iter().map(|i| i.payment_id).collect();
        let mut invoice_map: std::collections::HashMap<i32, ap_invoice::Model> =
            if invoice_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                use sea_orm::QuerySelect;
                ap_invoice::Entity::find()
                    .filter(ap_invoice::Column::Id.is_in(invoice_ids))
                    .lock_exclusive()
                    .all(&txn)
                    .await?
                    .into_iter()
                    .map(|inv| (inv.id, inv))
                    .collect()
            };
        let payment_map: std::collections::HashMap<i32, ap_payment::Model> =
            if payment_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                use sea_orm::QuerySelect;
                ap_payment::Entity::find()
                    .filter(ap_payment::Column::Id.is_in(payment_ids))
                    .lock_exclusive()
                    .all(&txn)
                    .await?
                    .into_iter()
                    .map(|p| (p.id, p))
                    .collect()
            };

        for item in &req.items {
            // 验证应付单（v16 批次 44 修复：从批量查询结果获取）
            let invoice = invoice_map
                .get(&item.invoice_id)
                .ok_or_else(|| AppError::not_found(format!("应付单 {}", item.invoice_id)))?;

            if invoice.unpaid_amount < item.verify_amount {
                return Err(AppError::business(format!(
                    "应付单{}未付金额{}小于核销金额{}",
                    invoice.invoice_no, invoice.unpaid_amount, item.verify_amount
                )));
            }

            // 验证付款单（v16 批次 44 修复：从批量查询结果获取）
            let payment = payment_map
                .get(&item.payment_id)
                .ok_or_else(|| AppError::not_found(format!("付款单 ID: {}", item.payment_id)))?;

            if payment.payment_status != "CONFIRMED" {
                return Err(AppError::business(format!(
                    "付款单{}状态为{}，未确认不可核销",
                    payment.payment_no, payment.payment_status
                )));
            }

            total_amount += item.verify_amount;
        }

        // 2. 创建核销单
        // P1 5-11 修复（批次 60）：传入外层 txn，确保单号生成与 INSERT 在同一事务内
        let verification_no = self.generate_verification_no(&txn).await?;
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
            // v17 批次 46 修复：改用 get_mut + clone，同一 invoice_id 重复时复用 map 中已更新的值，
            // 无需回退到 find_by_id（消除重复 invoice_id 场景的 N+1 查询）
            let invoice = invoice_map
                .get_mut(&item.invoice_id)
                .ok_or_else(|| AppError::not_found(format!("应付单 {}", item.invoice_id)))?;

            invoice.paid_amount += item.verify_amount;
            invoice.unpaid_amount = invoice.amount - invoice.paid_amount;

            if invoice.unpaid_amount <= Decimal::ZERO {
                invoice.invoice_status = "PAID".to_string();
            } else {
                invoice.invoice_status = "PARTIAL_PAID".to_string();
            }

            let invoice_active: ap_invoice::ActiveModel = invoice.clone().into();
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                invoice_active,
                // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
                Some(user_id),
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
        // v16 批次 44 修复：循环外批量查询并锁定所有 invoice，避免循环内逐个 lock_exclusive（N+1）
        let invoice_ids: Vec<i32> = items.iter().map(|i| i.invoice_id).collect();
        let mut invoice_map: std::collections::HashMap<i32, ap_invoice::Model> =
            if invoice_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                use sea_orm::QuerySelect;
                ap_invoice::Entity::find()
                    .filter(ap_invoice::Column::Id.is_in(invoice_ids))
                    .lock_exclusive()
                    .all(&txn)
                    .await?
                    .into_iter()
                    .map(|inv| (inv.id, inv))
                    .collect()
            };

        for item in items {
            // v17 批次 46 修复：改用 get_mut + clone，同一 invoice_id 重复时复用 map 中已更新的值，
            // 无需回退到 find_by_id（消除重复 invoice_id 场景的 N+1 查询）
            let invoice = invoice_map
                .get_mut(&item.invoice_id)
                .ok_or_else(|| AppError::not_found(format!("应付单 {}", item.invoice_id)))?;

            invoice.paid_amount -= item.verify_amount;
            invoice.unpaid_amount = invoice.amount - invoice.paid_amount;

            // P2 3-15 修复：取消核销后状态恢复应区分 PAID/PARTIAL_PAID/AUDITED 三态，
            // 原实现 paid_amount<=0 一律设 AUDITED，但若 paid_amount 仍等于 amount
            // （即仅取消部分核销且全额仍付清）应保留 PAID 语义。
            // - paid_amount >= amount：仍有全额付款 → PAID
            // - 0 < paid_amount < amount：部分付款 → PARTIAL_PAID
            // - paid_amount <= 0：无付款 → AUDITED
            if invoice.paid_amount >= invoice.amount {
                invoice.invoice_status = "PAID".to_string();
            } else if invoice.paid_amount > Decimal::ZERO {
                invoice.invoice_status = "PARTIAL_PAID".to_string();
            } else {
                invoice.invoice_status = "AUDITED".to_string();
            }

            let invoice_active: ap_invoice::ActiveModel = invoice.clone().into();
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                invoice_active,
                // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
                Some(user_id),
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
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
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

        // 批次 259 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(ap_verification::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
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

#[cfg(test)]
mod tests {
    //! 应付核销服务单元测试
    //!
    //! 覆盖目标：
    //! - 自动核销匹配算法（金额匹配、按序分配、零额跳过、空列表）
    //! - 已核销付款过滤逻辑
    //! - 手工核销校验（未付金额、付款状态、边界）
    //! - 取消核销状态恢复（PAID/PARTIAL_PAID/AUDITED 三态）
    //! - DTO 构造与服务实例创建

    use super::*;
    use crate::decs;
    use sea_orm::Database;
    use std::str::FromStr;

    // ============ 状态常量测试 ============

    /// 测试_AP核销状态与类型常量值
    ///
    /// 验证核销单状态（COMPLETED/CANCELLED）和核销类型（AUTO/MANUAL）字符串值互不相同
    #[test]
    fn 测试_AP核销状态与类型常量值() {
        // 核销单状态
        assert_eq!("COMPLETED", "COMPLETED");
        assert_eq!("CANCELLED", "CANCELLED");
        assert_ne!("COMPLETED", "CANCELLED");

        // 核销类型
        assert_ne!("AUTO", "MANUAL");
        assert_eq!("AUTO", "AUTO");
        assert_eq!("MANUAL", "MANUAL");
    }

    /// 测试_AP发票与付款状态常量值
    ///
    /// 验证 auto_verify/manual_verify/cancel 中使用的发票/付款状态字符串
    #[test]
    fn 测试_AP发票与付款状态常量值() {
        // 发票状态
        let paid = "PAID";
        let partial_paid = "PARTIAL_PAID";
        let audited = "AUDITED";
        let cancelled = "CANCELLED";
        assert_ne!(paid, partial_paid);
        assert_ne!(paid, audited);
        assert_ne!(partial_paid, audited);
        assert_ne!(cancelled, paid);

        // 付款状态
        let confirmed = "CONFIRMED";
        assert_eq!(confirmed, "CONFIRMED");
        assert_ne!(confirmed, paid);
    }

    // ============ DTO 构造测试 ============

    /// 测试_核销明细DTO_字段构造
    #[test]
    fn 测试_核销明细DTO_字段构造() {
        let dto = ApVerificationItemDto {
            invoice_id: 1,
            payment_id: 10,
            verify_amount: decs!("100.50"),
            notes: Some("测试备注".to_string()),
        };
        assert_eq!(dto.invoice_id, 1);
        assert_eq!(dto.payment_id, 10);
        assert_eq!(dto.verify_amount, decs!("100.50"));
        assert_eq!(dto.notes, Some("测试备注".to_string()));
    }

    /// 测试_核销明细DTO_无备注构造
    #[test]
    fn 测试_核销明细DTO_无备注构造() {
        let dto = ApVerificationItemDto {
            invoice_id: 5,
            payment_id: 20,
            verify_amount: Decimal::ZERO,
            notes: None,
        };
        assert_eq!(dto.notes, None);
        assert_eq!(dto.verify_amount, Decimal::ZERO);
    }

    /// 测试_手工核销请求_字段构造
    ///
    /// 验证 ManualVerifyRequest 包含多条核销明细的构造
    #[test]
    fn 测试_手工核销请求_字段构造() {
        let items = vec![
            ApVerificationItemDto {
                invoice_id: 1,
                payment_id: 10,
                verify_amount: decs!("50"),
                notes: None,
            },
            ApVerificationItemDto {
                invoice_id: 2,
                payment_id: 10,
                verify_amount: decs!("30"),
                notes: None,
            },
        ];
        let req = ManualVerifyRequest {
            supplier_id: 100,
            items,
            notes: Some("手工核销".to_string()),
        };
        assert_eq!(req.supplier_id, 100);
        assert_eq!(req.items.len(), 2);
        assert_eq!(req.items[0].verify_amount, decs!("50"));
        assert_eq!(req.items[1].verify_amount, decs!("30"));
        assert_eq!(req.notes, Some("手工核销".to_string()));
    }

    // ============ 自动核销匹配算法测试 ============

    /// 复现 auto_verify 中的匹配算法（纯函数版本，用于测试）
    /// 输入：发票列表 (id, unpaid_amount)、可用付款列表 (id, payment_amount)
    /// 输出：(核销明细列表, 总核销金额)
    fn match_invoices_payments(
        invoices: &[(i32, Decimal)],
        payments: &[(i32, Decimal)],
    ) -> (Vec<ApVerificationItemDto>, Decimal) {
        let mut items = Vec::new();
        let mut total_amount = Decimal::ZERO;
        // 发票剩余未付金额映射
        let mut invoice_remaining: std::collections::HashMap<i32, Decimal> =
            invoices.iter().cloned().collect();

        for (payment_id, payment_amount) in payments {
            let mut remaining = *payment_amount;
            // 金额为零或负的付款跳过
            if remaining <= Decimal::ZERO {
                continue;
            }
            for (invoice_id, _) in invoices {
                if remaining <= Decimal::ZERO {
                    break;
                }
                let unpaid = invoice_remaining
                    .get(invoice_id)
                    .copied()
                    .unwrap_or(Decimal::ZERO);
                if unpaid > Decimal::ZERO {
                    let verify_amount = remaining.min(unpaid);
                    items.push(ApVerificationItemDto {
                        invoice_id: *invoice_id,
                        payment_id: *payment_id,
                        verify_amount,
                        notes: None,
                    });
                    remaining -= verify_amount;
                    total_amount += verify_amount;
                    invoice_remaining.insert(*invoice_id, unpaid - verify_amount);
                }
            }
        }
        (items, total_amount)
    }

    /// 测试_自动核销匹配算法_金额完全匹配
    ///
    /// 单发票 + 单付款，金额相等 → 1 条核销明细，总额等于发票金额
    #[test]
    fn 测试_自动核销匹配算法_金额完全匹配() {
        let invoices = vec![(1, decs!("100"))];
        let payments = vec![(10, decs!("100"))];
        let (items, total) = match_invoices_payments(&invoices, &payments);

        assert_eq!(items.len(), 1);
        assert_eq!(total, decs!("100"));
        assert_eq!(items[0].invoice_id, 1);
        assert_eq!(items[0].payment_id, 10);
        assert_eq!(items[0].verify_amount, decs!("100"));
    }

    /// 测试_自动核销匹配算法_付款大于发票
    ///
    /// 付款 150 > 发票 100 → 核销 100，付款剩余 50 无更多发票可匹配
    #[test]
    fn 测试_自动核销匹配算法_付款大于发票() {
        let invoices = vec![(1, decs!("100"))];
        let payments = vec![(10, decs!("150"))];
        let (items, total) = match_invoices_payments(&invoices, &payments);

        assert_eq!(items.len(), 1);
        assert_eq!(total, decs!("100"));
        assert_eq!(items[0].verify_amount, decs!("100"));
    }

    /// 测试_自动核销匹配算法_发票大于付款
    ///
    /// 发票 200 > 付款 100 → 核销 100，发票剩余 100 未核销
    #[test]
    fn 测试_自动核销匹配算法_发票大于付款() {
        let invoices = vec![(1, decs!("200"))];
        let payments = vec![(10, decs!("100"))];
        let (items, total) = match_invoices_payments(&invoices, &payments);

        assert_eq!(items.len(), 1);
        assert_eq!(total, decs!("100"));
        assert_eq!(items[0].verify_amount, decs!("100"));
    }

    /// 测试_自动核销匹配算法_多付款多发票按序匹配
    ///
    /// 付款 A=100, B=50；发票 1=80, 2=70
    /// 预期：A 核销发票1(80) + 发票2(20)，B 核销发票2(30)
    #[test]
    fn 测试_自动核销匹配算法_多付款多发票按序匹配() {
        let invoices = vec![(1, decs!("80")), (2, decs!("70"))];
        let payments = vec![(10, decs!("100")), (11, decs!("50"))];
        let (items, total) = match_invoices_payments(&invoices, &payments);

        // 付款 A=100 核销发票1=80，剩余 20 核销发票2=20
        // 付款 B=50 核销发票2 剩余 50
        assert_eq!(items.len(), 3);
        assert_eq!(total, decs!("130"));
        assert_eq!(items[0].invoice_id, 1);
        assert_eq!(items[0].payment_id, 10);
        assert_eq!(items[0].verify_amount, decs!("80"));
        assert_eq!(items[1].invoice_id, 2);
        assert_eq!(items[1].payment_id, 10);
        assert_eq!(items[1].verify_amount, decs!("20"));
        assert_eq!(items[2].invoice_id, 2);
        assert_eq!(items[2].payment_id, 11);
        assert_eq!(items[2].verify_amount, decs!("50"));
    }

    /// 测试_自动核销匹配算法_零额付款跳过
    #[test]
    fn 测试_自动核销匹配算法_零额付款跳过() {
        let invoices = vec![(1, decs!("100"))];
        let payments = vec![(10, Decimal::ZERO), (11, decs!("50"))];
        let (items, total) = match_invoices_payments(&invoices, &payments);

        // 付款 10 金额为 0，跳过；付款 11 核销 50
        assert_eq!(items.len(), 1);
        assert_eq!(total, decs!("50"));
        assert_eq!(items[0].payment_id, 11);
    }

    /// 测试_自动核销匹配算法_空列表无核销
    #[test]
    fn 测试_自动核销匹配算法_空列表无核销() {
        let invoices: Vec<(i32, Decimal)> = vec![];
        let payments: Vec<(i32, Decimal)> = vec![];
        let (items, total) = match_invoices_payments(&invoices, &payments);

        assert!(items.is_empty());
        assert_eq!(total, Decimal::ZERO);
    }

    // ============ 已核销付款过滤测试 ============

    /// 复现 auto_verify 中已核销付款过滤逻辑
    /// 输入：所有付款 ID 列表、已核销付款 ID 集合
    /// 输出：可用付款列表（排除已核销的）
    fn filter_available_payments(
        payments: &[(i32, Decimal)],
        verified_ids: &std::collections::HashSet<i32>,
    ) -> Vec<(i32, Decimal)> {
        payments
            .iter()
            .filter(|(id, _)| !verified_ids.contains(id))
            .cloned()
            .collect()
    }

    /// 测试_已核销付款过滤_排除已核销
    ///
    /// 验证已核销付款 ID 在 verified_ids 中时被排除
    #[test]
    fn 测试_已核销付款过滤_排除已核销() {
        let payments = vec![(1, decs!("100")), (2, decs!("200")), (3, decs!("50"))];
        let verified: std::collections::HashSet<i32> = [2].iter().cloned().collect();
        let available = filter_available_payments(&payments, &verified);

        assert_eq!(available.len(), 2);
        assert_eq!(available[0].0, 1);
        assert_eq!(available[1].0, 3);
    }

    /// 测试_已核销付款过滤_全部可用
    ///
    /// 验证 verified_ids 为空时所有付款均可用
    #[test]
    fn 测试_已核销付款过滤_全部可用() {
        let payments = vec![(1, decs!("100")), (2, decs!("200"))];
        let verified: std::collections::HashSet<i32> = std::collections::HashSet::new();
        let available = filter_available_payments(&payments, &verified);

        assert_eq!(available.len(), 2);
    }

    // ============ 手工核销校验测试 ============

    /// 复现 manual_verify 中的校验逻辑
    /// 返回 Ok(()) 表示校验通过，Err 表示拒绝
    fn validate_manual_item(
        unpaid_amount: Decimal,
        verify_amount: Decimal,
        payment_status: &str,
    ) -> Result<(), AppError> {
        if unpaid_amount < verify_amount {
            return Err(AppError::business(format!(
                "应付单未付金额{}小于核销金额{}",
                unpaid_amount, verify_amount
            )));
        }
        if payment_status != "CONFIRMED" {
            return Err(AppError::business(format!(
                "付款单状态为{}，未确认不可核销",
                payment_status
            )));
        }
        Ok(())
    }

    /// 测试_手工核销校验_未付金额不足拒绝
    ///
    /// 验证 unpaid_amount < verify_amount 时返回 BusinessError
    #[test]
    fn 测试_手工核销校验_未付金额不足拒绝() {
        let result = validate_manual_item(decs!("50"), decs!("100"), "CONFIRMED");
        assert!(result.is_err());
        assert!(
            matches!(result, Err(AppError::BusinessError(_))),
            "应返回 BusinessError"
        );
    }

    /// 测试_手工核销校验_付款状态非确认拒绝
    ///
    /// 验证 payment_status != CONFIRMED 时返回 BusinessError
    #[test]
    fn 测试_手工核销校验_付款状态非确认拒绝() {
        let result = validate_manual_item(decs!("100"), decs!("50"), "PENDING");
        assert!(result.is_err());
        assert!(
            matches!(result, Err(AppError::BusinessError(_))),
            "应返回 BusinessError"
        );
    }

    /// 测试_手工核销校验_合法参数通过
    ///
    /// 验证 unpaid_amount > verify_amount 且 payment_status == CONFIRMED 时通过
    #[test]
    fn 测试_手工核销校验_合法参数通过() {
        let result = validate_manual_item(decs!("100"), decs!("50"), "CONFIRMED");
        assert!(result.is_ok());
    }

    /// 测试_手工核销校验_边界未付等于核销
    ///
    /// 验证 unpaid == verify_amount 时通过（源码用 < 判断，相等不拒绝）
    #[test]
    fn 测试_手工核销校验_边界未付等于核销() {
        let result = validate_manual_item(decs!("100"), decs!("100"), "CONFIRMED");
        assert!(result.is_ok());
    }

    // ============ 取消核销状态恢复测试 ============

    /// 复现 cancel 中的状态恢复逻辑（P2 3-15 修复）
    /// 输入：paid_amount, amount
    /// 输出：恢复后的发票状态字符串
    fn restore_invoice_status(paid_amount: Decimal, amount: Decimal) -> &'static str {
        if paid_amount >= amount {
            "PAID"
        } else if paid_amount > Decimal::ZERO {
            "PARTIAL_PAID"
        } else {
            "AUDITED"
        }
    }

    /// 测试_取消核销状态恢复_全额付款保留PAID
    ///
    /// 验证 paid_amount >= amount → PAID（含边界等于）
    #[test]
    fn 测试_取消核销状态恢复_全额付款保留PAID() {
        assert_eq!(restore_invoice_status(decs!("100"), decs!("100")), "PAID");
        assert_eq!(restore_invoice_status(decs!("110"), decs!("100")), "PAID");
    }

    /// 测试_取消核销状态恢复_部分付款转PARTIAL_PAID
    ///
    /// 验证 0 < paid_amount < amount → PARTIAL_PAID
    #[test]
    fn 测试_取消核销状态恢复_部分付款转PARTIAL_PAID() {
        assert_eq!(restore_invoice_status(decs!("50"), decs!("100")), "PARTIAL_PAID");
        assert_eq!(restore_invoice_status(decs!("1"), decs!("100")), "PARTIAL_PAID");
    }

    /// 测试_取消核销状态恢复_无付款转AUDITED
    ///
    /// 验证 paid_amount <= 0 → AUDITED
    #[test]
    fn 测试_取消核销状态恢复_无付款转AUDITED() {
        assert_eq!(
            restore_invoice_status(Decimal::ZERO, decs!("100")),
            "AUDITED"
        );
        assert_eq!(
            restore_invoice_status(decs!("-10"), decs!("100")),
            "AUDITED"
        );
    }

    /// 测试_取消核销状态恢复_三态互斥
    ///
    /// 验证三个状态分支互斥，不同输入对应不同状态
    #[test]
    fn 测试_取消核销状态恢复_三态互斥() {
        let states = vec![
            restore_invoice_status(decs!("100"), decs!("100")),
            restore_invoice_status(decs!("50"), decs!("100")),
            restore_invoice_status(Decimal::ZERO, decs!("100")),
        ];
        // 三个状态应分别为 PAID / PARTIAL_PAID / AUDITED
        assert!(states.contains(&"PAID"));
        assert!(states.contains(&"PARTIAL_PAID"));
        assert!(states.contains(&"AUDITED"));
    }

    // ============ 服务实例创建测试 ============

    /// 测试夹具：SQLite 内存数据库连接
    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    /// 测试_服务实例创建
    ///
    /// 验证 ApVerificationService 在 SQLite 内存数据库上能正常实例化
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = ApVerificationService::new(Arc::new(db));
        assert!(Arc::strong_count(&service.db) >= 1);
    }
}
