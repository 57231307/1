//! 应收账款-报表管理子模块（ar_ops/report）
//!
//! 批次 488 D10-1 拆分：从原 `ar_service.rs` L1780-2177 迁移。
//! 包含 9 个报表管理方法：
//! - get_statistics_report / get_daily_report / get_monthly_report / get_aging_report（公开 API）
//! - build_statistics_sql_and_params / build_statistics_response
//! - build_aging_sql_and_params / parse_aging_row / build_aging_response
//!
//! 业务规则：
//! - 报表基于 ar_invoice + ar_collection 聚合查询
//! - 统计/账龄报表使用 SQL 层聚合（v14 P0-2 修复，避免全表加载到内存）
//! - 规则 12 合规：全部参数使用参数化绑定，禁止字符串拼接
//! - 账龄分桶：0-30 / 31-60 / 61-90 / 90+，按 due_date 与 CURRENT_DATE 计算

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::ConnectionTrait;
use serde_json::json;

use crate::services::ar_service::ArService;
use crate::utils::error::AppError;

impl ArService {
    // ========== 报表管理 ==========

    /// 获取统计报表
    /// v14 中风险性能修复（批次 244）：SQL 层聚合，避免全量加载发票到内存
    pub async fn get_statistics_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 规则 12 合规：全部参数使用参数化绑定，禁止字符串拼接
        let today = Utc::now().date_naive();
        let (sql, params) = Self::build_statistics_sql_and_params(
            start_date, end_date, customer_id, today,
        );

        let row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("统计报表聚合查询失败: {}", e)))?;

        let row = row
            .ok_or_else(|| AppError::internal("统计报表聚合查询无结果".to_string()))?;

        let total_invoices: i64 = row.try_get_by_index::<i64>(0).unwrap_or(0);
        let total_amount: Decimal = row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO);
        let paid_amount: Decimal = row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
        let unpaid_amount: Decimal = row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
        let overdue_count: i64 = row.try_get_by_index::<i64>(4).unwrap_or(0);
        let overdue_amount: Decimal = row.try_get_by_index::<Decimal>(5).unwrap_or(Decimal::ZERO);

        Ok(Self::build_statistics_response(
            total_invoices,
            total_amount,
            paid_amount,
            unpaid_amount,
            overdue_count,
            overdue_amount,
        ))
    }

    /// 构建统计报表 SQL 与参数（参数化绑定 where 条件 + today 逾期占位）
    fn build_statistics_sql_and_params(
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
        today: NaiveDate,
    ) -> (String, Vec<sea_orm::Value>) {
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!("status <> ${}", params.len() + 1)];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());

        if let Some(cid) = customer_id {
            where_clauses.push(format!("customer_id = ${}", params.len() + 1));
            params.push(cid.into());
        }
        if let Some(sd) = start_date {
            where_clauses.push(format!("invoice_date >= ${}", params.len() + 1));
            params.push(sd.into());
        }
        if let Some(ed) = end_date {
            where_clauses.push(format!("invoice_date <= ${}", params.len() + 1));
            params.push(ed.into());
        }
        // today 用于逾期条件
        let today_param_idx = params.len() + 1;
        params.push(today.into());

        let sql = format!(
            r#"
            SELECT
                COUNT(*) AS total_invoices,
                COALESCE(SUM(invoice_amount), 0) AS total_amount,
                COALESCE(SUM(received_amount), 0) AS paid_amount,
                COALESCE(SUM(unpaid_amount), 0) AS unpaid_amount,
                COUNT(CASE WHEN due_date < ${today_idx} AND unpaid_amount > 0 THEN 1 END) AS overdue_count,
                COALESCE(SUM(CASE WHEN due_date < ${today_idx} AND unpaid_amount > 0 THEN unpaid_amount ELSE 0 END), 0) AS overdue_amount
            FROM ar_invoice
            WHERE {where}
            "#,
            today_idx = today_param_idx,
            where = where_clauses.join(" AND ")
        );
        (sql, params)
    }

    /// 构建统计报表响应 JSON（含 collection_rate 回款率计算）
    fn build_statistics_response(
        total_invoices: i64,
        total_amount: Decimal,
        paid_amount: Decimal,
        unpaid_amount: Decimal,
        overdue_count: i64,
        overdue_amount: Decimal,
    ) -> serde_json::Value {
        let collection_rate = if total_amount > Decimal::ZERO {
            (paid_amount / total_amount)
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
        } else {
            0.0
        };
        json!({
            "total_invoices": total_invoices,
            "total_amount": total_amount.to_string(),
            "paid_amount": paid_amount.to_string(),
            "unpaid_amount": unpaid_amount.to_string(),
            "overdue_count": overdue_count,
            "overdue_amount": overdue_amount.to_string(),
            "collection_rate": collection_rate,
        })
    }

    /// 获取日报表
    /// 按 invoice_date 聚合每日发票金额、已收、未收
    /// v14 中风险性能修复（批次 244）：SQL GROUP BY 聚合，避免全量加载到内存
    pub async fn get_daily_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 规则 12 合规：全部参数使用参数化绑定
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!("status <> ${}", params.len() + 1)];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());

        if let Some(cid) = customer_id {
            where_clauses.push(format!("customer_id = ${}", params.len() + 1));
            params.push(cid.into());
        }
        if let Some(sd) = start_date {
            where_clauses.push(format!("invoice_date >= ${}", params.len() + 1));
            params.push(sd.into());
        }
        if let Some(ed) = end_date {
            where_clauses.push(format!("invoice_date <= ${}", params.len() + 1));
            params.push(ed.into());
        }

        let sql = format!(
            r#"
            SELECT
                invoice_date,
                COUNT(*) AS invoice_count,
                COALESCE(SUM(invoice_amount), 0) AS invoice_amount,
                COALESCE(SUM(received_amount), 0) AS paid_amount,
                COALESCE(SUM(unpaid_amount), 0) AS unpaid_amount
            FROM ar_invoice
            WHERE {where}
            GROUP BY invoice_date
            ORDER BY invoice_date ASC
            "#,
            where = where_clauses.join(" AND ")
        );

        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("日报表聚合查询失败: {}", e)))?;

        let result: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                let date: NaiveDate = row.try_get_by_index::<NaiveDate>(0).unwrap_or_default();
                let invoice_count: i64 = row.try_get_by_index::<i64>(1).unwrap_or(0);
                let invoice_amount: Decimal =
                    row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
                let paid_amount: Decimal =
                    row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
                let unpaid_amount: Decimal =
                    row.try_get_by_index::<Decimal>(4).unwrap_or(Decimal::ZERO);
                json!({
                    "date": date.to_string(),
                    "invoice_count": invoice_count,
                    "invoice_amount": invoice_amount.to_string(),
                    "paid_amount": paid_amount.to_string(),
                    "unpaid_amount": unpaid_amount.to_string(),
                })
            })
            .collect();

        Ok(json!(result))
    }

    /// 获取月报表
    /// v14 中风险性能修复（批次 244）：SQL GROUP BY to_char 月份聚合，避免全量加载到内存
    pub async fn get_monthly_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 规则 12 合规：全部参数使用参数化绑定
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!("status <> ${}", params.len() + 1)];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());

        if let Some(cid) = customer_id {
            where_clauses.push(format!("customer_id = ${}", params.len() + 1));
            params.push(cid.into());
        }
        if let Some(sd) = start_date {
            where_clauses.push(format!("invoice_date >= ${}", params.len() + 1));
            params.push(sd.into());
        }
        if let Some(ed) = end_date {
            where_clauses.push(format!("invoice_date <= ${}", params.len() + 1));
            params.push(ed.into());
        }

        let sql = format!(
            r#"
            SELECT
                to_char(invoice_date, 'YYYY-MM') AS month,
                COUNT(*) AS invoice_count,
                COALESCE(SUM(invoice_amount), 0) AS invoice_amount,
                COALESCE(SUM(received_amount), 0) AS paid_amount,
                COALESCE(SUM(unpaid_amount), 0) AS unpaid_amount
            FROM ar_invoice
            WHERE {where}
            GROUP BY to_char(invoice_date, 'YYYY-MM')
            ORDER BY to_char(invoice_date, 'YYYY-MM') ASC
            "#,
            where = where_clauses.join(" AND ")
        );

        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("月报表聚合查询失败: {}", e)))?;

        let result: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                let month: String = row.try_get_by_index::<String>(0).unwrap_or_default();
                let invoice_count: i64 = row.try_get_by_index::<i64>(1).unwrap_or(0);
                let invoice_amount: Decimal =
                    row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
                let paid_amount: Decimal =
                    row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
                let unpaid_amount: Decimal =
                    row.try_get_by_index::<Decimal>(4).unwrap_or(Decimal::ZERO);
                json!({
                    "month": month,
                    "invoice_count": invoice_count,
                    "invoice_amount": invoice_amount.to_string(),
                    "paid_amount": paid_amount.to_string(),
                    "unpaid_amount": unpaid_amount.to_string(),
                })
            })
            .collect();

        Ok(json!(result))
    }

    /// 获取账龄报表（v14 P0-2 修复：SQL 层聚合，避免全表数据加载到应用层）
    /// 按 due_date 计算 0-30/31-60/61-90/90+ 分桶，数据库层完成 SUM/COUNT 聚合
    pub async fn get_aging_report(
        &self,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // v14 P0-2 修复：使用 SQL CASE WHEN + SUM + COUNT 在数据库层完成分桶聚合
        // 避免全表数据加载到应用层导致内存溢出风险（原实现 .all() 加载全部发票到内存）
        // 规则 12 合规：customer_id 使用参数化绑定，禁止字符串拼接
        let today = Utc::now().date_naive();
        let (sql, params) = Self::build_aging_sql_and_params(customer_id, today);

        let result: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("账龄报表聚合查询失败: {}", e)))?;

        let row = result
            .ok_or_else(|| AppError::internal("账龄报表聚合查询无结果".to_string()))?;

        let (not_due, bucket_0_30, bucket_31_60, bucket_61_90, bucket_90_plus, invoice_count) =
            Self::parse_aging_row(&row);

        Ok(Self::build_aging_response(
            not_due,
            bucket_0_30,
            bucket_31_60,
            bucket_61_90,
            bucket_90_plus,
            invoice_count,
        ))
    }

    /// 构建账龄报表 SQL 与参数（按 customer_id 是否存在分支）
    fn build_aging_sql_and_params(
        customer_id: Option<i32>,
        today: NaiveDate,
    ) -> (&'static str, Vec<sea_orm::Value>) {
        if let Some(cid) = customer_id {
            (
                r#"
                SELECT
                    COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                    COALESCE(SUM(CASE WHEN due_date < $1 AND (CURRENT_DATE - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                    COUNT(*) AS invoice_count
                FROM ar_invoice
                WHERE status <> $2
                  AND unpaid_amount > 0
                  AND customer_id = $3
                "#,
                vec![
                    today.into(),
                    crate::models::status::common::STATUS_CANCELLED.into(),
                    cid.into(),
                ],
            )
        } else {
            (
                r#"
                SELECT
                    COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                    COALESCE(SUM(CASE WHEN due_date < $1 AND (CURRENT_DATE - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                    COUNT(*) AS invoice_count
                FROM ar_invoice
                WHERE status <> $2
                  AND unpaid_amount > 0
                "#,
                vec![
                    today.into(),
                    crate::models::status::common::STATUS_CANCELLED.into(),
                ],
            )
        }
    }

    /// 解析账龄报表查询结果行（按索引读取 6 个聚合字段）
    fn parse_aging_row(
        row: &sea_orm::QueryResult,
    ) -> (Decimal, Decimal, Decimal, Decimal, Decimal, i64) {
        let not_due: Decimal = row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO);
        let bucket_0_30: Decimal = row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO);
        let bucket_31_60: Decimal = row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
        let bucket_61_90: Decimal = row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
        let bucket_90_plus: Decimal = row.try_get_by_index::<Decimal>(4).unwrap_or(Decimal::ZERO);
        let invoice_count: i64 = row.try_get_by_index::<i64>(5).unwrap_or(0);
        (
            not_due,
            bucket_0_30,
            bucket_31_60,
            bucket_61_90,
            bucket_90_plus,
            invoice_count,
        )
    }

    /// 构建账龄报表响应 JSON（含 total_overdue 汇总）
    fn build_aging_response(
        not_due: Decimal,
        bucket_0_30: Decimal,
        bucket_31_60: Decimal,
        bucket_61_90: Decimal,
        bucket_90_plus: Decimal,
        invoice_count: i64,
    ) -> serde_json::Value {
        let total_overdue = bucket_0_30 + bucket_31_60 + bucket_61_90 + bucket_90_plus;
        json!({
            "not_due": not_due.to_string(),
            "bucket_0_30": bucket_0_30.to_string(),
            "bucket_31_60": bucket_31_60.to_string(),
            "bucket_61_90": bucket_61_90.to_string(),
            "bucket_90_plus": bucket_90_plus.to_string(),
            "total_overdue": total_overdue.to_string(),
            "invoice_count": invoice_count,
        })
    }
}
