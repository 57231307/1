//! 应付管理统计报表 Service
//!
//! 应付管理统计报表服务层，负责各类统计报表的生成

use crate::models::ap_invoice;
use crate::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 应付统计报表服务
pub struct ApReportService {
    db: Arc<DatabaseConnection>,
}

impl ApReportService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取应付统计报表
    pub async fn get_statistics_report(
        &self,
        supplier_id: Option<i32>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<ApStatisticsReport, AppError> {
        let mut query = ap_invoice::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }

        // 查询所有有效应付单
        let invoices = query
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.gte(start_date))
            .filter(ap_invoice::Column::InvoiceDate.lte(end_date))
            .all(&*self.db)
            .await?;

        let mut report = ApStatisticsReport {
            period_start: start_date,
            period_end: end_date,
            total_invoice_count: 0,
            total_invoice_amount: Decimal::new(0, 2),
            total_paid_amount: Decimal::new(0, 2),
            total_unpaid_amount: Decimal::new(0, 2),
            paid_invoice_count: 0,
            partial_paid_count: 0,
            unpaid_count: 0,
            overdue_count: 0,
            overdue_amount: Decimal::new(0, 2),
            by_status: vec![],
            by_type: vec![],
        };

        let today = Utc::now().naive_utc().date();
        let mut status_map: std::collections::HashMap<String, StatusStatistics> =
            std::collections::HashMap::new();
        let mut type_map: std::collections::HashMap<String, TypeStatistics> =
            std::collections::HashMap::new();

        for invoice in invoices {
            report.total_invoice_count += 1;
            report.total_invoice_amount += invoice.amount;
            report.total_paid_amount += invoice.paid_amount;
            report.total_unpaid_amount += invoice.unpaid_amount;

            // 按状态统计
            let status_entry = status_map
                .entry(invoice.invoice_status.clone())
                .or_insert_with(|| StatusStatistics {
                    status: invoice.invoice_status.clone(),
                    count: 0,
                    amount: Decimal::new(0, 2),
                });
            status_entry.count += 1;
            status_entry.amount += invoice.amount;

            // 按类型统计
            let type_entry = type_map
                .entry(invoice.invoice_type.clone())
                .or_insert_with(|| TypeStatistics {
                    invoice_type: invoice.invoice_type.clone(),
                    count: 0,
                    amount: Decimal::new(0, 2),
                    unpaid_amount: Decimal::new(0, 2),
                });
            type_entry.count += 1;
            type_entry.amount += invoice.amount;
            type_entry.unpaid_amount += invoice.unpaid_amount;

            // 统计已付清
            if invoice.invoice_status == "PAID" {
                report.paid_invoice_count += 1;
            } else if invoice.invoice_status == "PARTIAL_PAID" {
                report.partial_paid_count += 1;
            } else if invoice.unpaid_amount > Decimal::new(0, 2) {
                report.unpaid_count += 1;
            }

            // 统计逾期
            if invoice.unpaid_amount > Decimal::new(0, 2) && invoice.due_date < today {
                report.overdue_count += 1;
                report.overdue_amount += invoice.unpaid_amount;
            }
        }

        report.by_status = status_map.into_values().collect();
        report.by_type = type_map.into_values().collect();

        Ok(report)
    }

    /// 获取应付日报
    pub async fn get_daily_report(
        &self,
        report_date: NaiveDate,
        supplier_id: Option<i32>,
    ) -> Result<ApDailyReport, AppError> {
        let mut query = ap_invoice::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }

        // 查询当日新增应付单
        let new_invoices = query
            .clone()
            .filter(ap_invoice::Column::InvoiceDate.eq(report_date))
            .all(&*self.db)
            .await?;

        let new_invoice_count = new_invoices.len() as i64;
        let new_invoice_amount: Decimal = new_invoices.iter().map(|inv| inv.amount).sum();

        // 查询当日到期应付单
        let due_invoices = query
            .clone()
            .filter(ap_invoice::Column::DueDate.eq(report_date))
            .filter(ap_invoice::Column::UnpaidAmount.gt(Decimal::new(0, 2)))
            .all(&*self.db)
            .await?;

        let due_invoice_count = due_invoices.len() as i64;
        let due_invoice_amount: Decimal = due_invoices.iter().map(|inv| inv.unpaid_amount).sum();

        // 查询当日付款
        use crate::models::ap_payment;
        let mut payment_query = ap_payment::Entity::find();

        if let Some(sid) = supplier_id {
            payment_query = payment_query.filter(ap_payment::Column::SupplierId.eq(sid));
        }

        let payments = payment_query
            .filter(ap_payment::Column::PaymentDate.eq(report_date))
            .filter(ap_payment::Column::PaymentStatus.eq("CONFIRMED"))
            .all(&*self.db)
            .await?;

        let payment_count = payments.len() as i64;
        let payment_amount: Decimal = payments.iter().map(|pay| pay.payment_amount).sum();

        Ok(ApDailyReport {
            report_date,
            supplier_id,
            new_invoice_count,
            new_invoice_amount,
            due_invoice_count,
            due_invoice_amount,
            payment_count,
            payment_amount,
        })
    }

    /// 获取应付月报
    pub async fn get_monthly_report(
        &self,
        year: i32,
        month: u32,
        supplier_id: Option<i32>,
    ) -> Result<ApMonthlyReport, AppError> {
        let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let end_date = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
        };

        // 获取统计报表
        let statistics = self
            .get_statistics_report(supplier_id, start_date, end_date)
            .await?;

        // 查询月初余额
        let opening_balance = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(supplier_id.unwrap_or(0)))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.lt(start_date))
            .all(&*self.db)
            .await?
            .iter()
            .map(|inv| inv.unpaid_amount)
            .sum::<Decimal>();

        // 查询月末余额
        let closing_balance = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(supplier_id.unwrap_or(0)))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.lte(end_date))
            .all(&*self.db)
            .await?
            .iter()
            .map(|inv| inv.unpaid_amount)
            .sum::<Decimal>();

        Ok(ApMonthlyReport {
            year,
            month,
            supplier_id,
            opening_balance,
            closing_balance,
            statistics,
        })
    }

    /// 获取账龄分析报告
    pub async fn get_aging_report(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<ApAgingReport, AppError> {
        let mut query = ap_invoice::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }

        // 查询未付清的应付单
        let invoices = query
            .filter(ap_invoice::Column::InvoiceStatus.ne("PAID"))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::UnpaidAmount.gt(Decimal::new(0, 2)))
            .all(&*self.db)
            .await?;

        let today = Utc::now().naive_utc().date();
        let mut aging_buckets = vec![
            AgingBucket {
                bucket_name: "未到期".to_string(),
                invoice_count: 0,
                total_amount: Decimal::new(0, 2),
                percentage: Decimal::new(0, 2),
            },
            AgingBucket {
                bucket_name: "逾期 1-30 天".to_string(),
                invoice_count: 0,
                total_amount: Decimal::new(0, 2),
                percentage: Decimal::new(0, 2),
            },
            AgingBucket {
                bucket_name: "逾期 31-60 天".to_string(),
                invoice_count: 0,
                total_amount: Decimal::new(0, 2),
                percentage: Decimal::new(0, 2),
            },
            AgingBucket {
                bucket_name: "逾期 61-90 天".to_string(),
                invoice_count: 0,
                total_amount: Decimal::new(0, 2),
                percentage: Decimal::new(0, 2),
            },
            AgingBucket {
                bucket_name: "逾期 91-180 天".to_string(),
                invoice_count: 0,
                total_amount: Decimal::new(0, 2),
                percentage: Decimal::new(0, 2),
            },
            AgingBucket {
                bucket_name: "逾期 180 天以上".to_string(),
                invoice_count: 0,
                total_amount: Decimal::new(0, 2),
                percentage: Decimal::new(0, 2),
            },
        ];

        let mut total_amount = Decimal::new(0, 2);

        for invoice in invoices {
            let unpaid = invoice.unpaid_amount;
            let days_overdue = if invoice.due_date < today {
                (today - invoice.due_date).num_days() as i32
            } else {
                -1 // 未到期
            };

            total_amount += unpaid;

            // 分类到对应的账龄区间
            let bucket_index = if days_overdue < 0 {
                0
            } else if days_overdue <= 30 {
                1
            } else if days_overdue <= 60 {
                2
            } else if days_overdue <= 90 {
                3
            } else if days_overdue <= 180 {
                4
            } else {
                5
            };

            aging_buckets[bucket_index].invoice_count += 1;
            aging_buckets[bucket_index].total_amount += unpaid;
        }

        // 计算百分比
        if total_amount > Decimal::new(0, 2) {
            for bucket in &mut aging_buckets {
                bucket.percentage = (bucket.total_amount / total_amount) * Decimal::new(100, 0);
            }
        }

        Ok(ApAgingReport {
            report_date: today,
            supplier_id,
            total_unpaid_amount: total_amount,
            aging_buckets,
        })
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 应付统计报表
#[derive(Debug, Serialize, Deserialize)]
pub struct ApStatisticsReport {
    /// 报表开始日期
    pub period_start: NaiveDate,

    /// 报表结束日期
    pub period_end: NaiveDate,

    /// 应付单总数
    pub total_invoice_count: i64,

    /// 应付总金额
    pub total_invoice_amount: Decimal,

    /// 已付总金额
    pub total_paid_amount: Decimal,

    /// 未付总金额
    pub total_unpaid_amount: Decimal,

    /// 已付清应付单数量
    pub paid_invoice_count: i64,

    /// 部分付款应付单数量
    pub partial_paid_count: i64,

    /// 未付款应付单数量
    pub unpaid_count: i64,

    /// 逾期应付单数量
    pub overdue_count: i64,

    /// 逾期金额
    pub overdue_amount: Decimal,

    /// 按状态统计
    pub by_status: Vec<StatusStatistics>,

    /// 按类型统计
    pub by_type: Vec<TypeStatistics>,
}

/// 状态统计
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusStatistics {
    /// 状态
    pub status: String,

    /// 数量
    pub count: i64,

    /// 金额
    pub amount: Decimal,
}

/// 类型统计
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeStatistics {
    /// 类型
    pub invoice_type: String,

    /// 数量
    pub count: i64,

    /// 金额
    pub amount: Decimal,

    /// 未付金额
    pub unpaid_amount: Decimal,
}

/// 应付日报
#[derive(Debug, Serialize, Deserialize)]
pub struct ApDailyReport {
    /// 报表日期
    pub report_date: NaiveDate,

    /// 供应商 ID
    pub supplier_id: Option<i32>,

    /// 新增应付单数量
    pub new_invoice_count: i64,

    /// 新增应付金额
    pub new_invoice_amount: Decimal,

    /// 到期应付单数量
    pub due_invoice_count: i64,

    /// 到期应付金额
    pub due_invoice_amount: Decimal,

    /// 付款单数量
    pub payment_count: i64,

    /// 付款金额
    pub payment_amount: Decimal,
}

/// 应付月报
#[derive(Debug, Serialize, Deserialize)]
pub struct ApMonthlyReport {
    /// 年份
    pub year: i32,

    /// 月份
    pub month: u32,

    /// 供应商 ID
    pub supplier_id: Option<i32>,

    /// 月初余额
    pub opening_balance: Decimal,

    /// 月末余额
    pub closing_balance: Decimal,

    /// 统计报表
    pub statistics: ApStatisticsReport,
}

/// 账龄区间
#[derive(Debug, Serialize, Deserialize)]
pub struct AgingBucket {
    /// 区间名称
    pub bucket_name: String,

    /// 应付单数量
    pub invoice_count: i64,

    /// 总金额
    pub total_amount: Decimal,

    /// 占比（%）
    pub percentage: Decimal,
}

/// 账龄分析报告
#[derive(Debug, Serialize, Deserialize)]
pub struct ApAgingReport {
    /// 报表日期
    pub report_date: NaiveDate,

    /// 供应商 ID
    pub supplier_id: Option<i32>,

    /// 未付总金额
    pub total_unpaid_amount: Decimal,

    /// 账龄区间
    pub aging_buckets: Vec<AgingBucket>,
}
