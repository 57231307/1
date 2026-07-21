//! 应付管理统计报表 Service
//!
//! 应付管理统计报表服务层，负责各类统计报表的生成

use crate::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
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

    /// 获取应付统计报表（SQL 层聚合避免全量加载）
    pub async fn get_statistics_report(
        &self,
        supplier_id: Option<i32>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<ApStatisticsReport, AppError> {
        let today = Utc::now().naive_utc().date();
        let main = self
            .fetch_ap_statistics_main_aggregate(supplier_id, start_date, end_date, today)
            .await?;
        let by_status = self
            .fetch_ap_statistics_by_status(supplier_id, start_date, end_date)
            .await?;
        let by_type = self
            .fetch_ap_statistics_by_type(supplier_id, start_date, end_date)
            .await?;
        Ok(ApStatisticsReport {
            period_start: start_date,
            period_end: end_date,
            total_invoice_count: main.total_invoice_count,
            total_invoice_amount: main.total_invoice_amount,
            total_paid_amount: main.total_paid_amount,
            total_unpaid_amount: main.total_unpaid_amount,
            paid_invoice_count: main.paid_invoice_count,
            partial_paid_count: main.partial_paid_count,
            unpaid_count: main.unpaid_count,
            overdue_count: main.overdue_count,
            overdue_amount: main.overdue_amount,
            by_status,
            by_type,
        })
    }

    /// 查询应付统计主聚合数据（COUNT/SUM/逾期分桶）
    async fn fetch_ap_statistics_main_aggregate(
        &self,
        supplier_id: Option<i32>,
        start_date: NaiveDate,
        end_date: NaiveDate,
        today: NaiveDate,
    ) -> Result<ApStatisticsMainAggregate, AppError> {
        use sea_orm::ConnectionTrait;

        // 规则 12 合规：全部参数使用 $N 参数化绑定
        let mut params: Vec<sea_orm::Value> = vec![
            start_date.into(),
            end_date.into(),
            "CANCELLED".into(),
        ];
        // supplier_id 为 Copy 类型，可直接 map 后 push
        let supplier_filter = supplier_id
            .map(|sid| {
                params.push(sid.into());
                format!(" AND supplier_id = ${}", params.len())
            })
            .unwrap_or_default();
        let today_idx = params.len() + 1;
        params.push(today.into());

        let main_sql = format!(
            r#"
            SELECT
                COUNT(*) AS total_invoice_count,
                COALESCE(SUM(amount), 0) AS total_invoice_amount,
                COALESCE(SUM(paid_amount), 0) AS total_paid_amount,
                COALESCE(SUM(unpaid_amount), 0) AS total_unpaid_amount,
                COUNT(CASE WHEN invoice_status = 'PAID' THEN 1 END) AS paid_invoice_count,
                COUNT(CASE WHEN invoice_status = 'PARTIAL_PAID' THEN 1 END) AS partial_paid_count,
                COUNT(CASE WHEN unpaid_amount > 0 AND invoice_status NOT IN ('PAID', 'CANCELLED') THEN 1 END) AS unpaid_count,
                COUNT(CASE WHEN due_date < ${today_idx} AND unpaid_amount > 0 THEN 1 END) AS overdue_count,
                COALESCE(SUM(CASE WHEN due_date < ${today_idx} AND unpaid_amount > 0 THEN unpaid_amount ELSE 0 END), 0) AS overdue_amount
            FROM ap_invoice
            WHERE invoice_date >= $1
              AND invoice_date <= $2
              AND invoice_status <> $3{supplier_filter}
            "#,
            today_idx = today_idx,
            supplier_filter = supplier_filter
        );

        let row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                main_sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("应付统计报表主聚合查询失败: {}", e)))?;
        let row = row
            .ok_or_else(|| AppError::internal("应付统计报表主聚合查询无结果".to_string()))?;

        Ok(ApStatisticsMainAggregate {
            total_invoice_count: row.try_get_by_index::<i64>(0).unwrap_or(0),
            total_invoice_amount: row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO),
            total_paid_amount: row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO),
            total_unpaid_amount: row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO),
            paid_invoice_count: row.try_get_by_index::<i64>(4).unwrap_or(0),
            partial_paid_count: row.try_get_by_index::<i64>(5).unwrap_or(0),
            unpaid_count: row.try_get_by_index::<i64>(6).unwrap_or(0),
            overdue_count: row.try_get_by_index::<i64>(7).unwrap_or(0),
            overdue_amount: row.try_get_by_index::<Decimal>(8).unwrap_or(Decimal::ZERO),
        })
    }

    /// 按状态聚合应付统计（GROUP BY invoice_status）
    async fn fetch_ap_statistics_by_status(
        &self,
        supplier_id: Option<i32>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<StatusStatistics>, AppError> {
        use sea_orm::ConnectionTrait;

        let mut params: Vec<sea_orm::Value> =
            vec![start_date.into(), end_date.into(), "CANCELLED".into()];
        let supplier_filter = supplier_id
            .map(|sid| {
                params.push(sid.into());
                format!(" AND supplier_id = ${}", params.len())
            })
            .unwrap_or_default();
        let sql = format!(
            r#"
            SELECT invoice_status, COUNT(*) AS count, COALESCE(SUM(amount), 0) AS amount
            FROM ap_invoice
            WHERE invoice_date >= $1 AND invoice_date <= $2 AND invoice_status <> $3{sf}
            GROUP BY invoice_status
            "#,
            sf = supplier_filter
        );
        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("按状态聚合查询失败: {}", e)))?;
        Ok(rows
            .into_iter()
            .map(|r| StatusStatistics {
                status: r.try_get_by_index::<String>(0).unwrap_or_default(),
                count: r.try_get_by_index::<i64>(1).unwrap_or(0),
                amount: r.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO),
            })
            .collect())
    }

    /// 按类型聚合应付统计（GROUP BY invoice_type）
    async fn fetch_ap_statistics_by_type(
        &self,
        supplier_id: Option<i32>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<TypeStatistics>, AppError> {
        use sea_orm::ConnectionTrait;

        let mut params: Vec<sea_orm::Value> =
            vec![start_date.into(), end_date.into(), "CANCELLED".into()];
        let supplier_filter = supplier_id
            .map(|sid| {
                params.push(sid.into());
                format!(" AND supplier_id = ${}", params.len())
            })
            .unwrap_or_default();
        let sql = format!(
            r#"
            SELECT invoice_type, COUNT(*) AS count, COALESCE(SUM(amount), 0) AS amount, COALESCE(SUM(unpaid_amount), 0) AS unpaid_amount
            FROM ap_invoice
            WHERE invoice_date >= $1 AND invoice_date <= $2 AND invoice_status <> $3{sf}
            GROUP BY invoice_type
            "#,
            sf = supplier_filter
        );
        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("按类型聚合查询失败: {}", e)))?;
        Ok(rows
            .into_iter()
            .map(|r| TypeStatistics {
                invoice_type: r.try_get_by_index::<String>(0).unwrap_or_default(),
                count: r.try_get_by_index::<i64>(1).unwrap_or(0),
                amount: r.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO),
                unpaid_amount: r.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO),
            })
            .collect())
    }

    /// 获取应付日报
    /// v14 中风险性能修复（批次 245）：3 个聚合查询替代 3 次全量加载
    pub async fn get_daily_report(
        &self,
        report_date: NaiveDate,
        supplier_id: Option<i32>,
    ) -> Result<ApDailyReport, AppError> {
        use sea_orm::ConnectionTrait;

        // 1. 当日新增应付单聚合查询
        let mut new_params: Vec<sea_orm::Value> = vec![report_date.into()];
        let new_supplier_filter = supplier_id
            .map(|sid| {
                new_params.push(sid.into());
                format!(" AND supplier_id = ${}", new_params.len())
            })
            .unwrap_or_default();
        let new_sql = format!(
            r#"
            SELECT COUNT(*) AS cnt, COALESCE(SUM(amount), 0) AS amt
            FROM ap_invoice
            WHERE invoice_date = $1{sf}
            "#,
            sf = new_supplier_filter
        );
        let new_row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                new_sql,
                new_params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("应付日报新增聚合查询失败: {}", e)))?;
        let new_row = new_row
            .ok_or_else(|| AppError::internal("应付日报新增聚合查询无结果".to_string()))?;
        let new_invoice_count: i64 = new_row.try_get_by_index::<i64>(0).unwrap_or(0);
        let new_invoice_amount: Decimal =
            new_row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO);

        // 2. 当日到期应付单聚合查询
        let mut due_params: Vec<sea_orm::Value> = vec![report_date.into(), Decimal::new(0, 2).into()];
        let due_supplier_filter = supplier_id
            .map(|sid| {
                due_params.push(sid.into());
                format!(" AND supplier_id = ${}", due_params.len())
            })
            .unwrap_or_default();
        let due_sql = format!(
            r#"
            SELECT COUNT(*) AS cnt, COALESCE(SUM(unpaid_amount), 0) AS amt
            FROM ap_invoice
            WHERE due_date = $1 AND unpaid_amount > $2{sf}
            "#,
            sf = due_supplier_filter
        );
        let due_row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                due_sql,
                due_params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("应付日报到期聚合查询失败: {}", e)))?;
        let due_row = due_row
            .ok_or_else(|| AppError::internal("应付日报到期聚合查询无结果".to_string()))?;
        let due_invoice_count: i64 = due_row.try_get_by_index::<i64>(0).unwrap_or(0);
        let due_invoice_amount: Decimal =
            due_row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO);

        // 3. 当日付款聚合查询
        let mut pay_params: Vec<sea_orm::Value> = vec![report_date.into(), "CONFIRMED".into()];
        let pay_supplier_filter = supplier_id
            .map(|sid| {
                pay_params.push(sid.into());
                format!(" AND supplier_id = ${}", pay_params.len())
            })
            .unwrap_or_default();
        let pay_sql = format!(
            r#"
            SELECT COUNT(*) AS cnt, COALESCE(SUM(payment_amount), 0) AS amt
            FROM ap_payment
            WHERE payment_date = $1 AND payment_status = $2{sf}
            "#,
            sf = pay_supplier_filter
        );
        let pay_row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                pay_sql,
                pay_params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("应付日报付款聚合查询失败: {}", e)))?;
        let pay_row = pay_row
            .ok_or_else(|| AppError::internal("应付日报付款聚合查询无结果".to_string()))?;
        let payment_count: i64 = pay_row.try_get_by_index::<i64>(0).unwrap_or(0);
        let payment_amount: Decimal =
            pay_row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO);

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
    /// v14 中风险性能修复（批次 245）：月初/月末余额改为 SQL 聚合查询
    /// D08 Tier 4 子批次2：拆分为 ≤50 行主函数 + 2 个 helper（compute_month_date_range / query_ap_invoice_balance）
    pub async fn get_monthly_report(
        &self,
        year: i32,
        month: u32,
        supplier_id: Option<i32>,
    ) -> Result<ApMonthlyReport, AppError> {
        let (start_date, end_date) = Self::compute_month_date_range(year, month)?;
        let statistics = self
            .get_statistics_report(supplier_id, start_date, end_date)
            .await?;
        let opening_balance = self
            .query_ap_invoice_balance(supplier_id, start_date, "<", "月初")
            .await?;
        let closing_balance = self
            .query_ap_invoice_balance(supplier_id, end_date, "<=", "月末")
            .await?;
        Ok(ApMonthlyReport {
            year,
            month,
            supplier_id,
            opening_balance,
            closing_balance,
            statistics,
        })
    }

    /// 计算月报日期范围（月初第一天 + 月末最后一天）
    fn compute_month_date_range(
        year: i32,
        month: u32,
    ) -> Result<(NaiveDate, NaiveDate), AppError> {
        let start_date = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::bad_request("无效的日期参数"))?;
        let next_month_first = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| AppError::bad_request("无效的日期参数"))?;
        let end_date = next_month_first
            .pred_opt()
            .ok_or_else(|| AppError::bad_request("无效的日期参数"))?;
        Ok((start_date, end_date))
    }

    /// 查询应付发票余额聚合（月初/月末复用）
    /// comparison_op: "<" 用于月初（invoice_date < start_date），"<=" 用于月末（invoice_date <= end_date）
    async fn query_ap_invoice_balance(
        &self,
        supplier_id: Option<i32>,
        boundary_date: NaiveDate,
        comparison_op: &str,
        label: &str,
    ) -> Result<Decimal, AppError> {
        use sea_orm::ConnectionTrait;
        let mut params: Vec<sea_orm::Value> = vec!["CANCELLED".into(), boundary_date.into()];
        let supplier_filter = supplier_id
            .map(|sid| {
                params.push(sid.into());
                format!(" AND supplier_id = ${}", params.len())
            })
            .unwrap_or_default();
        let sql = format!(
            r#"
            SELECT COALESCE(SUM(unpaid_amount), 0) AS balance
            FROM ap_invoice
            WHERE invoice_status <> $1 AND invoice_date {op} $2{sf}
            "#,
            op = comparison_op,
            sf = supplier_filter
        );
        let row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("{}余额聚合查询失败: {}", label, e)))?;
        let row = row
            .ok_or_else(|| AppError::internal(format!("{}余额聚合查询无结果", label)))?;
        Ok(row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO))
    }

    /// 获取账龄分析报告
    /// v14 中风险性能修复（批次 245）：SQL CASE WHEN + SUM + COUNT 分桶聚合，避免全量加载
    pub async fn get_aging_report(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<ApAgingReport, AppError> {
        let today = Utc::now().naive_utc().date();
        let overdue = self
            .fetch_aging_overdue_aggregate(today, supplier_id)
            .await?;
        let not_due = self
            .fetch_aging_not_due_aggregate(today, supplier_id)
            .await?;
        let total_amount = not_due.amount + overdue.overdue_total;
        let aging_buckets = build_aging_buckets(not_due, overdue, total_amount);
        Ok(ApAgingReport {
            report_date: today,
            supplier_id,
            total_unpaid_amount: total_amount,
            aging_buckets,
        })
    }

    /// 查询账龄逾期分桶聚合数据（5 个区间 + 逾期合计）
    async fn fetch_aging_overdue_aggregate(
        &self,
        today: NaiveDate,
        supplier_id: Option<i32>,
    ) -> Result<AgingOverdueAggregate, AppError> {
        use sea_orm::ConnectionTrait;

        // 规则 12 合规：全部参数使用 $N 参数化绑定
        let mut params: Vec<sea_orm::Value> =
            vec![today.into(), "PAID".into(), "CANCELLED".into()];
        let supplier_filter = supplier_id
            .map(|sid| {
                params.push(sid.into());
                format!(" AND supplier_id = ${}", params.len())
            })
            .unwrap_or_default();

        let sql = format!(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 1 AND 30 THEN unpaid_amount ELSE 0 END), 0) AS b1_amt,
                COUNT(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 1 AND 30 THEN 1 END) AS b1_cnt,
                COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS b2_amt,
                COUNT(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN 1 END) AS b2_cnt,
                COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS b3_amt,
                COUNT(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN 1 END) AS b3_cnt,
                COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 91 AND 180 THEN unpaid_amount ELSE 0 END), 0) AS b4_amt,
                COUNT(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 91 AND 180 THEN 1 END) AS b4_cnt,
                COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 180 THEN unpaid_amount ELSE 0 END), 0) AS b5_amt,
                COUNT(CASE WHEN (CURRENT_DATE - due_date) > 180 THEN 1 END) AS b5_cnt,
                COALESCE(SUM(unpaid_amount), 0) AS total_amt
            FROM ap_invoice
            WHERE due_date < $1
              AND invoice_status <> $2
              AND invoice_status <> $3
              AND unpaid_amount > 0{sf}
            "#,
            sf = supplier_filter
        );

        let row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("应付账龄报表聚合查询失败: {}", e)))?;
        let row = row
            .ok_or_else(|| AppError::internal("应付账龄报表聚合查询无结果".to_string()))?;

        Ok(AgingOverdueAggregate {
            b1_amt: row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO),
            b1_cnt: row.try_get_by_index::<i64>(1).unwrap_or(0),
            b2_amt: row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO),
            b2_cnt: row.try_get_by_index::<i64>(3).unwrap_or(0),
            b3_amt: row.try_get_by_index::<Decimal>(4).unwrap_or(Decimal::ZERO),
            b3_cnt: row.try_get_by_index::<i64>(5).unwrap_or(0),
            b4_amt: row.try_get_by_index::<Decimal>(6).unwrap_or(Decimal::ZERO),
            b4_cnt: row.try_get_by_index::<i64>(7).unwrap_or(0),
            b5_amt: row.try_get_by_index::<Decimal>(8).unwrap_or(Decimal::ZERO),
            b5_cnt: row.try_get_by_index::<i64>(9).unwrap_or(0),
            overdue_total: row.try_get_by_index::<Decimal>(10).unwrap_or(Decimal::ZERO),
        })
    }

    /// 查询未到期聚合数据（due_date >= today）
    async fn fetch_aging_not_due_aggregate(
        &self,
        today: NaiveDate,
        supplier_id: Option<i32>,
    ) -> Result<AgingNotDueAggregate, AppError> {
        use sea_orm::ConnectionTrait;

        let mut params: Vec<sea_orm::Value> =
            vec![today.into(), "PAID".into(), "CANCELLED".into()];
        let supplier_filter = supplier_id
            .map(|sid| {
                params.push(sid.into());
                format!(" AND supplier_id = ${}", params.len())
            })
            .unwrap_or_default();
        let sql = format!(
            r#"
            SELECT COALESCE(SUM(unpaid_amount), 0) AS amt, COUNT(*) AS cnt
            FROM ap_invoice
            WHERE due_date >= $1 AND invoice_status <> $2 AND invoice_status <> $3 AND unpaid_amount > 0{sf}
            "#,
            sf = supplier_filter
        );
        let row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("未到期聚合查询失败: {}", e)))?;
        let row = row
            .ok_or_else(|| AppError::internal("未到期聚合查询无结果".to_string()))?;
        Ok(AgingNotDueAggregate {
            amount: row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO),
            count: row.try_get_by_index::<i64>(1).unwrap_or(0),
        })
    }
}

/// 构建账龄分桶列表并计算百分比
fn build_aging_buckets(
    not_due: AgingNotDueAggregate,
    overdue: AgingOverdueAggregate,
    total_amount: Decimal,
) -> Vec<AgingBucket> {
    let mut aging_buckets = vec![
        AgingBucket {
            bucket_name: "未到期".to_string(),
            invoice_count: not_due.count,
            total_amount: not_due.amount,
            percentage: Decimal::ZERO,
        },
        AgingBucket {
            bucket_name: "逾期 1-30 天".to_string(),
            invoice_count: overdue.b1_cnt,
            total_amount: overdue.b1_amt,
            percentage: Decimal::ZERO,
        },
        AgingBucket {
            bucket_name: "逾期 31-60 天".to_string(),
            invoice_count: overdue.b2_cnt,
            total_amount: overdue.b2_amt,
            percentage: Decimal::ZERO,
        },
        AgingBucket {
            bucket_name: "逾期 61-90 天".to_string(),
            invoice_count: overdue.b3_cnt,
            total_amount: overdue.b3_amt,
            percentage: Decimal::ZERO,
        },
        AgingBucket {
            bucket_name: "逾期 91-180 天".to_string(),
            invoice_count: overdue.b4_cnt,
            total_amount: overdue.b4_amt,
            percentage: Decimal::ZERO,
        },
        AgingBucket {
            bucket_name: "逾期 180 天以上".to_string(),
            invoice_count: overdue.b5_cnt,
            total_amount: overdue.b5_amt,
            percentage: Decimal::ZERO,
        },
    ];

    if total_amount > Decimal::ZERO {
        for bucket in &mut aging_buckets {
            bucket.percentage = (bucket.total_amount / total_amount) * Decimal::new(100, 0);
        }
    }

    aging_buckets
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 应付统计主聚合查询结果（内部传递用）
#[derive(Debug)]
struct ApStatisticsMainAggregate {
    /// 应付单总数
    total_invoice_count: i64,
    /// 应付总金额
    total_invoice_amount: Decimal,
    /// 已付总金额
    total_paid_amount: Decimal,
    /// 未付总金额
    total_unpaid_amount: Decimal,
    /// 已付清应付单数量
    paid_invoice_count: i64,
    /// 部分付款应付单数量
    partial_paid_count: i64,
    /// 未付款应付单数量
    unpaid_count: i64,
    /// 逾期应付单数量
    overdue_count: i64,
    /// 逾期金额
    overdue_amount: Decimal,
}

/// 应付账龄逾期分桶聚合查询结果（内部传递用）
#[derive(Debug)]
struct AgingOverdueAggregate {
    b1_amt: Decimal,
    b1_cnt: i64,
    b2_amt: Decimal,
    b2_cnt: i64,
    b3_amt: Decimal,
    b3_cnt: i64,
    b4_amt: Decimal,
    b4_cnt: i64,
    b5_amt: Decimal,
    b5_cnt: i64,
    overdue_total: Decimal,
}

/// 应付账龄未到期聚合查询结果（内部传递用）
#[derive(Debug)]
struct AgingNotDueAggregate {
    amount: Decimal,
    count: i64,
}

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
