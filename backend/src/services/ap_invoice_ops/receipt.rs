//! 采购入库/退货自动生成应付单 impl 子模块（ap_invoice_ops/receipt）
//!
//! 批次 490 D10-4b 拆分：从原 `ap_invoice_service.rs` L90-425 迁移。
//! 包含 ApInvoiceService 的 2 个公开自动生成方法 + 7 个私有 helper：
//! - auto_generate_from_receipt（从采购入库单自动生成应付单 + 同步生成应付凭证）
//! - auto_generate_from_return（从采购退货单自动生成红字应付单）
//! - find_receipt_and_check_exists（查询入库单 + 检查重复 + 校验供应商）
//! - build_and_insert_receipt_invoice（构造并插入应付单 ActiveModel）
//! - build_inventory_voucher_item / build_tax_voucher_item / build_payable_voucher_item（凭证分录构造）
//! - build_receipt_voucher_items（组装凭证分录列表）
//! - try_generate_receipt_voucher（commit 后生成应付凭证，失败仅 warn）
//!
//! ReceiptVoucherContext 仅在本模块使用，随方法一同迁移至此（保持私有）。

use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, Set, TransactionTrait,
};
use std::sync::Arc;

use crate::models::{ap_invoice, purchase_receipt, purchase_return};
use crate::utils::error::AppError;
use crate::services::ap_invoice_service::{ApInvoiceService, DEFAULT_BASE_CURRENCY_EXCHANGE_RATE};
use crate::services::voucher_service::{CreateVoucherRequest, VoucherItemRequest, VoucherService};

/// 采购入库应付单凭证上下文（D08-1 第二梯队拆分辅助结构）
///
/// F-P0-7 修复（批次 382 v13 复审）：commit 前保存生成应付凭证所需字段，
/// invoice 在 Ok 返回时被 move，提前捕获避免后续 voucher 生成无法访问。
struct ReceiptVoucherContext {
    invoice_no: String,
    invoice_id: i32,
    supplier_id: i32,
    amount: Decimal,
    tax_amount: Decimal,
    invoice_date: NaiveDate,
}

impl ReceiptVoucherContext {
    fn from_invoice(invoice: &ap_invoice::Model) -> Self {
        Self {
            invoice_no: invoice.invoice_no.clone(),
            invoice_id: invoice.id,
            supplier_id: invoice.supplier_id,
            amount: invoice.amount,
            tax_amount: invoice.tax_amount,
            invoice_date: invoice.invoice_date,
        }
    }
}

impl ApInvoiceService {
    /// 从采购入库单自动生成应付单
    pub async fn auto_generate_from_receipt(
        &self,
        receipt_id: i32,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询入库单 + 检查是否已生成应付 + 校验供应商
        let receipt = Self::find_receipt_and_check_exists(&txn, receipt_id).await?;

        // 2. 生成应付单
        let invoice_no = self.generate_invoice_no().await?;
        let invoice =
            Self::build_and_insert_receipt_invoice(&txn, &receipt, invoice_no, user_id).await?;

        // F-P0-7 修复（批次 382 v13 复审）：commit 前保存生成应付凭证所需字段
        // invoice 在 Ok 返回时被 move，提前捕获避免后续 voucher 生成无法访问
        let voucher_ctx = ReceiptVoucherContext::from_invoice(&invoice);

        txn.commit().await?;

        // F-P0-7 修复：commit 后生成应付凭证（失败仅 warn 不阻断主流程）
        Self::try_generate_receipt_voucher(&self.db, voucher_ctx, user_id).await;

        Ok(invoice)
    }

    /// 查询采购入库单 + 检查重复生成 + 校验供应商存在
    async fn find_receipt_and_check_exists(
        txn: &DatabaseTransaction,
        receipt_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        // 1. 查询采购入库单
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查是否已生成应付
        let exists = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RECEIPT"))
            .filter(ap_invoice::Column::SourceId.eq(receipt_id))
            .one(txn)
            .await?;

        if exists.is_some() {
            return Err(AppError::business("该入库单已生成应付单"));
        }

        // 3. 获取供应商信息
        let _supplier = crate::models::supplier::Entity::find_by_id(receipt.supplier_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商 {}", receipt.supplier_id)))?;

        Ok(receipt)
    }

    /// 构造并插入应付单 ActiveModel（账期 30 天 / 初始 DRAFT）
    async fn build_and_insert_receipt_invoice(
        txn: &DatabaseTransaction,
        receipt: &purchase_receipt::Model,
        invoice_no: String,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        // 使用默认账期 30 天
        let payment_terms = crate::constants::DEFAULT_PAYMENT_TERMS_DAYS;

        // 4. 生成应付单
        let invoice_date = receipt.receipt_date;
        let due_date = invoice_date + Duration::days(payment_terms as i64);

        let invoice = ap_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            supplier_id: Set(receipt.supplier_id),
            invoice_type: Set("PURCHASE".to_string()),
            source_type: Set(Some("PURCHASE_RECEIPT".to_string())),
            source_id: Set(Some(receipt.id)),
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
        .insert(txn)
        .await?;

        Ok(invoice)
    }

    /// 采购入库应付确认 - 库存商品分录（借：1405，不含税金额）
    fn build_inventory_voucher_item(
        invoice_no: &str,
        supplier_id: i32,
        amount: Decimal,
    ) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: Some(1),
            subject_code: Some("1405".to_string()),
            subject_name: Some("库存商品".to_string()),
            debit: amount,
            credit: Decimal::ZERO,
            summary: Some(format!("采购入库应付确认-{}", invoice_no)),
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

    /// 采购入库应付确认 - 进项税额分录（借：222101，仅 tax_amount > 0 时插入）
    fn build_tax_voucher_item(
        invoice_no: &str,
        supplier_id: i32,
        tax_amount: Decimal,
    ) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: Some(2),
            subject_code: Some("222101".to_string()),
            subject_name: Some("应交税费-应交增值税-进项税额".to_string()),
            debit: tax_amount,
            credit: Decimal::ZERO,
            summary: Some(format!("采购入库应付确认-{}", invoice_no)),
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

    /// 采购入库应付确认 - 应付账款分录（贷：2202，含税总额，挂供应商辅助核算）
    fn build_payable_voucher_item(
        invoice_no: &str,
        supplier_id: i32,
        total: Decimal,
    ) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: Some(2),
            subject_code: Some("2202".to_string()),
            subject_name: Some("应付账款".to_string()),
            debit: Decimal::ZERO,
            credit: total,
            summary: Some(format!("采购入库应付确认-{}", invoice_no)),
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

    /// 组装采购入库应付确认凭证分录列表
    ///
    /// 默认 2 行（库存商品 / 应付账款），tax_amount > 0 时插入进项税额，
    /// 重排为 3 行：1=库存商品 / 2=进项税额 / 3=应付账款。
    fn build_receipt_voucher_items(ctx: &ReceiptVoucherContext) -> Vec<VoucherItemRequest> {
        let voucher_total = ctx.amount + ctx.tax_amount;
        let mut voucher_items = vec![
            Self::build_inventory_voucher_item(&ctx.invoice_no, ctx.supplier_id, ctx.amount),
            Self::build_payable_voucher_item(&ctx.invoice_no, ctx.supplier_id, voucher_total),
        ];

        // 进项税额仅在 tax_amount > 0 时插入，避免零额凭证分录引起校验告警
        if ctx.tax_amount > Decimal::ZERO {
            voucher_items.insert(
                1,
                Self::build_tax_voucher_item(&ctx.invoice_no, ctx.supplier_id, ctx.tax_amount),
            );
            // 重排行号：1=库存商品 / 2=进项税额 / 3=应付账款
            voucher_items[2].line_no = Some(3);
        }

        voucher_items
    }

    /// 采购入库生成应付单后同步生成应付凭证（失败仅 warn 不阻断主流程）
    ///
    /// F-P0-7 修复（批次 382 v13 复审）：采购入库生成应付单后同步生成应付凭证
    /// 借：1405 库存商品（不含税金额）
    /// 借：222101 应交税费-进项税额（tax_amount > 0 时）
    /// 贷：2202 应付账款（含税总额，挂供应商辅助核算）
    /// 失败时仅 warn 不阻断主流程（与采购入库 confirm_receipt 容错模式一致），
    /// 避免凭证生成失败影响主业务流程，便于人工补偿。
    async fn try_generate_receipt_voucher(
        db: &Arc<DatabaseConnection>,
        ctx: ReceiptVoucherContext,
        user_id: i32,
    ) {
        let voucher_total = ctx.amount + ctx.tax_amount;
        if voucher_total <= Decimal::ZERO {
            return;
        }

        let voucher_items = Self::build_receipt_voucher_items(&ctx);
        let voucher_req = CreateVoucherRequest {
            voucher_type: "转".to_string(),
            voucher_date: ctx.invoice_date,
            source_type: Some("PURCHASE_RECEIPT".to_string()),
            source_module: Some("purchase".to_string()),
            source_bill_id: Some(ctx.invoice_id),
            source_bill_no: Some(ctx.invoice_no.clone()),
            batch_no: None,
            color_no: None,
            items: voucher_items,
        };
        let voucher_service = VoucherService::new(db.clone());
        if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
            tracing::warn!(
                "应付单 {} 应付凭证生成失败，需人工补生成：{}",
                ctx.invoice_no,
                e
            );
        }
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
}
