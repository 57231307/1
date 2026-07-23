//! 供应商对账 CRUD + 查询 impl 子模块（ap_reconciliation_ops/crud）
//!
//! D10-5 拆分：从原 `ap_reconciliation_service.rs` 迁移。
//! 包含 ApReconciliationService 的 4 个方法：
//! - generate_reconciliation（生成对账单，计算期初/本期/期末余额）
//! - get_by_id / get_list（对账单查询 + 分页筛选）
//! - get_invoice_relations（发票关联入库单/付款记录）

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

use crate::models::{ap_invoice, ap_payment, ap_reconciliation};
use crate::models::status::ap_reconciliation as reconciliation_status;
use crate::models::status::payment;
use crate::services::ap_reconciliation_ops::types::{GenerateReconciliationRequest, InvoiceRelationInfo};
use crate::services::ap_reconciliation_service::ApReconciliationService;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

impl ApReconciliationService {
    /// 生成供应商对账单
    pub async fn generate_reconciliation(
        &self,
        req: GenerateReconciliationRequest,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成对账单号
        let reconciliation_no = self.generate_reconciliation_no().await?;

        // 2. 查询该供应商在对账期间内的应付单
        let invoices = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.gte(req.start_date))
            .filter(ap_invoice::Column::InvoiceDate.lte(req.end_date))
            .all(&txn)
            .await?;

        // 3. 查询该供应商在对账期间内的付款单
        let payments = ap_payment::Entity::find()
            .filter(ap_payment::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_payment::Column::PaymentStatus.eq(payment::PAYMENT_CONFIRMED))
            .filter(ap_payment::Column::PaymentDate.gte(req.start_date))
            .filter(ap_payment::Column::PaymentDate.lte(req.end_date))
            .all(&txn)
            .await?;

        // 4. 计算期初余额（对账开始日期前的未付金额）
        let opening_balance = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.lt(req.start_date))
            .all(&txn)
            .await?
            .iter()
            .map(|inv| inv.unpaid_amount)
            .sum::<Decimal>();

        // 5. 计算本期应付合计
        let total_invoice: Decimal = invoices.iter().map(|inv| inv.amount).sum();

        // 6. 计算本期付款合计
        let total_payment: Decimal = payments.iter().map(|pay| pay.payment_amount).sum();

        // 7. 计算期末余额
        let closing_balance = opening_balance + total_invoice - total_payment;

        // 8. 创建对账单
        let reconciliation = ap_reconciliation::ActiveModel {
            reconciliation_no: Set(reconciliation_no),
            supplier_id: Set(req.supplier_id),
            start_date: Set(req.start_date),
            end_date: Set(req.end_date),
            opening_balance: Set(opening_balance),
            total_invoice: Set(total_invoice),
            total_payment: Set(total_payment),
            closing_balance: Set(closing_balance),
            reconciliation_status: Set(reconciliation_status::PENDING.to_string()),
            notes: Set(req.notes),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(reconciliation)
    }

    /// 获取对账单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_reconciliation::Model, AppError> {
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("对账单 {}", id)))?;

        Ok(reconciliation)
    }

    /// 获取对账单列表
    pub async fn get_list(
        &self,
        supplier_id: Option<i32>,
        reconciliation_status: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ap_reconciliation::Model>, u64), AppError> {
        let mut query = ap_reconciliation::Entity::find();

        // 筛选条件
        if let Some(sid) = supplier_id {
            query = query.filter(ap_reconciliation::Column::SupplierId.eq(sid));
        }
        if let Some(status) = reconciliation_status {
            query = query.filter(ap_reconciliation::Column::ReconciliationStatus.eq(status));
        }
        if let Some(sd) = start_date {
            query = query.filter(ap_reconciliation::Column::StartDate.gte(sd));
        }
        if let Some(ed) = end_date {
            query = query.filter(ap_reconciliation::Column::EndDate.lte(ed));
        }

        // 批次 259 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(ap_reconciliation::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// 获取发票关联信息
    pub async fn get_invoice_relations(
        &self,
        invoice_id: i32,
    ) -> Result<Vec<InvoiceRelationInfo>, AppError> {
        let invoice = ap_invoice::Entity::find_by_id(invoice_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付单 {}", invoice_id)))?;

        let mut relations = Vec::new();

        // 关联采购入库单
        if invoice.source_type.as_deref() == Some("PURCHASE_RECEIPT") {
            relations.push(InvoiceRelationInfo {
                invoice_id: invoice.id,
                invoice_no: invoice.invoice_no.clone(),
                source_type: invoice.source_type.clone().unwrap_or_default(),
                source_id: invoice.source_id.unwrap_or_default(),
                source_no: None,
                supplier_id: invoice.supplier_id,
                amount: invoice.amount,
                status: invoice.invoice_status.clone(),
            });
        }

        // 关联付款记录
        let payments = ap_payment::Entity::find()
            .filter(ap_payment::Column::SupplierId.eq(invoice.supplier_id))
            .all(&*self.db)
            .await?;

        for payment in payments {
            relations.push(InvoiceRelationInfo {
                invoice_id: invoice.id,
                invoice_no: invoice.invoice_no.clone(),
                source_type: "PAYMENT".to_string(),
                source_id: payment.id,
                source_no: Some(payment.payment_no.clone()),
                supplier_id: invoice.supplier_id,
                amount: payment.payment_amount,
                status: payment.payment_status.clone(),
            });
        }

        Ok(relations)
    }
}
