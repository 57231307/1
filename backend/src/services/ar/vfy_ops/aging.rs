//! 应收对账 - 账龄分桶报告（ar/vfy_ops/aging）
//!
//! 批次 490 D10-4b 拆分自原 `ar/vfy.rs` 的 `get_aging_report` 方法及其辅助函数。
//! 职责：按到期日将未结发票金额分入 5 档账龄桶
//! （当期 / 1-30 / 31-60 / 61-90 / 90+），输出客户级与整体汇总。
//! 本模块扩展 `ArReconciliationService` 的 `get_aging_report` 公开方法与
//! `load_unpaid_invoices` / `group_invoices_by_customer` / `init_aging_buckets` /
//! `compute_aging_bucket_index` / `build_customer_aging_summaries` 私有辅助。
//!
//! V15 P1 17.4-D1/D2 扩展：
//! - `save_aging_snapshot`：期末账龄快照入表，支持历史追溯
//! - `get_aging_trend`：账龄趋势分析，按月对比各档金额变化

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::models::ar_aging_analysis;
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

    /// V15 P1 17.4-D1：保存账龄快照
    ///
    /// 期末自动生成账龄快照入表 ar_aging_analysis，按客户粒度记录各档金额，
    /// 支持历史追溯与趋势分析。同一客户同一天仅保留一条快照（覆盖更新）。
    ///
    /// 业务流程：
    /// 1. 调用 get_aging_report 获取当前账龄分布
    /// 2. 遍历每个客户，构造 ar_aging_analysis ActiveModel
    /// 3. 检查同客户同日是否已有快照，有则更新，无则插入
    pub async fn save_aging_snapshot(&self) -> Result<u64, AppError> {
        let report = self.get_aging_report(None).await?;
        let now = Utc::now();
        let mut upserted: u64 = 0;

        for summary in &report.customer_summaries {
            // 检查同客户同日是否已有快照
            let existing = ar_aging_analysis::Entity::find()
                .filter(ar_aging_analysis::Column::CustomerId.eq(summary.customer_id))
                .filter(ar_aging_analysis::Column::AnalysisDate.eq(report.analysis_date))
                .one(&*self.db)
                .await?;

            // 提取各档金额（buckets 顺序：当期/1-30/31-60/61-90/90+）
            let current = summary.buckets.get(0).map(|b| b.amount).unwrap_or(Decimal::ZERO);
            let days_1_30 = summary.buckets.get(1).map(|b| b.amount).unwrap_or(Decimal::ZERO);
            let days_31_60 = summary.buckets.get(2).map(|b| b.amount).unwrap_or(Decimal::ZERO);
            let days_61_90 = summary.buckets.get(3).map(|b| b.amount).unwrap_or(Decimal::ZERO);
            let days_over_90 = summary.buckets.get(4).map(|b| b.amount).unwrap_or(Decimal::ZERO);

            if let Some(existing) = existing {
                // 更新已有快照
                let mut active: ar_aging_analysis::ActiveModel = existing.into();
                active.current_amount = Set(current);
                active.days_1_30 = Set(days_1_30);
                active.days_31_60 = Set(days_31_60);
                active.days_61_90 = Set(days_61_90);
                active.days_over_90 = Set(days_over_90);
                active.total_amount = Set(summary.total_amount);
                active.updated_at = Set(now);
                active.update(&*self.db).await?;
            } else {
                // 插入新快照
                let active = ar_aging_analysis::ActiveModel {
                    customer_id: Set(summary.customer_id),
                    analysis_date: Set(report.analysis_date),
                    current_amount: Set(current),
                    days_1_30: Set(days_1_30),
                    days_31_60: Set(days_31_60),
                    days_61_90: Set(days_61_90),
                    days_over_90: Set(days_over_90),
                    total_amount: Set(summary.total_amount),
                    salesperson_id: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                };
                active.insert(&*self.db).await?;
            }
            upserted += 1;
        }
        Ok(upserted)
    }

    /// V15 P1 17.4-D2：账龄趋势分析
    ///
    /// 返回指定客户（或全部客户汇总）在指定月数内的账龄趋势数据，
    /// 用于观察账龄恶化/改善趋势，支持预警。
    ///
    /// 参数：
    /// - customer_id：可选，None 表示全部客户汇总
    /// - months：回溯月数（默认 6）
    ///
    /// 返回按 analysis_date 升序排列的趋势数据点列表
    pub async fn get_aging_trend(
        &self,
        customer_id: Option<i32>,
        months: Option<i32>,
    ) -> Result<Vec<AgingTrendPoint>, AppError> {
        let months = months.unwrap_or(6).clamp(1, 24);
        let today = Utc::now().date_naive();
        let start_date = today
            .checked_sub_months(chrono::Months::new(months as u32))
            .unwrap_or(today);

        let mut query = ar_aging_analysis::Entity::find()
            .filter(ar_aging_analysis::Column::AnalysisDate.gte(start_date))
            .filter(ar_aging_analysis::Column::AnalysisDate.lte(today));
        if let Some(cid) = customer_id {
            query = query.filter(ar_aging_analysis::Column::CustomerId.eq(cid));
        }
        let snapshots = query
            .order_by_asc(ar_aging_analysis::Column::AnalysisDate)
            .all(&*self.db)
            .await?;

        // 按日期聚合（同一天多条客户快照汇总为一条趋势点）
        let mut trend_map: std::collections::HashMap<chrono::NaiveDate, AgingTrendPoint> =
            std::collections::HashMap::new();
        for snap in snapshots {
            let point = trend_map
                .entry(snap.analysis_date)
                .or_insert_with(|| AgingTrendPoint {
                    analysis_date: snap.analysis_date,
                    current_amount: Decimal::ZERO,
                    days_1_30: Decimal::ZERO,
                    days_31_60: Decimal::ZERO,
                    days_61_90: Decimal::ZERO,
                    days_over_90: Decimal::ZERO,
                    total_amount: Decimal::ZERO,
                    customer_count: 0,
                });
            point.current_amount += snap.current_amount;
            point.days_1_30 += snap.days_1_30;
            point.days_31_60 += snap.days_31_60;
            point.days_61_90 += snap.days_61_90;
            point.days_over_90 += snap.days_over_90;
            point.total_amount += snap.total_amount;
            point.customer_count += 1;
        }
        let mut trend: Vec<AgingTrendPoint> = trend_map.into_values().collect();
        trend.sort_by_key(|t| t.analysis_date);
        Ok(trend)
    }
}

/// V15 P1 17.4-D2：账龄趋势分析数据点
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgingTrendPoint {
    /// 分析日期
    pub analysis_date: chrono::NaiveDate,
    /// 当期金额（未逾期）汇总
    pub current_amount: Decimal,
    /// 1-30 天逾期金额汇总
    pub days_1_30: Decimal,
    /// 31-60 天逾期金额汇总
    pub days_31_60: Decimal,
    /// 61-90 天逾期金额汇总
    pub days_61_90: Decimal,
    /// 90 天以上逾期金额汇总
    pub days_over_90: Decimal,
    /// 总应收金额
    pub total_amount: Decimal,
    /// 客户数（当天有快照的客户数）
    pub customer_count: i64,
}
