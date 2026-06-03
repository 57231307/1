//! 应收对账 - 发票 PDF 导出（ar/inv）
//!
//! 包含对账单 PDF 导出：
//! - `export_pdf` 公开方法，从数据库拉取对账单与明细并生成 PDF
//! - `generate_reconciliation_pdf` 内部方法，调用 `export_service::ExportService`
//!
//! 拆分自原 `ar_reconciliation_service.rs` 的 `export_pdf` / `generate_reconciliation_pdf` 两个方法。

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::ar_reconciliation::{
    Entity as ReconciliationEntity, Model as ReconciliationModel,
};
use crate::models::ar_reconciliation_item::{
    Entity as ReconciliationItemEntity, Model as ReconciliationItemModel,
};
use crate::utils::error::AppError;

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
                .unwrap_or("draft"),
            pdf_items,
            &reconciliation.closing_balance.to_string(),
        )
    }
}
