//! 采购入库/退货自动生成应付单 impl 子模块（ap_invoice_ops/receipt）
//!
//! 批次 490 D10-4b 拆分：从原 `ap_invoice_service.rs` L90-425 迁移。
//! 包含 ApInvoiceService 的 2 个公开自动生成方法 + 9 个私有 helper：
//! - auto_generate_from_receipt（从采购入库单自动生成应付单 + 同步生成应付凭证）
//! - auto_generate_from_return（从采购退货单自动生成红字应付单 + 同步生成红字冲销凭证）
//! - find_receipt_and_check_exists（查询入库单 + 结算门控 + 检查重复 + 校验供应商）
//! - build_and_insert_receipt_invoice（构造并插入应付单 ActiveModel）
//! - build_inventory_voucher_item / build_tax_voucher_item / build_payable_voucher_item（凭证分录构造）
//! - build_receipt_voucher_items（组装凭证分录列表）
//! - try_generate_receipt_voucher（commit 后生成应付凭证，失败仅 warn）
//! - build_return_voucher_items（组装退货红字凭证分录列表）
//! - try_generate_return_voucher（commit 后生成红字冲销凭证，按来源单据查重保证幂等）
//!
//! ReceiptVoucherContext 仅在本模块使用，随方法一同迁移至此（保持私有）。
//!
//! 价税取值：`ApTaxBasis` 是同一笔应付业务含税/不含税的唯一取值点，收货实收基数上的
//! 进项税额由 `resolve_receipt_input_tax` 逐行解析（税率取所回指采购订单行、金额派生
//! 复用采购订单行金额口径的权威实现），应付单账面额、应付凭证贷方与付款核销都从它取数。

use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::{
    ap_invoice, purchase_order_item, purchase_receipt, purchase_receipt_item, purchase_return,
    voucher,
};
use crate::services::po::CreateOrderItemRequest;
use crate::services::po::order::PurchaseOrderService;
use std::collections::HashMap;
use std::sync::Arc;

use crate::services::ap_invoice_service::{ApInvoiceService, DEFAULT_BASE_CURRENCY_EXCHANGE_RATE};
use crate::services::voucher_service::{CreateVoucherRequest, VoucherItemRequest, VoucherService};
use crate::utils::error::AppError;

/// 采购入库应付的价税取值点（本仓同一笔应付业务含税/不含税的唯一来源）
///
/// 不含税基与税额基都取本张收货单的实收量：`net` 为收货行金额合计，`tax` 为同一实收基数
/// 上按采购订单行税率解析的进项税额。应付单账面额（amount 与 unpaid_amount）、应付凭证
/// 2202 贷方、付款核销基准三处一律取 `gross`，凭证库存商品借方取 `net`，
/// 禁止任何一处再自行拼一次价税。
struct ApTaxBasis {
    /// 不含税额（收货行金额合计）
    net: Decimal,
    /// 进项税额（属应交税费—进项税额，不并入应付账款的成本段）
    tax: Decimal,
}

impl ApTaxBasis {
    /// 含税确认额（价税合计）：应付账款的确认额，也是付清时的核销基准
    fn gross(&self) -> Decimal {
        self.net + self.tax
    }
}

/// 采购应付/退货红字应付共用的凭证上下文：commit 前从应付单捕获生成凭证所需字段，
/// invoice 在 Ok 返回时被 move，提前捕获避免 commit 后凭证生成无法访问。
/// 金额语义随单据方向：收货单为正数、退货单为红字负数，两者 amount 均存含税价税合计，
/// tax_amount 为其中进项税额（退货为负），库存商品借方由 amount 扣税推定。
struct ReceiptVoucherContext {
    invoice_no: String,
    invoice_id: i32,
    supplier_id: i32,
    amount: Decimal,
    tax_amount: Decimal,
    invoice_date: NaiveDate,
}

/// 应付单的来源单据类型：采购入库（ap_invoice.source_type 取值）
const AP_SOURCE_TYPE_PURCHASE_RECEIPT: &str = "PURCHASE_RECEIPT";

/// 应付单的来源单据类型：采购退货（ap_invoice.source_type 取值）
const AP_SOURCE_TYPE_PURCHASE_RETURN: &str = "PURCHASE_RETURN";

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

    /// 凭证库存商品借方取不含税额：账面额（含税）扣掉归属进项税额的那一段，
    /// 与应付单落库时的价税取值同式，不在分录侧另算
    fn net_amount(&self) -> Decimal {
        self.amount - self.tax_amount
    }
}

impl ApInvoiceService {
    /// 入库单是否已生成应付单（source_type=采购入库 且 source_id=入库单 ID）
    /// 供收货完成事件在补偿生成前判定，避免对正常已生成的单据反复报"补偿失败"
    pub async fn exists_for_receipt(&self, receipt_id: i32) -> Result<bool, AppError> {
        let existing = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq(AP_SOURCE_TYPE_PURCHASE_RECEIPT))
            .filter(ap_invoice::Column::SourceId.eq(receipt_id))
            .one(&*self.db)
            .await?;
        Ok(existing.is_some())
    }

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

    /// 查询采购入库单 + 结算门控（质检合格方可结算） + 检查重复生成 + 校验供应商存在
    async fn find_receipt_and_check_exists(
        txn: &DatabaseTransaction,
        receipt_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        // 1. 查询采购入库单
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 1b. 结算门控与确认入库同一口径（词表语义：PASSED=允许后续入库/**结算**流转）：
        // 本入口可被 HTTP auto_generate 端点直连调用、不经 confirm，未质检合格/不合格的
        // 收货单同样不得生成应付；判定复用 PurchaseReceiptService 的唯一实现，不另写第二套。
        // 必须先于重复生成检查与任何应付写入——被拒时事务未落任何行（契约测试锁定顺序）。
        crate::services::purchase_receipt_service::PurchaseReceiptService::ensure_receipt_inspection_allows_flow(
            &receipt,
            "生成应付结算",
        )?;

        // 2. 检查是否已生成应付
        let exists = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq(AP_SOURCE_TYPE_PURCHASE_RECEIPT))
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
    ///
    /// 入参 receipt 的 total_amount 是收货行金额合计（不含税基），税额按同一实收基数逐行
    /// 解析；落库 amount 与 unpaid_amount 存价税合计（含税应付义务），tax_amount 存其中
    /// 归属「应交税费—进项税额」的那一段。应付凭证 2202 贷方与付款核销都从本账面额取数。
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

        let receipt_items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt.id))
            .all(txn)
            .await?;

        // 5. 价税取值：不含税基与税额基同落在本张收货单的实收量上，含税确认额由两者相加
        let tax_basis = ApTaxBasis {
            net: receipt.total_amount,
            tax: Self::resolve_receipt_input_tax(txn, &receipt_items).await?,
        };
        let payable_amount = tax_basis.gross();
        tracing::info!(
            receipt_id = receipt.id,
            invoice_no = %invoice_no,
            net = %tax_basis.net,
            input_tax = %tax_basis.tax,
            payable = %payable_amount,
            "应付单价税取值：按实收基数解析进项税额，账面额取价税合计"
        );

        let invoice = ap_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            supplier_id: Set(receipt.supplier_id),
            invoice_type: Set("PURCHASE".to_string()),
            source_type: Set(Some(AP_SOURCE_TYPE_PURCHASE_RECEIPT.to_string())),
            source_id: Set(Some(receipt.id)),
            invoice_date: Set(invoice_date),
            due_date: Set(due_date),
            payment_terms: Set(payment_terms),
            amount: Set(payable_amount),
            paid_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(payable_amount),
            // P0 3-1 修复（2026-07-01 八维度审计）：自动生成 AP 发票初始为 DRAFT，
            // 经 approve 流程审核后转 AUDITED 才能进入付款环节。
            // 原 PENDING 状态不在 approve 状态机枚举内（仅 DRAFT→AUDITED），
            // 导致自动生成的应付单死锁在 PENDING 永远无法审批。
            invoice_status: Set(crate::models::status::common::STATUS_DRAFT.to_string()),
            currency: Set(crate::constants::DEFAULT_CURRENCY.to_string()),
            exchange_rate: Set(DEFAULT_BASE_CURRENCY_EXCHANGE_RATE),
            tax_amount: Set(tax_basis.tax),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(txn)
        .await?;

        Ok(invoice)
    }

    /// 汇总本张收货单在实收基数上的进项税额（传给应付单 tax_amount 与应付凭证借方行）
    ///
    /// 逐行以「本行实收量 × 本行实录单价」为计税基，税率与折扣率取所回指采购订单行的值，
    /// 派生金额复用采购订单行金额口径的唯一权威实现
    /// `PurchaseOrderService::calculate_item_amounts`，本处不另写第二套公式。
    /// 未回指订单行的收货行（无来源订单的入库）无从解析税率，按不计税入账并显式告警，
    /// 与不含税基保持同基，不臆造税率、不静默丢行；回指行查不到则拒绝生成，避免少计税款。
    async fn resolve_receipt_input_tax(
        txn: &DatabaseTransaction,
        items: &[purchase_receipt_item::Model],
    ) -> Result<Decimal, AppError> {
        let order_item_ids: Vec<i32> = items.iter().filter_map(|item| item.order_item_id).collect();
        let order_item_map = Self::load_source_order_items(txn, order_item_ids).await?;

        let mut tax_total = Decimal::ZERO;
        for item in items {
            let Some(order_item_id) = item.order_item_id else {
                tracing::warn!(
                    receipt_item_id = item.id,
                    receipt_id = item.receipt_id,
                    "收货行未回指采购订单行，税率无从解析，本行按不计税计入应付单进项税额"
                );
                continue;
            };
            let order_item = order_item_map.get(&order_item_id).ok_or_else(|| {
                AppError::business(format!("收货行 {} 回指的采购订单行不存在", item.id))
            })?;
            if item.unit_price.is_none() {
                tracing::warn!(
                    receipt_item_id = item.id,
                    order_item_id,
                    "收货行未记录单价，计税基与不含税基同为 0，本行不计进项税"
                );
            }
            // 只取权威实现的税额列：不含税基已由收货主表合计给出，折扣不参与本处取值
            let amounts = PurchaseOrderService::calculate_item_amounts(&CreateOrderItemRequest {
                line_no: Some(item.line_no),
                material_id: Some(item.product_id),
                unit_price: item.unit_price,
                quantity_ordered: Some(item.quantity),
                quantity_alt_ordered: item.quantity_alt,
                tax_rate: Some(order_item.tax_percent),
                discount_percent: Some(order_item.discount_percent),
                quantity_tolerance_pct: order_item.quantity_tolerance_pct,
                color_no: item.color_code.clone(),
                notes: item.notes.clone(),
            });
            tax_total += amounts.tax_amount;
        }

        Ok(tax_total)
    }

    /// 批量加载收货行回指的采购订单行（循环外单次查询，避免逐行回查的 N+1）
    async fn load_source_order_items(
        txn: &DatabaseTransaction,
        order_item_ids: Vec<i32>,
    ) -> Result<HashMap<i32, purchase_order_item::Model>, AppError> {
        if order_item_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::Id.is_in(order_item_ids))
            .all(txn)
            .await?;
        Ok(rows.into_iter().map(|row| (row.id, row)).collect())
    }

    /// 库存商品分录（借方科目码 1405；收货传正数不含税额，退货传红字负数）
    fn build_inventory_voucher_item(
        summary: &str,
        supplier_id: i32,
        amount: Decimal,
    ) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: Some(1),
            subject_id: None,
            subject_code: Some("1405".to_string()),
            subject_name: Some("库存商品".to_string()),
            debit: amount,
            credit: Decimal::ZERO,
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

    /// 进项税额分录（借方科目码 222102，仅税额非零时插入；收货传正数，退货传红字负数）。
    /// 销项税额走 222101（见销售发货开票链路），两码分挂同一父科目 2221：
    /// 同用一枚科目会让采购进项与销售销项在明细账里同码互抵，进税额被销项吃掉。
    fn build_tax_voucher_item(
        summary: &str,
        supplier_id: i32,
        tax_amount: Decimal,
    ) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: Some(2),
            subject_id: None,
            subject_code: Some("222102".to_string()),
            subject_name: Some("应交税费-应交增值税-进项税额".to_string()),
            debit: tax_amount,
            credit: Decimal::ZERO,
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

    /// 应付账款分录（贷方，挂供应商辅助核算；收货传含税正数，退货传含税红字负数）
    fn build_payable_voucher_item(
        summary: &str,
        supplier_id: i32,
        payable_amount: Decimal,
    ) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: Some(2),
            subject_id: None,
            subject_code: Some("2202".to_string()),
            subject_name: Some("应付账款".to_string()),
            debit: Decimal::ZERO,
            credit: payable_amount,
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

    /// 组装采购入库应付确认凭证分录（默认 2 行：1=库存商品 / 2=应付账款，tax_amount > 0 时
    /// 在中间插入 222101 进项税额并重排行号为 1=库存商品 / 2=进项税额 / 3=应付账款）
    ///
    /// 贷方应付账款直接取应付单账面额（含税），库存商品借方取账面额扣掉税额后的不含段，
    /// 使借方两行之和恒等于贷方一行，与付款核销基准同源
    fn build_receipt_voucher_items(ctx: &ReceiptVoucherContext) -> Vec<VoucherItemRequest> {
        let summary = format!("采购入库应付确认-{}", ctx.invoice_no);
        let mut voucher_items = vec![
            Self::build_inventory_voucher_item(&summary, ctx.supplier_id, ctx.net_amount()),
            Self::build_payable_voucher_item(&summary, ctx.supplier_id, ctx.amount),
        ];

        // 进项税额仅在 tax_amount > 0 时插入，避免零额凭证分录引起校验告警
        if ctx.tax_amount > Decimal::ZERO {
            voucher_items.insert(
                1,
                Self::build_tax_voucher_item(&summary, ctx.supplier_id, ctx.tax_amount),
            );
            // 重排行号：1=库存商品 / 2=进项税额 / 3=应付账款
            voucher_items[2].line_no = Some(3);
        }

        voucher_items
    }

    /// 组装采购退货红字冲销凭证分录列表，行结构与收货凭证镜像（负数红字）
    ///
    /// 口径来源：红字应付单落库时 amount 已存含税负总额、tax_amount 存负税额（见
    /// auto_generate_from_return），故不含税红字段按 amount - tax_amount 推导；
    /// 本方法只按应付单存值红冲，不改动账面额自身的含税/不含税判定口径。
    /// 借贷自平：(amount - tax) + tax = amount。
    fn build_return_voucher_items(ctx: &ReceiptVoucherContext) -> Vec<VoucherItemRequest> {
        let net_amount = ctx.amount - ctx.tax_amount;
        let summary = format!("采购退货红字冲销-{}", ctx.invoice_no);
        let mut voucher_items = vec![
            Self::build_inventory_voucher_item(&summary, ctx.supplier_id, net_amount),
            Self::build_payable_voucher_item(&summary, ctx.supplier_id, ctx.amount),
        ];

        // 税额为零时不插行，避免零额凭证分录引起校验告警（与收货侧同一处理）
        if ctx.tax_amount != Decimal::ZERO {
            voucher_items.insert(
                1,
                Self::build_tax_voucher_item(&summary, ctx.supplier_id, ctx.tax_amount),
            );
            // 重排行号：1=库存商品 / 2=进项税额 / 3=应付账款
            voucher_items[2].line_no = Some(3);
        }

        voucher_items
    }

    /// 采购入库应付凭证生成：借 1405 库存商品（不含税金额）、借 222102 应交税费-进项税额
    /// （仅 tax_amount 大于 0 时插入）、贷 2202 应付账款（含税总额，挂供应商辅助核算）。
    /// 调用方为采购入库确认链路；入参 ctx 携带金额、税额与供应商；产出发票凭证并回写
    /// 应付单的 voucher_id，落 account_voucher 与 voucher_items。
    /// 生成失败只 warn 不阻断主流程（与 confirm_receipt 的容错口径一致），避免因凭证
    /// 侧异常回滚已完成的收货业务，缺失凭证由留痕日志指向人工补偿。
    async fn try_generate_receipt_voucher(
        db: &Arc<DatabaseConnection>,
        ctx: ReceiptVoucherContext,
        user_id: i32,
    ) {
        // 应付单账面额（含税）就是本凭证应付账款贷方额：为零说明收货未计出任何应付义务，
        // 货已入库却无凭证属账实脱节，跳过必须显式告警、不静默
        if ctx.amount <= Decimal::ZERO {
            tracing::warn!(
                invoice_id = ctx.invoice_id,
                invoice_no = %ctx.invoice_no,
                "应付单账面含税额为 0，跳过应付凭证生成：请核对收货行单价是否维护"
            );
            return;
        }

        let voucher_items = Self::build_receipt_voucher_items(&ctx);
        let voucher_req = CreateVoucherRequest {
            voucher_type: "转".to_string(),
            voucher_date: ctx.invoice_date,
            source_type: Some(AP_SOURCE_TYPE_PURCHASE_RECEIPT.to_string()),
            source_module: Some("purchase".to_string()),
            source_bill_id: Some(i64::from(ctx.invoice_id)),
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

    /// 生成采购退货红字冲销凭证（收货应付凭证的镜像负数分录，唯一承载退货记账）
    ///
    /// 幂等：按 source_type + 应付单 ID 查重，已存在即跳过，重复调用不会二次入账；
    /// 失败：显式 error 日志并以 Err 向上传播（凭证已无可回滚上下文，交由调用方告警，
    /// 需人工补生成时凭日志中的应付单号定位）。红字单零金额（amount 与税额均为 0）不入账。
    async fn try_generate_return_voucher(
        db: &Arc<DatabaseConnection>,
        ctx: ReceiptVoucherContext,
        user_id: i32,
    ) -> Result<(), AppError> {
        if ctx.amount.is_zero() && ctx.tax_amount.is_zero() {
            tracing::info!(
                "退货应付单 {} 金额与税额均为 0，无需生成红字冲销凭证",
                ctx.invoice_no
            );
            return Ok(());
        }

        let existing = voucher::Entity::find()
            .filter(voucher::Column::SourceType.eq(AP_SOURCE_TYPE_PURCHASE_RETURN))
            .filter(voucher::Column::SourceBillId.eq(i64::from(ctx.invoice_id)))
            .limit(1)
            .all(&**db)
            .await?;
        if !existing.is_empty() {
            tracing::info!(
                "退货应付单 {} 的红字冲销凭证已存在，幂等跳过重复生成",
                ctx.invoice_no
            );
            return Ok(());
        }

        let voucher_items = Self::build_return_voucher_items(&ctx);
        let voucher_req = CreateVoucherRequest {
            voucher_type: "转".to_string(),
            voucher_date: ctx.invoice_date,
            source_type: Some(AP_SOURCE_TYPE_PURCHASE_RETURN.to_string()),
            source_module: Some("purchase".to_string()),
            source_bill_id: Some(i64::from(ctx.invoice_id)),
            source_bill_no: Some(ctx.invoice_no.clone()),
            batch_no: None,
            color_no: None,
            items: voucher_items,
        };
        let voucher_service = VoucherService::new(db.clone());
        match voucher_service.create_and_post(voucher_req, user_id).await {
            Ok(posted) => {
                tracing::info!(
                    "退货应付单 {} 红字冲销凭证生成成功：凭证号 {}",
                    ctx.invoice_no,
                    posted.voucher_no
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "⚠ 退货应付单 {} 红字冲销凭证生成失败，需人工补生成：{}",
                    ctx.invoice_no,
                    e
                );
                Err(e)
            }
        }
    }

    /// 从采购退货单自动生成应付单（红字）并同步生成红字冲销凭证
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
            .filter(ap_invoice::Column::SourceType.eq(AP_SOURCE_TYPE_PURCHASE_RETURN))
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
            source_type: Set(Some(AP_SOURCE_TYPE_PURCHASE_RETURN.to_string())),
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

        // commit 前捕获红字凭证所需字段（invoice 在 Ok 返回时被 move）
        let voucher_ctx = ReceiptVoucherContext::from_invoice(&invoice);

        txn.commit().await?;

        // 凭证生成在应付单事务 commit 之后：create_and_post 自带独立事务，
        // 此处已无可回滚的上下文，失败以 Err 返回交调用方显式告警（不静默），
        // 幂等由 try_generate_return_voucher 内的来源单据查重保证。
        Self::try_generate_return_voucher(&self.db, voucher_ctx, user_id).await?;

        Ok(invoice)
    }
}
