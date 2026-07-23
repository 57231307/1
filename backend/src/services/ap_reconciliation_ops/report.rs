//! 供应商应付汇总 impl 子模块（ap_reconciliation_ops/report）
//!
//! D10-5 拆分：从原 `ap_reconciliation_service.rs` 迁移。
//! 包含 ApReconciliationService 的 4 个方法（1 公开 + 3 静态辅助）：
//! - get_supplier_summary（获取供应商应付汇总，按供应商分组统计金额/付款状态/逾期）
//! - query_invoices_for_summary（查询应付发票）
//! - aggregate_invoices_by_supplier（按供应商分组统计发票）
//! - enrich_supplier_names（查询供应商信息并填充 code/name）

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::{ap_invoice, supplier};
use crate::services::ap_reconciliation_ops::types::SupplierApSummary;
use crate::services::ap_reconciliation_service::ApReconciliationService;
use crate::utils::error::AppError;

impl ApReconciliationService {
    /// 获取供应商应付汇总（从物化视图）
    pub async fn get_supplier_summary(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<Vec<SupplierApSummary>, AppError> {
        let invoices = Self::query_invoices_for_summary(&*self.db, supplier_id).await?;
        let mut summary_map = Self::aggregate_invoices_by_supplier(&invoices);
        Self::enrich_supplier_names(&*self.db, &mut summary_map).await?;
        Ok(summary_map.into_values().collect())
    }

    /// 查询应付发票（按 supplier_id 可选过滤）
    async fn query_invoices_for_summary(
        db: &sea_orm::DatabaseConnection,
        supplier_id: Option<i32>,
    ) -> Result<Vec<ap_invoice::Model>, AppError> {
        let mut invoice_query = ap_invoice::Entity::find();
        if let Some(sid) = supplier_id {
            invoice_query = invoice_query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }
        invoice_query.all(db).await.map_err(AppError::from)
    }

    /// 按供应商分组统计发票（金额/付款状态/逾期）
    fn aggregate_invoices_by_supplier(
        invoices: &[ap_invoice::Model],
    ) -> std::collections::HashMap<i32, SupplierApSummary> {
        let mut summary_map: std::collections::HashMap<i32, SupplierApSummary> =
            std::collections::HashMap::new();
        for invoice in invoices {
            let entry = summary_map
                .entry(invoice.supplier_id)
                .or_insert_with(|| SupplierApSummary {
                    supplier_id: invoice.supplier_id,
                    supplier_code: String::new(),
                    supplier_name: String::new(),
                    total_invoice_count: 0,
                    total_invoice_amount: Decimal::ZERO,
                    total_paid_amount: Decimal::ZERO,
                    total_unpaid_amount: Decimal::ZERO,
                    paid_invoice_count: 0,
                    partial_paid_invoice_count: 0,
                    overdue_invoice_count: 0,
                    overdue_amount: Decimal::ZERO,
                });

            entry.total_invoice_count += 1;
            entry.total_invoice_amount += invoice.amount;
            entry.total_paid_amount += invoice.paid_amount;
            entry.total_unpaid_amount += invoice.unpaid_amount;

            // 判断付款状态
            if (invoice.amount > Decimal::ZERO && invoice.paid_amount >= invoice.amount)
                || (invoice.amount < Decimal::ZERO && invoice.paid_amount <= invoice.amount)
            {
                entry.paid_invoice_count += 1;
            } else if invoice.paid_amount != Decimal::ZERO {
                entry.partial_paid_invoice_count += 1;
            }

            // 判断是否逾期
            if invoice.due_date < chrono::Utc::now().date_naive()
                && invoice.unpaid_amount > Decimal::ZERO
            {
                entry.overdue_invoice_count += 1;
                entry.overdue_amount += invoice.unpaid_amount;
            }
        }
        summary_map
    }

    /// 查询供应商信息并填充到汇总 map（code/name）
    async fn enrich_supplier_names(
        db: &sea_orm::DatabaseConnection,
        summary_map: &mut std::collections::HashMap<i32, SupplierApSummary>,
    ) -> Result<(), AppError> {
        let supplier_ids: Vec<i32> = summary_map.keys().cloned().collect();
        if supplier_ids.is_empty() {
            return Ok(());
        }
        let suppliers = supplier::Entity::find()
            .filter(supplier::Column::Id.is_in(supplier_ids))
            .all(db)
            .await?;
        for s in suppliers {
            if let Some(entry) = summary_map.get_mut(&s.id) {
                entry.supplier_code = s.supplier_code;
                entry.supplier_name = s.supplier_name;
            }
        }
        Ok(())
    }
}
