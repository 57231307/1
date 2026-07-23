//! 应付单报表分析 impl 子模块（ap_invoice_ops/report）
//!
//! 批次 490 D10-4b 拆分：从原 `ap_invoice_service.rs` L794-930 迁移。
//! 包含 ApInvoiceService 的 3 个报表分析方法：
//! - get_aging_analysis（账龄分析，按 6 区间分桶未付清应付单）
//! - get_balance_summary（应付余额汇总，总应付/已付/未付/数量）
//! - get_statistics（综合统计报表 = 余额汇总 + 账龄分析 + 状态分布）
//!
//! 报表 DTOs（AgingAnalysisItem / BalanceSummary / StatusStatItem / ApInvoiceStatistics）
//! 定义在 `ap_invoice_ops::types`，由 facade 重新导出。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::ap_invoice;
use crate::utils::error::AppError;
use crate::services::ap_invoice_service::{
    AgingAnalysisItem, ApInvoiceService, ApInvoiceStatistics, BalanceSummary, StatusStatItem,
};

impl ApInvoiceService {
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
