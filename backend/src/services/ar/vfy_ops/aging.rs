//! 应收对账 - 账龄分桶报告（ar/vfy_ops/aging）
//!
//! 批次 490 D10-4b 拆分自原 `ar/vfy.rs` 的 `get_aging_report` 方法及其辅助函数。
//! 职责：按到期日将未结发票金额分入 5 档账龄桶
//! （当期 / 1-30 / 31-60 / 61-90 / 90+），输出客户级与整体汇总。
//! 本模块扩展 `ArReconciliationService` 的 `get_aging_report` 公开方法与
//! `load_unpaid_invoices` / `group_invoices_by_customer` / `init_aging_buckets` /
//! `compute_aging_bucket_index` / `build_customer_aging_summaries` 私有辅助。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::ar_invoice;
use crate::utils::error::AppError;

use super::super::{AgingBucket, AgingReport, ArReconciliationService, CustomerAgingSummary};

impl ArReconciliationService {
    /// 计算账龄分析报告
    ///
    /// 分桶规则：
    /// - 当期（未逾期）：due_date >= 今天
    /// - 1-30天：今天 - due_date 在 1~30 天
    /// - 31-60天：今天 - due_date 在 31~60 天
    /// - 61-90天：今天 - due_date 在 61~90 天
    /// - 90天以上：今天 - due_date > 90 天
    pub async fn get_aging_report(
        &self,
        customer_id: Option<i32>,
    ) -> Result<AgingReport, AppError> {
        let today = Utc::now().date_naive();
        let invoices = self.load_unpaid_invoices(customer_id).await?;
        let customer_map = Self::group_invoices_by_customer(&invoices);
        let mut overall_buckets = Self::init_aging_buckets();
        let (mut customer_summaries, total_receivable) =
            Self::build_customer_aging_summaries(&customer_map, today, &mut overall_buckets);
        customer_summaries.sort_by_key(|b| std::cmp::Reverse(b.total_amount));
        Ok(AgingReport {
            analysis_date: today,
            total_receivable,
            customer_summaries,
            overall_buckets,
        })
    }

    async fn load_unpaid_invoices(
        &self,
        customer_id: Option<i32>,
    ) -> Result<Vec<ar_invoice::Model>, AppError> {
        let mut query = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne("CANCELLED"))
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO));
        if let Some(cid) = customer_id {
            query = query.filter(ar_invoice::Column::CustomerId.eq(cid));
        }
        Ok(query.all(&*self.db).await?)
    }

    fn group_invoices_by_customer(
        invoices: &[ar_invoice::Model],
    ) -> std::collections::HashMap<i32, (String, Vec<&ar_invoice::Model>)> {
        let mut map: std::collections::HashMap<i32, (String, Vec<&ar_invoice::Model>)> =
            std::collections::HashMap::new();
        for inv in invoices {
            let entry = map
                .entry(inv.customer_id)
                .or_insert_with(|| (inv.customer_name.clone().unwrap_or_default(), Vec::new()));
            entry.1.push(inv);
        }
        map
    }

    fn init_aging_buckets() -> Vec<AgingBucket> {
        vec![
            AgingBucket {
                label: "当期".to_string(),
                min_days: 0,
                max_days: Some(0),
                amount: Decimal::ZERO,
                count: 0,
            },
            AgingBucket {
                label: "1-30天".to_string(),
                min_days: 1,
                max_days: Some(30),
                amount: Decimal::ZERO,
                count: 0,
            },
            AgingBucket {
                label: "31-60天".to_string(),
                min_days: 31,
                max_days: Some(60),
                amount: Decimal::ZERO,
                count: 0,
            },
            AgingBucket {
                label: "61-90天".to_string(),
                min_days: 61,
                max_days: Some(90),
                amount: Decimal::ZERO,
                count: 0,
            },
            AgingBucket {
                label: "90天以上".to_string(),
                min_days: 91,
                max_days: None,
                amount: Decimal::ZERO,
                count: 0,
            },
        ]
    }

    fn compute_aging_bucket_index(overdue_days: i64) -> usize {
        if overdue_days <= 0 {
            0
        } else if overdue_days <= 30 {
            1
        } else if overdue_days <= 60 {
            2
        } else if overdue_days <= 90 {
            3
        } else {
            4
        }
    }

    fn build_customer_aging_summaries(
        customer_map: &std::collections::HashMap<i32, (String, Vec<&ar_invoice::Model>)>,
        today: chrono::NaiveDate,
        overall_buckets: &mut [AgingBucket],
    ) -> (Vec<CustomerAgingSummary>, Decimal) {
        let mut customer_summaries = Vec::new();
        let mut total_receivable = Decimal::ZERO;
        for (cust_id, (cust_name, cust_invoices)) in customer_map {
            let mut buckets = Self::init_aging_buckets();
            let mut cust_total = Decimal::ZERO;
            for inv in cust_invoices {
                let overdue_days = (today - inv.due_date).num_days();
                let amount = inv.unpaid_amount;
                cust_total += amount;
                let bucket_idx = Self::compute_aging_bucket_index(overdue_days);
                buckets[bucket_idx].amount += amount;
                buckets[bucket_idx].count += 1;
                overall_buckets[bucket_idx].amount += amount;
                overall_buckets[bucket_idx].count += 1;
            }
            total_receivable += cust_total;
            customer_summaries.push(CustomerAgingSummary {
                customer_id: *cust_id,
                customer_name: cust_name.clone(),
                total_amount: cust_total,
                buckets,
            });
        }
        (customer_summaries, total_receivable)
    }
}
