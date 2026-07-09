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

    /// 获取应付统计报表
    /// v14 中风险性能修复（批次 245）：SQL 层聚合，避免全量加载发票到内存
    pub async fn get_statistics_report(
        &self,
        supplier_id: Option<i32>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<ApStatisticsReport, AppError> {
        use sea_orm::ConnectionTrait;

        let today = Utc::now().naive_utc().date();

        // 规则 12 合规：全部参数使用 $N 参数化绑定
        // 主聚合查询：COUNT + SUM + 条件 COUNT/SUM（overdue/paid/partial/unpaid）
        let mut params: Vec<sea_orm::Value> = vec![
            start_date.into(),
            end_date.into(),
            "CANCELLED".into(),
        ];
        // 规则 12 合规：supplier_id 使用 $N 参数化绑定（i32 为 Copy，可直接 map）
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

        let total_invoice_count: i64 = row.try_get_by_index::<i64>(0).unwrap_or(0);
        let total_invoice_amount: Decimal =
            row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO);
        let total_paid_amount: Decimal =
            row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
        let total_unpaid_amount: Decimal =
            row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
        let paid_invoice_count: i64 = row.try_get_by_index::<i64>(4).unwrap_or(0);
        let partial_paid_count: i64 = row.try_get_by_index::<i64>(5).unwrap_or(0);
        let unpaid_count: i64 = row.try_get_by_index::<i64>(6).unwrap_or(0);
        let overdue_count: i64 = row.try_get_by_index::<i64>(7).unwrap_or(0);
        let overdue_amount: Decimal =
            row.try_get_by_index::<Decimal>(8).unwrap_or(Decimal::ZERO);

        // by_status 子查询：GROUP BY invoice_status
        let mut status_params: Vec<sea_orm::Value> = vec![start_date.into(), end_date.into(), "CANCELLED".into()];
        let status_supplier_filter = supplier_id
            .map(|sid| {
                status_params.push(sid.into());
                format!(" AND supplier_id = ${}", status_params.len())
            })
            .unwrap_or_default();
        let status_sql = format!(
            r#"
            SELECT invoice_status, COUNT(*) AS count, COALESCE(SUM(amount), 0) AS amount
            FROM ap_invoice
            WHERE invoice_date >= $1 AND invoice_date <= $2 AND invoice_status <> $3{ssf}
            GROUP BY invoice_status
            "#,
            ssf = status_supplier_filter
        );
        let status_rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                status_sql,
                status_params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("按状态聚合查询失败: {}", e)))?;
        let by_status: Vec<StatusStatistics> = status_rows
            .into_iter()
            .map(|r| StatusStatistics {
                status: r.try_get_by_index::<String>(0).unwrap_or_default(),
                count: r.try_get_by_index::<i64>(1).unwrap_or(0),
                amount: r.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO),
            })
            .collect();

        // by_type 子查询：GROUP BY invoice_type
        let mut type_params: Vec<sea_orm::Value> = vec![start_date.into(), end_date.into(), "CANCELLED".into()];
        let type_supplier_filter = supplier_id
            .map(|sid| {
                type_params.push(sid.into());
                format!(" AND supplier_id = ${}", type_params.len())
            })
            .unwrap_or_default();
        let type_sql = format!(
            r#"
            SELECT invoice_type, COUNT(*) AS count, COALESCE(SUM(amount), 0) AS amount, COALESCE(SUM(unpaid_amount), 0) AS unpaid_amount
            FROM ap_invoice
            WHERE invoice_date >= $1 AND invoice_date <= $2 AND invoice_status <> $3{tsf}
            GROUP BY invoice_type
            "#,
            tsf = type_supplier_filter
        );
        let type_rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                type_sql,
                type_params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("按类型聚合查询失败: {}", e)))?;
        let by_type: Vec<TypeStatistics> = type_rows
            .into_iter()
            .map(|r| TypeStatistics {
                invoice_type: r.try_get_by_index::<String>(0).unwrap_or_default(),
                count: r.try_get_by_index::<i64>(1).unwrap_or(0),
                amount: r.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO),
                unpaid_amount: r.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO),
            })
            .collect();

        Ok(ApStatisticsReport {
            period_start: start_date,
            period_end: end_date,
            total_invoice_count,
            total_invoice_amount,
            total_paid_amount,
            total_unpaid_amount,
            paid_invoice_count,
            partial_paid_count,
            unpaid_count,
            overdue_count,
            overdue_amount,
            by_status,
            by_type,
        })
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
    pub async fn get_monthly_report(
        &self,
        year: i32,
        month: u32,
        supplier_id: Option<i32>,
    ) -> Result<ApMonthlyReport, AppError> {
        use sea_orm::ConnectionTrait;

        let start_date = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::bad_request("无效的日期参数"))?;
        let end_date = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .ok_or_else(|| AppError::bad_request("无效的日期参数"))?
                .pred_opt()
                .ok_or_else(|| AppError::bad_request("无效的日期参数"))?
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
                .ok_or_else(|| AppError::bad_request("无效的日期参数"))?
                .pred_opt()
                .ok_or_else(|| AppError::bad_request("无效的日期参数"))?
        };

        // 获取统计报表（已在上一步改为 SQL 聚合）
        let statistics = self
            .get_statistics_report(supplier_id, start_date, end_date)
            .await?;

        // 查询月初余额：SQL 聚合 SUM(unpaid_amount)
        let mut opening_params: Vec<sea_orm::Value> = vec!["CANCELLED".into(), start_date.into()];
        let opening_supplier_filter = supplier_id
            .map(|sid| {
                opening_params.push(sid.into());
                format!(" AND supplier_id = ${}", opening_params.len())
            })
            .unwrap_or_default();
        let opening_sql = format!(
            r#"
            SELECT COALESCE(SUM(unpaid_amount), 0) AS opening_balance
            FROM ap_invoice
            WHERE invoice_status <> $1 AND invoice_date < $2{sf}
            "#,
            sf = opening_supplier_filter
        );
        let opening_row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                opening_sql,
                opening_params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("月初余额聚合查询失败: {}", e)))?;
        let opening_row = opening_row
            .ok_or_else(|| AppError::internal("月初余额聚合查询无结果".to_string()))?;
        let opening_balance: Decimal =
            opening_row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO);

        // 查询月末余额：SQL 聚合 SUM(unpaid_amount)
        let mut closing_params: Vec<sea_orm::Value> = vec!["CANCELLED".into(), end_date.into()];
        let closing_supplier_filter = supplier_id
            .map(|sid| {
                closing_params.push(sid.into());
                format!(" AND supplier_id = ${}", closing_params.len())
            })
            .unwrap_or_default();
        let closing_sql = format!(
            r#"
            SELECT COALESCE(SUM(unpaid_amount), 0) AS closing_balance
            FROM ap_invoice
            WHERE invoice_status <> $1 AND invoice_date <= $2{sf}
            "#,
            sf = closing_supplier_filter
        );
        let closing_row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                closing_sql,
                closing_params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("月末余额聚合查询失败: {}", e)))?;
        let closing_row = closing_row
            .ok_or_else(|| AppError::internal("月末余额聚合查询无结果".to_string()))?;
        let closing_balance: Decimal =
            closing_row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO);

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
    /// v14 中风险性能修复（批次 245）：SQL CASE WHEN + SUM + COUNT 分桶聚合，避免全量加载
    pub async fn get_aging_report(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<ApAgingReport, AppError> {
        use sea_orm::ConnectionTrait;

        let today = Utc::now().naive_utc().date();

        // 规则 12 合规：全部参数使用 $N 参数化绑定
        let mut params: Vec<sea_orm::Value> = vec![today.into(), "PAID".into(), "CANCELLED".into()];
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

        // 未到期单独查询（due_date >= today）
        let mut not_due_params: Vec<sea_orm::Value> = vec![today.into(), "PAID".into(), "CANCELLED".into()];
        let not_due_supplier_filter = supplier_id
            .map(|sid| {
                not_due_params.push(sid.into());
                format!(" AND supplier_id = ${}", not_due_params.len())
            })
            .unwrap_or_default();
        let not_due_sql = format!(
            r#"
            SELECT COALESCE(SUM(unpaid_amount), 0) AS amt, COUNT(*) AS cnt
            FROM ap_invoice
            WHERE due_date >= $1 AND invoice_status <> $2 AND invoice_status <> $3 AND unpaid_amount > 0{sf}
            "#,
            sf = not_due_supplier_filter
        );
        let not_due_row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                not_due_sql,
                not_due_params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("未到期聚合查询失败: {}", e)))?;
        let not_due_row = not_due_row
            .ok_or_else(|| AppError::internal("未到期聚合查询无结果".to_string()))?;
        let not_due_amt: Decimal =
            not_due_row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO);
        let not_due_cnt: i64 = not_due_row.try_get_by_index::<i64>(1).unwrap_or(0);

        // 读取逾期分桶（索引 0-9 是 5 个分桶的 amt/cnt，索引 10 是 overdue total）
        let b1_amt: Decimal = row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO);
        let b1_cnt: i64 = row.try_get_by_index::<i64>(1).unwrap_or(0);
        let b2_amt: Decimal = row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
        let b2_cnt: i64 = row.try_get_by_index::<i64>(3).unwrap_or(0);
        let b3_amt: Decimal = row.try_get_by_index::<Decimal>(4).unwrap_or(Decimal::ZERO);
        let b3_cnt: i64 = row.try_get_by_index::<i64>(5).unwrap_or(0);
        let b4_amt: Decimal = row.try_get_by_index::<Decimal>(6).unwrap_or(Decimal::ZERO);
        let b4_cnt: i64 = row.try_get_by_index::<i64>(7).unwrap_or(0);
        let b5_amt: Decimal = row.try_get_by_index::<Decimal>(8).unwrap_or(Decimal::ZERO);
        let b5_cnt: i64 = row.try_get_by_index::<i64>(9).unwrap_or(0);
        let overdue_total: Decimal =
            row.try_get_by_index::<Decimal>(10).unwrap_or(Decimal::ZERO);

        let total_amount = not_due_amt + overdue_total;

        // 构建分桶（百分比在应用层计算，因为依赖 total_amount）
        let mut aging_buckets = vec![
            AgingBucket {
                bucket_name: "未到期".to_string(),
                invoice_count: not_due_cnt,
                total_amount: not_due_amt,
                percentage: Decimal::ZERO,
            },
            AgingBucket {
                bucket_name: "逾期 1-30 天".to_string(),
                invoice_count: b1_cnt,
                total_amount: b1_amt,
                percentage: Decimal::ZERO,
            },
            AgingBucket {
                bucket_name: "逾期 31-60 天".to_string(),
                invoice_count: b2_cnt,
                total_amount: b2_amt,
                percentage: Decimal::ZERO,
            },
            AgingBucket {
                bucket_name: "逾期 61-90 天".to_string(),
                invoice_count: b3_cnt,
                total_amount: b3_amt,
                percentage: Decimal::ZERO,
            },
            AgingBucket {
                bucket_name: "逾期 91-180 天".to_string(),
                invoice_count: b4_cnt,
                total_amount: b4_amt,
                percentage: Decimal::ZERO,
            },
            AgingBucket {
                bucket_name: "逾期 180 天以上".to_string(),
                invoice_count: b5_cnt,
                total_amount: b5_amt,
                percentage: Decimal::ZERO,
            },
        ];

        // 计算百分比
        if total_amount > Decimal::ZERO {
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
