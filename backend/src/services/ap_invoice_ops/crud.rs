//! 应付单 CRUD + 状态流转 impl 子模块（ap_invoice_ops/crud）
//!
//! 批次 490 D10-4b 拆分：从原 `ap_invoice_service.rs` L428-791 迁移。
//! 包含 ApInvoiceService 的 8 个方法（手工 CRUD + 状态机流转 + 查询）：
//! - create_manual（手工创建应付单，金额精度校验）
//! - update / delete（仅 DRAFT 状态可改/删，lock_exclusive 串行化）
//! - approve（DRAFT → AUDITED）
//! - mark_as_paid（AUDITED/PARTIAL_PAID → PAID，白名单状态门 P0 3-3 修复）
//! - cancel（AUDITED/PARTIAL_PAID → CANCELLED，需 paid_amount 为 0）
//! - get_by_id / get_list（查询 + 分页筛选）
//!
//! 业务规则：
//! - 仅 DRAFT 状态可修改/删除
//! - 状态机：DRAFT → AUDITED → PAID；AUDITED/PARTIAL_PAID → CANCELLED
//! - 并发状态变更通过 lock_exclusive 串行化（批次 26 v6 P1 修复）

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use crate::models::ap_invoice;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::services::ap_invoice_service::{
    ApInvoiceListQuery, ApInvoiceService, CreateApInvoiceRequest, DEFAULT_BASE_CURRENCY_EXCHANGE_RATE,
    UpdateApInvoiceRequest,
};

impl ApInvoiceService {
    /// 手工创建应付单
    pub async fn create_manual(
        &self,
        req: CreateApInvoiceRequest,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // P2-4 修复（批次 84 v1 复审）：金额精度校验，最多 2 位小数（货币精度）
        if let Some(amount) = req.amount {
            if amount.round_dp(2) != amount {
                return Err(AppError::validation("应付单金额精度不能超过 2 位小数"));
            }
        }

        // 1. 生成应付单号
        let invoice_no = self.generate_invoice_no().await?;

        // 2. 创建应付单
        // 供应商 ID 缺失时拒绝创建，避免脏 supplier_id=0 记录
        let invoice = ap_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            supplier_id: Set(req
                .supplier_id
                .ok_or_else(|| AppError::validation("应付单缺少供应商ID"))?),
            invoice_type: Set(req.invoice_type.unwrap_or_else(|| "PURCHASE".to_string())),
            source_type: Set(Some("MANUAL".to_string())),
            source_id: Set(None),
            invoice_date: Set(req
                .invoice_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive())),
            due_date: Set(req
                .due_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive())),
            payment_terms: Set(req.payment_terms.unwrap_or(crate::constants::DEFAULT_PAYMENT_TERMS_DAYS)),
            amount: Set(req.amount.unwrap_or(Decimal::ZERO)),
            paid_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(req.amount.unwrap_or(Decimal::ZERO)),
            invoice_status: Set(crate::models::status::common::STATUS_DRAFT.to_string()),
            currency: Set(req.currency.unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(DEFAULT_BASE_CURRENCY_EXCHANGE_RATE)),
            tax_amount: Set(req.tax_amount.unwrap_or(Decimal::ZERO)),
            notes: Set(req.notes),
            attachment_urls: Set(req.attachment_urls),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 更新应付单（仅草稿状态）
    ///
    /// 批次 86 v2 复审 P2-4 修复：find_by_id 后追加 lock_exclusive 串行化并发状态变更
    pub async fn update(
        &self,
        id: i32,
        req: UpdateApInvoiceRequest,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单（加 lock_exclusive 串行化）
        let invoice = ap_invoice::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付单 {}", id)))?;

        // 2. 检查状态（仅草稿可修改）
        if invoice.invoice_status != crate::models::status::common::STATUS_DRAFT {
            return Err(AppError::business(format!(
                "应付单状态为{}，不可修改",
                invoice.invoice_status
            )));
        }

        // 3. 更新应付单
        let original_paid_amount = invoice.paid_amount;
        let mut invoice_active: ap_invoice::ActiveModel = invoice.into();

        if let Some(invoice_type) = req.invoice_type {
            invoice_active.invoice_type = Set(invoice_type);
        }
        if let Some(invoice_date) = req.invoice_date {
            invoice_active.invoice_date = Set(invoice_date);
        }
        if let Some(due_date) = req.due_date {
            invoice_active.due_date = Set(due_date);
        }
        if let Some(payment_terms) = req.payment_terms {
            invoice_active.payment_terms = Set(payment_terms);
        }
        if let Some(amount) = req.amount {
            // P2-4 修复（批次 84 v1 复审）：金额精度校验，最多 2 位小数（货币精度）
            if amount.round_dp(2) != amount {
                return Err(AppError::validation("应付单金额精度不能超过 2 位小数"));
            }
            invoice_active.amount = Set(amount);
            invoice_active.unpaid_amount = Set(amount - original_paid_amount);
        }
        if let Some(notes) = req.notes {
            invoice_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            invoice_active.attachment_urls = Set(Some(attachment_urls));
        }

        invoice_active.updated_by = Set(Some(user_id));

        // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID，
        // 原 Some(0) 硬编码导致审计日志无法追溯修改人。
        let invoice = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            invoice_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 删除应付单（仅草稿状态）
    ///
    /// 批次 86 v2 复审 P2-5 修复：find_by_id 后追加 lock_exclusive 串行化并发状态变更
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单（加 lock_exclusive 串行化）
        let invoice = ap_invoice::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付单 {}", id)))?;

        // 2. 检查状态（仅草稿可删除）
        if invoice.invoice_status != crate::models::status::common::STATUS_DRAFT {
            return Err(AppError::business(format!(
                "应付单状态为{}，不可删除",
                invoice.invoice_status
            )));
        }

        // 3. 删除应付单（P0 8-3 修复：补审计日志，事务内 find→delete→audit 三步原子）
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            ap_invoice::Entity,
            _,
        >(&txn, "ap_invoice", invoice.id, Some(user_id))
        .await?;

        txn.commit().await?;

        Ok(())
    }

    /// 审核应付单
    pub async fn approve(&self, id: i32, user_id: i32) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单（批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更）
        let invoice = ap_invoice::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付单 {}", id)))?;

        // 2. 检查状态
        if invoice.invoice_status != crate::models::status::common::STATUS_DRAFT {
            return Err(AppError::business(format!(
                "应付单状态为{}，不可审核",
                invoice.invoice_status
            )));
        }

        // 3. 审核应付单
        let now = Utc::now();
        let mut invoice_active: ap_invoice::ActiveModel = invoice.into();
        invoice_active.invoice_status = Set(crate::models::status::ap_invoice::INVOICE_AUDITED.to_string());
        invoice_active.approved_by = Set(Some(user_id));
        invoice_active.approved_at = Set(Some(now));
        invoice_active.updated_at = Set(now);

        // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID
        let invoice = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            invoice_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 标记应付单为已付清
    ///
    /// `user_id` 为触发本次状态变更的操作人 ID，用于审计日志透传。
    /// 通常由事件总线监听 `PaymentCompleted` 事件后调用，
    /// 事件 payload 携带付款操作人 ID。
    pub async fn mark_as_paid(&self, id: i32, user_id: i32) -> Result<ap_invoice::Model, AppError> {
        // 批次 11（2026-06-28）：事务包裹"状态变更 + 审计日志"，保证原子性
        // 原实现直接 &*self.db 调用 update_with_audit，内部 2 次独立写入非原子；
        // 异步事件驱动场景下若审计写入失败，发票已 PAID 但无审计，且难以发现。
        // 批次 22（2026-06-28 v5 P0-1）：状态门查询加 lock_exclusive 串行化并发 mark_as_paid
        // 原实现状态门无锁，两并发 mark_as_paid 均通过状态检查后基于过期状态写入，
        // 导致 paid_amount 重复累加（资金双重支付风险）。
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单（加 lock_exclusive 串行化并发 mark_as_paid）
        let invoice = ap_invoice::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付单 {}", id)))?;

        // 2. 检查状态
        // P0 3-3 修复（2026-07-01 八维度审计）：状态门改为白名单，仅 AUDITED/PARTIAL_PAID 可标记 PAID，
        // 堵住 DRAFT/PENDING 直接跳过审核标记已付清的漏洞。
        if ![
            crate::models::status::ap_invoice::INVOICE_AUDITED,
            crate::models::status::payment::PAYMENT_PARTIAL_PAID,
        ]
        .contains(&invoice.invoice_status.as_str())
        {
            return Err(AppError::business(format!(
                "应付单状态为{}，不可标记为已付清（仅 AUDITED/PARTIAL_PAID 可标记）",
                invoice.invoice_status
            )));
        }

        // 3. 更新状态
        let now = Utc::now();
        let mut invoice_active: ap_invoice::ActiveModel = invoice.into();
        invoice_active.invoice_status = Set(crate::models::status::payment::PAYMENT_PAID.to_string());
        invoice_active.updated_at = Set(now);

        // 批次 97 P1-2 修复：原 Some(0) 占位符改为真实操作人 user_id
        // 事件总线 PaymentCompleted 事件携带付款操作人 ID，透传给审计日志
        let invoice = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            invoice_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 取消应付单
    pub async fn cancel(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单（批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更）
        let invoice = ap_invoice::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付单 {}", id)))?;

        // 2. 检查状态（已审核或部分付款可取消）
        if ![
            crate::models::status::ap_invoice::INVOICE_AUDITED,
            crate::models::status::payment::PAYMENT_PARTIAL_PAID,
        ]
        .contains(&invoice.invoice_status.as_str())
        {
            return Err(AppError::business(format!(
                "应付单状态为{}，不可取消",
                invoice.invoice_status
            )));
        }

        // 3. 检查是否已付款
        if invoice.paid_amount > Decimal::ZERO {
            return Err(AppError::business(
                "应付单已有付款记录，不可取消".to_string(),
            ));
        }

        // 4. 取消应付单
        let now = Utc::now();
        let mut invoice_active: ap_invoice::ActiveModel = invoice.into();
        invoice_active.invoice_status = Set(crate::models::status::common::STATUS_CANCELLED.to_string());
        invoice_active.cancelled_by = Set(Some(user_id));
        invoice_active.cancelled_at = Set(Some(now));
        invoice_active.cancelled_reason = Set(Some(reason));
        invoice_active.updated_at = Set(now);

        // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID
        let invoice = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            invoice_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 获取应付单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_invoice::Model, AppError> {
        let invoice = ap_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付单 {}", id)))?;

        Ok(invoice)
    }

    /// 获取应付单列表（含分页、筛选）
    pub async fn get_list(
        &self,
        params: ApInvoiceListQuery,
    ) -> Result<(Vec<ap_invoice::Model>, u64), AppError> {
        let mut query = ap_invoice::Entity::find();

        // 筛选条件
        if let Some(sid) = params.supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }
        if let Some(status) = params.invoice_status {
            query = query.filter(ap_invoice::Column::InvoiceStatus.eq(status));
        }
        if let Some(itype) = params.invoice_type {
            query = query.filter(ap_invoice::Column::InvoiceType.eq(itype));
        }
        if let Some(sd) = params.start_date {
            query = query.filter(ap_invoice::Column::InvoiceDate.gte(sd));
        }
        if let Some(ed) = params.end_date {
            query = query.filter(ap_invoice::Column::InvoiceDate.lte(ed));
        }

        // 分页
        let paginator = query
            .order_by(ap_invoice::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, params.page_size);

        // 批次 255 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 0-indexed 偏移）
        let (items, total) = paginate_with_total(paginator, params.page.clamp(1, 1000)).await?;

        Ok((items, total))
    }
}
