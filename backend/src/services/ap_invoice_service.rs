//! 应付单 Service
//!
//! 应付单服务层，负责应付单的核心业务逻辑
//! 包含应付单自动生成、手工创建、审核、核销等全流程管理

use crate::models::{ap_invoice, purchase_receipt, purchase_return};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

// 批次 102 v6 P3-2 修复：状态字符串常量化，引用 crate::models::status

/// 默认本位币汇率（CNY 本位币 = 1.0）。
///
/// 历史缺陷（P0-1，2026-06-25 综合审计）：自动生成 AP 发票时曾误用
/// `Decimal::new(1, 2)` = 0.01，导致下游按汇率换算本位币金额被缩小 100 倍。
/// 抽取为常量并在单元测试中断言其值，避免再次被改错。
///
/// 注意：`Decimal::new` 不是 const fn，不能用于 const 初始化；
/// 使用 rust_decimal 提供的 const 关联常量 `Decimal::ONE`（= 1.0）。
pub const DEFAULT_BASE_CURRENCY_EXCHANGE_RATE: Decimal = Decimal::ONE;

/// 应付单服务
pub struct ApInvoiceService {
    db: Arc<DatabaseConnection>,
}

impl ApInvoiceService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成应付单号
    // 格式：AP + 年月日 + 三位序号（AP20260315001）
    crate::impl_generate_no!(
        generate_invoice_no,
        "API",
        ap_invoice::Entity,
        ap_invoice::Column::InvoiceNo
    );

    /// 从采购入库单自动生成应付单
    pub async fn auto_generate_from_receipt(
        &self,
        receipt_id: i32,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询采购入库单
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查是否已生成应付
        let exists = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RECEIPT"))
            .filter(ap_invoice::Column::SourceId.eq(receipt_id))
            .one(&txn)
            .await?;

        if exists.is_some() {
            return Err(AppError::business("该入库单已生成应付单"));
        }

        // 3. 获取供应商信息
        let _supplier = crate::models::supplier::Entity::find_by_id(receipt.supplier_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商 {}", receipt.supplier_id)))?;

        // 使用默认账期 30 天
        let payment_terms = crate::constants::DEFAULT_PAYMENT_TERMS_DAYS;

        // 4. 生成应付单
        let invoice_no = self.generate_invoice_no().await?;
        let invoice_date = receipt.receipt_date;
        let due_date = invoice_date + Duration::days(payment_terms as i64);

        let invoice = ap_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            supplier_id: Set(receipt.supplier_id),
            invoice_type: Set("PURCHASE".to_string()),
            source_type: Set(Some("PURCHASE_RECEIPT".to_string())),
            source_id: Set(Some(receipt_id)),
            invoice_date: Set(invoice_date),
            due_date: Set(due_date),
            payment_terms: Set(payment_terms),
            amount: Set(receipt.total_amount),
            paid_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(receipt.total_amount),
            // P0 3-1 修复（2026-07-01 八维度审计）：自动生成 AP 发票初始为 DRAFT，
            // 经 approve 流程审核后转 AUDITED 才能进入付款环节。
            // 原 PENDING 状态不在 approve 状态机枚举内（仅 DRAFT→AUDITED），
            // 导致自动生成的应付单死锁在 PENDING 永远无法审批。
            invoice_status: Set(crate::models::status::common::STATUS_DRAFT.to_string()),
            currency: Set(crate::constants::DEFAULT_CURRENCY.to_string()),
            exchange_rate: Set(DEFAULT_BASE_CURRENCY_EXCHANGE_RATE),
            // P1-10 修复：tax_amount 应从源单据税额传递。
            // purchase_receipt 主表与 purchase_receipt_item 均无 tax_amount 字段
            // （模型设计不记录税额），暂保持 ZERO。
            // TODO(tech-debt): receipt 模型补充 tax_amount 字段后从源单据传递。
            tax_amount: Set(Decimal::ZERO),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // F-P0-7 修复（批次 382 v13 复审）：commit 前保存生成应付凭证所需字段
        // invoice 在 Ok 返回时被 move，提前捕获避免后续 voucher 生成无法访问
        let voucher_invoice_no = invoice.invoice_no.clone();
        let voucher_invoice_id = invoice.id;
        let voucher_supplier_id = invoice.supplier_id;
        let voucher_amount = invoice.amount;
        let voucher_tax_amount = invoice.tax_amount;
        let voucher_invoice_date = invoice.invoice_date;

        txn.commit().await?;

        // F-P0-7 修复（批次 382 v13 复审）：采购入库生成应付单后同步生成应付凭证
        // 借：1405 库存商品（不含税金额）
        // 借：222101 应交税费-进项税额（tax_amount > 0 时）
        // 贷：2202 应付账款（含税总额，挂供应商辅助核算）
        // 失败时仅 warn 不阻断主流程（与采购入库 confirm_receipt 容错模式一致），
        // 避免凭证生成失败影响主业务流程，便于人工补偿。
        let voucher_total = voucher_amount + voucher_tax_amount;
        if voucher_total > Decimal::ZERO {
            let mut voucher_items = vec![
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: Some(1),
                    subject_code: Some("1405".to_string()),
                    subject_name: Some("库存商品".to_string()),
                    debit: voucher_amount,
                    credit: Decimal::ZERO,
                    summary: Some(format!("采购入库应付确认-{}", voucher_invoice_no)),
                    assist_customer_id: None,
                    assist_supplier_id: Some(voucher_supplier_id),
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
                    subject_code: Some("2202".to_string()),
                    subject_name: Some("应付账款".to_string()),
                    debit: Decimal::ZERO,
                    credit: voucher_total,
                    summary: Some(format!("采购入库应付确认-{}", voucher_invoice_no)),
                    assist_customer_id: None,
                    assist_supplier_id: Some(voucher_supplier_id),
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
            ];
            // 进项税额仅在 tax_amount > 0 时插入，避免零额凭证分录引起校验告警
            if voucher_tax_amount > Decimal::ZERO {
                voucher_items.insert(
                    1,
                    crate::services::voucher_service::VoucherItemRequest {
                        line_no: Some(2),
                        subject_code: Some("222101".to_string()),
                        subject_name: Some("应交税费-应交增值税-进项税额".to_string()),
                        debit: voucher_tax_amount,
                        credit: Decimal::ZERO,
                        summary: Some(format!("采购入库应付确认-{}", voucher_invoice_no)),
                        assist_customer_id: None,
                        assist_supplier_id: Some(voucher_supplier_id),
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
                );
                // 重排行号：1=库存商品 / 2=进项税额 / 3=应付账款
                voucher_items[2].line_no = Some(3);
            }
            let voucher_req = crate::services::voucher_service::CreateVoucherRequest {
                voucher_type: "转".to_string(),
                voucher_date: voucher_invoice_date,
                source_type: Some("PURCHASE_RECEIPT".to_string()),
                source_module: Some("purchase".to_string()),
                source_bill_id: Some(voucher_invoice_id),
                source_bill_no: Some(voucher_invoice_no.clone()),
                batch_no: None,
                color_no: None,
                items: voucher_items,
            };
            let voucher_service =
                crate::services::voucher_service::VoucherService::new(self.db.clone());
            if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
                tracing::warn!(
                    "应付单 {} 应付凭证生成失败，需人工补生成：{}",
                    voucher_invoice_no,
                    e
                );
            }
        }

        Ok(invoice)
    }

    /// 从采购退货单自动生成应付单（红字）
    pub async fn auto_generate_from_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询采购退货单
        let return_doc = purchase_return::Entity::find_by_id(return_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购退货单 {}", return_id)))?;

        // 2. 检查是否已生成应付
        let exists = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RETURN"))
            .filter(ap_invoice::Column::SourceId.eq(return_id))
            .one(&txn)
            .await?;

        if exists.is_some() {
            return Err(AppError::business(
                "该退货单已生成应付单（红字）".to_string(),
            ));
        }

        // 3. 生成红字应付单（负数）
        let invoice_no = self.generate_invoice_no().await?;
        let invoice_date = return_doc.return_date;
        let payment_terms = 0; // 退货立即冲销
        let due_date = invoice_date;

        // 退货金额为负数
        let amount = -return_doc.total_amount.unwrap_or(Decimal::ZERO);

        // P1-10 修复（2026-06-25 综合审计）：从退货单明细汇总税额。
        // purchase_return_item 表有 tax_amount 字段，按 return_id 汇总。
        let tax_amount: Decimal = crate::models::purchase_return_item::Entity::find()
            .filter(crate::models::purchase_return_item::Column::ReturnId.eq(return_id))
            .all(&txn)
            .await?
            .iter()
            .fold(Decimal::ZERO, |acc, item| acc + item.tax_amount);
        // 退货税额为负数（红字冲销）
        let tax_amount = -tax_amount;

        let invoice = ap_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            supplier_id: Set(return_doc.supplier_id),
            invoice_type: Set("PURCHASE".to_string()),
            source_type: Set(Some("PURCHASE_RETURN".to_string())),
            source_id: Set(Some(return_id)),
            invoice_date: Set(invoice_date),
            due_date: Set(due_date),
            payment_terms: Set(payment_terms),
            amount: Set(amount),
            paid_amount: Set(Decimal::ZERO),
            // P2 3-16 修复：红字应付单（amount 为负数）不需要再支付，
            // unpaid_amount 不能等于 amount（负数），应为 0。
            // 原 unpaid_amount: Set(amount) 导致待支付金额为负数，业务语义错误。
            unpaid_amount: Set(Decimal::ZERO),
            // P0 3-1 修复：初始为 DRAFT，经 approve 流程审核后转 AUDITED
            invoice_status: Set(crate::models::status::common::STATUS_DRAFT.to_string()),
            currency: Set(crate::constants::DEFAULT_CURRENCY.to_string()),
            exchange_rate: Set(DEFAULT_BASE_CURRENCY_EXCHANGE_RATE),
            // P1-10 修复：从退货单明细汇总税额（负数红字）
            tax_amount: Set(tax_amount),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

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
        supplier_id: Option<i32>,
        invoice_status: Option<String>,
        invoice_type: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ap_invoice::Model>, u64), AppError> {
        let mut query = ap_invoice::Entity::find();

        // 筛选条件
        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }
        if let Some(status) = invoice_status {
            query = query.filter(ap_invoice::Column::InvoiceStatus.eq(status));
        }
        if let Some(itype) = invoice_type {
            query = query.filter(ap_invoice::Column::InvoiceType.eq(itype));
        }
        if let Some(sd) = start_date {
            query = query.filter(ap_invoice::Column::InvoiceDate.gte(sd));
        }
        if let Some(ed) = end_date {
            query = query.filter(ap_invoice::Column::InvoiceDate.lte(ed));
        }

        // 分页
        let paginator = query
            .order_by(ap_invoice::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        // 批次 255 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 0-indexed 偏移）
        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 获取账龄分析
    pub async fn get_aging_analysis(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<Vec<AgingAnalysisItem>, AppError> {
        let mut query = ap_invoice::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }

        // 查询未付清的应付单
        let invoices = query
            .filter(ap_invoice::Column::InvoiceStatus.ne(crate::models::status::payment::PAYMENT_PAID))
            .filter(ap_invoice::Column::InvoiceStatus.ne(crate::models::status::common::STATUS_CANCELLED))
            .all(&*self.db)
            .await?;

        let today = Utc::now().naive_utc().date();
        let mut aging_map: std::collections::BTreeMap<String, AgingAnalysisItem> =
            std::collections::BTreeMap::new();

        for invoice in invoices {
            let unpaid = invoice.unpaid_amount;
            let days_overdue = if invoice.due_date < today {
                (today - invoice.due_date).num_days() as i32
            } else {
                -1 // 未到期
            };

            // 按账龄区间分类
            let aging_bucket = if days_overdue < 0 {
                "未到期".to_string()
            } else if days_overdue <= 30 {
                "逾期 1-30 天".to_string()
            } else if days_overdue <= 60 {
                "逾期 31-60 天".to_string()
            } else if days_overdue <= 90 {
                "逾期 61-90 天".to_string()
            } else if days_overdue <= 180 {
                "逾期 91-180 天".to_string()
            } else {
                "逾期 180 天以上".to_string()
            };

            let entry =
                aging_map
                    .entry(aging_bucket.clone())
                    .or_insert_with(|| AgingAnalysisItem {
                        aging_bucket,
                        invoice_count: 0,
                        total_amount: Decimal::ZERO,
                    });

            entry.invoice_count += 1;
            entry.total_amount += unpaid;
        }

        Ok(aging_map.into_values().collect())
    }

    /// 获取应付余额表
    pub async fn get_balance_summary(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<BalanceSummary, AppError> {
        let mut query = ap_invoice::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }

        // 查询所有有效应付单
        let invoices = query
            .filter(ap_invoice::Column::InvoiceStatus.ne(crate::models::status::common::STATUS_CANCELLED))
            .all(&*self.db)
            .await?;

        let mut summary = BalanceSummary {
            total_invoice_amount: Decimal::ZERO,
            total_paid_amount: Decimal::ZERO,
            total_unpaid_amount: Decimal::ZERO,
            invoice_count: 0,
        };

        for invoice in invoices {
            summary.total_invoice_amount += invoice.amount;
            summary.total_paid_amount += invoice.paid_amount;
            summary.total_unpaid_amount += invoice.unpaid_amount;
            summary.invoice_count += 1;
        }

        Ok(summary)
    }

    /// 获取应付统计报表
    ///
    /// 批次 133 v9 复审 P1：原 handler get_statistics 返回 "统计报表功能开发中" 占位，
    /// 现综合调用 get_balance_summary + get_aging_analysis + 按状态分组统计，
    /// 返回完整统计报表（余额汇总 + 账龄分析 + 状态分布）。
    pub async fn get_statistics(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<ApInvoiceStatistics, AppError> {
        // 1. 余额汇总（排除已取消）
        let balance_summary = self.get_balance_summary(supplier_id).await?;

        // 2. 账龄分析（未付清的应付单）
        let aging_analysis = self.get_aging_analysis(supplier_id).await?;

        // 3. 按状态分组统计
        let mut query = ap_invoice::Entity::find();
        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }
        let all_invoices = query.all(&*self.db).await?;

        let mut status_map: std::collections::BTreeMap<String, StatusStatItem> =
            std::collections::BTreeMap::new();
        for invoice in all_invoices {
            let entry = status_map
                .entry(invoice.invoice_status.clone())
                .or_insert_with(|| StatusStatItem {
                    status: invoice.invoice_status.clone(),
                    invoice_count: 0,
                    total_amount: Decimal::ZERO,
                });
            entry.invoice_count += 1;
            entry.total_amount += invoice.amount;
        }
        let status_distribution: Vec<StatusStatItem> = status_map.into_values().collect();

        Ok(ApInvoiceStatistics {
            balance_summary,
            aging_analysis,
            status_distribution,
        })
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 创建应付单请求
///
/// TS-S-5 安全加固（2026-06-26）：补齐 exchange_rate / amount / currency / notes / attachment_urls 校验，
/// 防止手工传入 0.01 汇率（P0-1）或负数金额。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateApInvoiceRequest {
    /// 供应商 ID
    pub supplier_id: Option<i32>,

    /// 应付类型
    #[validate(length(min = 1, max = 20, message = "发票号码长度必须在1到20个字符之间"))]
    pub invoice_type: Option<String>,

    /// 应付日期
    pub invoice_date: Option<NaiveDate>,

    /// 到期日期
    pub due_date: Option<NaiveDate>,

    /// 账期（天）
    #[validate(range(min = 0, max = 365, message = "账期必须在0到365天之间"))]
    pub payment_terms: Option<i32>,

    /// 应付金额（必须为正数）
    #[validate(custom(function = "validate_positive_decimal"))]
    pub amount: Option<Decimal>,

    /// 币种（ISO 4217 三字母代码）
    #[validate(length(equal = 3, message = "币种必须为 ISO 4217 三字母代码"))]
    pub currency: Option<String>,

    /// 汇率（必须大于 0，防止 P0-1 历史缺陷的 0.01 汇率再次发生）
    #[validate(custom(function = "validate_exchange_rate"))]
    pub exchange_rate: Option<Decimal>,

    /// 税额（必须非负）
    #[validate(custom(function = "validate_non_negative_decimal"))]
    pub tax_amount: Option<Decimal>,

    /// 备注
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub notes: Option<String>,

    /// 附件 URL 列表
    #[validate(length(max = 10, message = "附件数量不能超过10个"))]
    pub attachment_urls: Option<Vec<String>>,
}

/// 更新应付单请求
///
/// TS-S-5 安全加固（2026-06-26）：补齐字段校验，与 CreateApInvoiceRequest 保持一致。
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateApInvoiceRequest {
    /// 应付类型
    #[validate(length(min = 1, max = 20, message = "发票号码长度必须在1到20个字符之间"))]
    pub invoice_type: Option<String>,

    /// 应付日期
    pub invoice_date: Option<NaiveDate>,

    /// 到期日期
    pub due_date: Option<NaiveDate>,

    /// 账期（天）
    #[validate(range(min = 0, max = 365, message = "账期必须在0到365天之间"))]
    pub payment_terms: Option<i32>,

    /// 应付金额（必须为正数）
    #[validate(custom(function = "validate_positive_decimal"))]
    pub amount: Option<Decimal>,

    /// 备注
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub notes: Option<String>,

    /// 附件 URL 列表
    #[validate(length(max = 10, message = "附件数量不能超过10个"))]
    pub attachment_urls: Option<Vec<String>>,
}

// =====================================================
// DTO 校验函数（TS-S-5 安全加固）
// =====================================================

/// 校验 Decimal 为正数
fn validate_positive_decimal(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value <= Decimal::ZERO {
        return Err(validator::ValidationError::new("金额必须为正数"));
    }
    Ok(())
}

/// 校验 Decimal 为非负数
fn validate_non_negative_decimal(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value < Decimal::ZERO {
        return Err(validator::ValidationError::new("金额不能为负数"));
    }
    Ok(())
}

/// 校验汇率合法：必须大于 0 且不等于 P0-1 历史缺陷值 0.01
fn validate_exchange_rate(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value <= Decimal::ZERO {
        return Err(validator::ValidationError::new("汇率必须大于0"));
    }
    // P0-1 防护：拒绝 0.01 汇率（历史缺陷值）
    if *value == Decimal::new(1, 2) {
        return Err(validator::ValidationError::new(
            "汇率不能为0.01（P0-1历史缺陷值，本位币汇率应为1.0）",
        ));
    }
    Ok(())
}

/// 账龄分析项
#[derive(Debug, Serialize, Deserialize)]
pub struct AgingAnalysisItem {
    /// 账龄区间
    pub aging_bucket: String,

    /// 应付单数量
    pub invoice_count: i64,

    /// 总金额
    pub total_amount: Decimal,
}

/// 应付余额汇总
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceSummary {
    /// 应付总金额
    pub total_invoice_amount: Decimal,

    /// 已付总金额
    pub total_paid_amount: Decimal,

    /// 未付总金额
    pub total_unpaid_amount: Decimal,

    /// 应付单数量
    pub invoice_count: i64,
}

/// 应付状态分布项（批次 133 v9 复审 P1）
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusStatItem {
    /// 状态（DRAFT/AUDITED/PARTIAL_PAID/PAID/CANCELLED）
    pub status: String,
    /// 该状态下的应付单数量
    pub invoice_count: i64,
    /// 该状态下的应付总金额
    pub total_amount: Decimal,
}

/// 应付统计报表（批次 133 v9 复审 P1）
///
/// 综合 3 个维度的统计：
/// 1. 余额汇总（balance_summary）：总应付 / 已付 / 未付 / 数量
/// 2. 账龄分析（aging_analysis）：按账龄区间分组的未付清应付单
/// 3. 状态分布（status_distribution）：按状态分组的应付单
#[derive(Debug, Serialize, Deserialize)]
pub struct ApInvoiceStatistics {
    /// 余额汇总
    pub balance_summary: BalanceSummary,
    /// 账龄分析
    pub aging_analysis: Vec<AgingAnalysisItem>,
    /// 状态分布
    pub status_distribution: Vec<StatusStatItem>,
}

#[cfg(test)]
mod tests {
    //! AP 发票服务单元测试
    //!
    //! 覆盖目标：
    //! - DEFAULT_BASE_CURRENCY_EXCHANGE_RATE 常量值正确性（防止 P0-1 缺陷复发）
    //! - 汇率换算逻辑（金额 × 汇率 = 本位币金额）

    use super::*;

    /// 防止 P0-1 缺陷复发：默认本位币汇率必须是 1.0，不能是 0.01。
    ///
    /// 历史缺陷：`Decimal::new(1, 2)` 误用导致自动生成 AP 发票汇率被设为 0.01，
    /// 下游按汇率换算本位币金额的财务计算被缩小 100 倍。
    #[test]
    fn test_default_exchange_rate_is_one_not_zero_dot_zero_one() {
        assert_eq!(
            DEFAULT_BASE_CURRENCY_EXCHANGE_RATE,
            Decimal::new(1, 0),
            "默认本位币汇率应为 1.0，当前值 {:?} 不正确（P0-1 缺陷复发风险）",
            DEFAULT_BASE_CURRENCY_EXCHANGE_RATE
        );
        // 数值断言：1.0 而非 0.01
        assert_eq!(DEFAULT_BASE_CURRENCY_EXCHANGE_RATE, Decimal::ONE);
        assert_ne!(
            DEFAULT_BASE_CURRENCY_EXCHANGE_RATE,
            Decimal::new(1, 2),
            "默认汇率不应为 0.01"
        );
    }

    /// 验证按默认汇率换算本位币金额：金额 × 1.0 = 金额本身。
    ///
    /// 该测试模拟下游按汇率换算本位币金额的场景，确保 P0-1 修复后
    /// 自动生成的 AP 发票换算结果不会被缩小 100 倍。
    #[test]
    fn test_exchange_rate_conversion_not_shrunk_by_100() {
        let invoice_amount = Decimal::new(12345, 2); // 123.45
        let base_currency_amount = invoice_amount * DEFAULT_BASE_CURRENCY_EXCHANGE_RATE;

        // 修复前（汇率 0.01）：123.45 * 0.01 = 1.2345（被缩小 100 倍）
        assert_ne!(
            base_currency_amount,
            Decimal::new(12345, 4), // 1.2345（错误结果）
            "本位币金额被缩小 100 倍，P0-1 缺陷未修复"
        );

        // 修复后（汇率 1.0）：123.45 * 1.0 = 123.45（正确）
        assert_eq!(
            base_currency_amount,
            Decimal::new(12345, 2),
            "按汇率 1.0 换算后本位币金额应等于原金额"
        );
    }
}
