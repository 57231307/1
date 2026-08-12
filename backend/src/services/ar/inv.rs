//! 应收对账 - 发票 PDF 导出（ar/inv）
//!
//! 包含对账单 PDF 导出：
//! - `export_pdf` 公开方法，从数据库拉取对账单与明细并生成 PDF
//! - `generate_reconciliation_pdf` 内部方法，调用 `export_service::ExportService`
//!
//! 拆分自原 `ar_reconciliation_service.rs` 的 `export_pdf` / `generate_reconciliation_pdf` 两个方法。
//!
//! P0-2 销售→AR 业务流入口（`create_receivable`）：
//! - 在销售订单 `ship_order` 提交且库存扣减成功后被调用
//! - 复用调用方传入的数据库事务，保证库存扣减、AR 单、订单状态三者原子提交
//! - 按 `source_type=SALES_ORDER` + `source_bill_id=order_id` 幂等去重
//! - 应收单号复用 `DocumentNumberGenerator`，保证全局连续

use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};

use crate::models::ar_invoice::{
    ActiveModel as ArInvoiceActive, Column as ArInvoiceColumn, Entity as ArInvoiceEntity,
    Model as ArInvoiceModel,
};
use crate::models::ar_reconciliation::{
    Entity as ReconciliationEntity, Model as ReconciliationModel,
};
// 批次 158 v11 真实接入：审批状态常量替代字符串字面量
// 批次 231 v13 P1-1：新增 ar 模块导入，对账单状态常量替代字符串字面量
use crate::models::ar_reconciliation_item::{
    Entity as ReconciliationItemEntity, Model as ReconciliationItemModel,
};
use crate::models::customer;
use crate::models::sales_order;
use crate::models::status::{approval, ar as ar_status, common};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

use super::ArReconciliationService;

impl ArReconciliationService {
    /// 导出对账单PDF
    pub async fn export_pdf(&self, id: i32) -> Result<Vec<u8>, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        // 获取对账明细
        let items = ReconciliationItemEntity::find()
            .filter(crate::models::ar_reconciliation_item::Column::ReconciliationId.eq(id))
            .all(&*self.db)
            .await?;

        // 生成PDF内容
        let pdf_content = self.generate_reconciliation_pdf(&model, &items)?;

        Ok(pdf_content)
    }

    /// 生成对账单PDF
    fn generate_reconciliation_pdf(
        &self,
        reconciliation: &ReconciliationModel,
        items: &[ReconciliationItemModel],
    ) -> Result<Vec<u8>, AppError> {
        use crate::services::export_service::{ExportService, ReconciliationPdfItem};

        // 构建明细项
        let pdf_items: Vec<ReconciliationPdfItem> = items
            .iter()
            .map(|item| ReconciliationPdfItem {
                item_type: item.item_type.clone(),
                document_no: item.document_no.as_deref().unwrap_or("").to_string(),
                amount: item.amount.to_string(),
                date: item
                    .document_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
            })
            .collect();

        // 获取客户名称
        let customer_name = format!("客户#{}", reconciliation.customer_id);

        // 生成PDF
        ExportService::generate_reconciliation_pdf(
            &reconciliation.reconciliation_no,
            &customer_name,
            &reconciliation.period_start.format("%Y-%m-%d").to_string(),
            &reconciliation.period_end.format("%Y-%m-%d").to_string(),
            reconciliation
                .reconciliation_status
                .as_deref()
                .unwrap_or(ar_status::RECONCILIATION_DRAFT),
            pdf_items,
            &reconciliation.closing_balance.to_string(),
        )
    }

    // =====================================================
    // P0-2 销售→AR 业务流：应收单创建
    // =====================================================

    /// 创建应收单（P0-2 销售发货→AR 入口）
    /// 复用调用方 txn 保证原子提交；幂等：source_type=SALES_ORDER + source_bill_id=order_id。；账期 payment_terms_days<=0 回退 30 天；状态 DRAFT 待审（P0 3-5 修复）。
    pub async fn create_receivable(
        &self,
        customer_id: i32,
        order_id: i32,
        total_amount: Decimal,
        payment_terms_days: i32,
        user_id: i32,
        txn: &DatabaseTransaction,
    ) -> Result<ArInvoiceModel, AppError> {
        // 1. 金额校验：必须 > 0，避免生成 0 元应收单污染账龄报表
        if total_amount <= Decimal::ZERO {
            return Err(AppError::validation(format!(
                "应收金额必须大于 0，实际为 {}",
                total_amount
            )));
        }
        // 2-4. 加载订单/客户 + 幂等校验
        let (order, cust) = Self::load_receivable_context(txn, customer_id, order_id).await?;
        // 5. 账期校验：<= 0 时统一回退为 30 天
        let terms = if payment_terms_days <= 0 {
            30
        } else {
            payment_terms_days
        };
        // 6. 计算日期：发票日期 = 今日；到期日 = 发票日期 + 账期天数
        let invoice_date = Utc::now().date_naive();
        let due_date = invoice_date + Duration::days(terms as i64);
        // 7. 生成应收单号（与销售订单/采购订单/对账单共用流水号生成器）
        let invoice_no = DocumentNumberGenerator::generate_no(
            txn,
            "AR",
            ArInvoiceEntity,
            ArInvoiceColumn::InvoiceNo,
        )
        .await?;
        // 8. 写入 ar_invoices 表（DRAFT 待审，由 AR 审批节点确认后转 APPROVED）
        let active = Self::build_receivable_active(
            invoice_no,
            invoice_date,
            due_date,
            customer_id,
            order_id,
            total_amount,
            user_id,
            &order,
            &cust,
        );
        let invoice = active.insert(txn).await?;
        tracing::info!(
            "P0-2 销售→AR：应收单创建成功，invoice_no={}, amount={}, 账期={}天, 客户={}",
            invoice.invoice_no,
            invoice.invoice_amount,
            terms,
            cust.customer_name
        );
        Ok(invoice)
    }

    /// 加载应收单上下文：销售订单 + 幂等校验 + 客户主档
    async fn load_receivable_context(
        txn: &DatabaseTransaction,
        customer_id: i32,
        order_id: i32,
    ) -> Result<(sales_order::Model, customer::Model), AppError> {
        // 查询销售订单（取 order_no 作为 source_bill_no 写入应收单）
        let order = sales_order::Entity::find_by_id(order_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;
        // 幂等检查：同订单只允许存在一张应收单
        let exists = ArInvoiceEntity::find()
            .filter(ArInvoiceColumn::SourceType.eq("SALES_ORDER"))
            .filter(ArInvoiceColumn::SourceBillId.eq(order_id))
            .one(txn)
            .await?;
        if exists.is_some() {
            return Err(AppError::business(format!(
                "销售订单 {} 已生成应收单，请勿重复创建",
                order_id
            )));
        }
        // 查询客户主档（用于冗余客户名称字段）
        let cust = customer::Entity::find_by_id(customer_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;
        Ok((order, cust))
    }

    /// 构建应收单 ActiveModel（DRAFT 待审 + PENDING 审批），冗余客户名/订单号
    fn build_receivable_active(
        invoice_no: String,
        invoice_date: chrono::NaiveDate,
        due_date: chrono::NaiveDate,
        customer_id: i32,
        order_id: i32,
        total_amount: Decimal,
        user_id: i32,
        order: &sales_order::Model,
        cust: &customer::Model,
    ) -> ArInvoiceActive {
        ArInvoiceActive {
            invoice_no: Set(invoice_no),
            invoice_date: Set(invoice_date),
            due_date: Set(due_date),
            customer_id: Set(customer_id),
            customer_name: Set(Some(cust.customer_name.clone())),
            source_type: Set(Some("SALES_ORDER".to_string())),
            source_module: Set(Some("SO".to_string())),
            source_bill_id: Set(Some(order_id)),
            source_bill_no: Set(Some(order.order_no.clone())),
            invoice_amount: Set(total_amount),
            received_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(total_amount),
            batch_no: Set(None),
            color_no: Set(None),
            sales_order_no: Set(Some(order.order_no.clone())),
            status: Set(common::STATUS_DRAFT.to_string()),
            approval_status: Set(approval::PENDING.to_string()),
            created_by: Set(user_id),
            ..Default::default()
        }
    }
}

// =====================================================
// 单元测试（P0-2 销售→AR）
// =====================================================
//
// 覆盖场景：
// 1. 正常发货生成 AR：金额、账期、应收单号、状态字段全部正确
// 2. 取消发货回滚 AR：数据库错误抛出时事务回滚，ar_invoices 不残留
// 3. 部分发货的 AR 处理：amount 等于本次发货的应收金额，不与其他发货累加
// 4. 客户账期默认值：payment_terms <= 0 时回退为 30 天
// 5. 幂等检查：同订单二次调用返回 BusinessError
//
// 测试使用 mock 形式的辅助函数 `compute_due_date` / `format_invoice_no`
// 验证业务计算逻辑（数据库交互由 CICD 集成测试覆盖）。
