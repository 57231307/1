//! 应收账款-报表管理子模块（ar_ops/report）
//!
//! 批次 488 D10-1 拆分：从原 `ar_service.rs` L1780-2177 迁移。
//! 包含 9 个报表管理方法：
//! - get_statistics_report / get_daily_report / get_monthly_report / get_aging_report（公开 API）
//! - build_statistics_sql_and_params / build_statistics_response
//! - build_aging_sql_and_params / parse_aging_row / build_aging_response
//!
//! 业务规则：
//! - 报表基于 ar_invoices + ar_collection 聚合查询
//! - 统计/账龄报表使用 SQL 层聚合（v14 P0-2 修复，避免全表加载到内存）
//! - 规则 12 合规：全部参数使用参数化绑定，禁止字符串拼接
//! - 账龄分桶：0-30 / 31-60 / 61-90 / 90+，按 due_date 与 CURRENT_DATE 计算
//! - 统计口径：DRAFT 与 CANCELLED 一律不计入（AR 发票创建即写入 DRAFT，草稿不是
//!   既成应收），与 BI 聚合、仪表盘排除门同口径；状态取值唯一来源
//!   `crate::models::status::common::{STATUS_DRAFT, STATUS_CANCELLED}`，SQL 中一律
//!   `status NOT IN ($k, $k+1)` 参数化绑定常量，禁止裸字面量。

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
        let (sql, params) =
            Self::build_statistics_sql_and_params(start_date, end_date, customer_id, today);

        let row: Option<sea_orm::QueryResult> = self
            .db
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::database(format!("统计报表聚合查询失败: {e}")))?;

        let row = row.ok_or_else(|| AppError::database("统计报表聚合查询无结果".to_string()))?;

        // 解码错误必须向上传播：列类型/表结构错配时不得静默归零成"正常空数据"
        let total_invoices: i64 = row.try_get_by_index::<i64>(0)?;
        let total_amount: Decimal = row.try_get_by_index::<Decimal>(1)?;
        let paid_amount: Decimal = row.try_get_by_index::<Decimal>(2)?;
        let unpaid_amount: Decimal = row.try_get_by_index::<Decimal>(3)?;
        let overdue_count: i64 = row.try_get_by_index::<i64>(4)?;
        let overdue_amount: Decimal = row.try_get_by_index::<Decimal>(5)?;

        Ok(Self::build_statistics_response(
            total_invoices,
            total_amount,
            paid_amount,
            unpaid_amount,
            overdue_count,
            overdue_amount,
        ))
    }

    /// 构建统计报表 SQL 与参数（排除门 NOT IN 绑定 common 词表常量 $1/$2，
    /// 其余条件按 params.len()+1 顺延，today 恒为最后一个占位）
    pub fn build_statistics_sql_and_params(
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
        today: NaiveDate,
    ) -> (String, Vec<sea_orm::Value>) {
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!(
            "status NOT IN (${}, ${})",
            params.len() + 1,
            params.len() + 2
        )];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());
        params.push(crate::models::status::common::STATUS_DRAFT.into());

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
            FROM ar_invoices
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

    /// 获取日报表（按 invoice_date 聚合每日发票金额、已收、未收；v14 中风险性能修复（批次 244）：SQL GROUP BY 聚合，避免全量加载到内存）
    pub async fn get_daily_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 规则 12 合规：全部参数使用参数化绑定（排除门 NOT IN $1/$2，条件按占位顺延）
        let (sql, params) = Self::build_daily_sql_and_params(start_date, end_date, customer_id);

        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::database(format!("日报表聚合查询失败: {e}")))?;

        // 解码错误必须向上传播，禁止 unwrap_or 静默归零
        let result: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| -> Result<serde_json::Value, AppError> {
                let date: NaiveDate = row.try_get_by_index::<NaiveDate>(0)?;
                let invoice_count: i64 = row.try_get_by_index::<i64>(1)?;
                let invoice_amount: Decimal = row.try_get_by_index::<Decimal>(2)?;
                let paid_amount: Decimal = row.try_get_by_index::<Decimal>(3)?;
                let unpaid_amount: Decimal = row.try_get_by_index::<Decimal>(4)?;
                Ok(json!({
                    "date": date.to_string(),
                    "invoice_count": invoice_count,
                    "invoice_amount": invoice_amount.to_string(),
                    "paid_amount": paid_amount.to_string(),
                    "unpaid_amount": unpaid_amount.to_string(),
                }))
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(json!(result))
    }

    /// 构建日报表 SQL 与参数（排除门 NOT IN 绑定 common 词表常量 $1/$2，
    /// customer/日期条件按 params.len()+1 顺延）
    pub fn build_daily_sql_and_params(
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> (String, Vec<sea_orm::Value>) {
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!(
            "status NOT IN (${}, ${})",
            params.len() + 1,
            params.len() + 2
        )];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());
        params.push(crate::models::status::common::STATUS_DRAFT.into());

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
            FROM ar_invoices
            WHERE {where}
            GROUP BY invoice_date
            ORDER BY invoice_date ASC
            "#,
            where = where_clauses.join(" AND ")
        );
        (sql, params)
    }

    /// 获取月报表
    /// v14 中风险性能修复（批次 244）：SQL GROUP BY to_char 月份聚合，避免全量加载到内存
    pub async fn get_monthly_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 规则 12 合规：全部参数使用参数化绑定（排除门 NOT IN $1/$2，条件按占位顺延）
        let (sql, params) = Self::build_monthly_sql_and_params(start_date, end_date, customer_id);

        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::database(format!("月报表聚合查询失败: {e}")))?;

        // 解码错误必须向上传播，禁止 unwrap_or 静默归零
        let result: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| -> Result<serde_json::Value, AppError> {
                let month: String = row.try_get_by_index::<String>(0)?;
                let invoice_count: i64 = row.try_get_by_index::<i64>(1)?;
                let invoice_amount: Decimal = row.try_get_by_index::<Decimal>(2)?;
                let paid_amount: Decimal = row.try_get_by_index::<Decimal>(3)?;
                let unpaid_amount: Decimal = row.try_get_by_index::<Decimal>(4)?;
                Ok(json!({
                    "month": month,
                    "invoice_count": invoice_count,
                    "invoice_amount": invoice_amount.to_string(),
                    "paid_amount": paid_amount.to_string(),
                    "unpaid_amount": unpaid_amount.to_string(),
                }))
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(json!(result))
    }

    /// 构建月报表 SQL 与参数（排除门 NOT IN 绑定 common 词表常量 $1/$2，
    /// customer/日期条件按 params.len()+1 顺延；to_char 月份聚合为 PG 语义）
    pub fn build_monthly_sql_and_params(
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> (String, Vec<sea_orm::Value>) {
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!(
            "status NOT IN (${}, ${})",
            params.len() + 1,
            params.len() + 2
        )];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());
        params.push(crate::models::status::common::STATUS_DRAFT.into());

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
            FROM ar_invoices
            WHERE {where}
            GROUP BY to_char(invoice_date, 'YYYY-MM')
            ORDER BY to_char(invoice_date, 'YYYY-MM') ASC
            "#,
            where = where_clauses.join(" AND ")
        );
        (sql, params)
    }

    /// 获取账龄报表（v14 P0-2 修复：SQL 层聚合，避免全表数据加载到应用层）
    /// 按 due_date 计算 0-30/31-60/61-90/90+ 分桶，数据库层完成 SUM/COUNT 聚合
    /// baseline_date：可选基准日，None 时使用当天（17.4-D3 向后兼容）
    /// salesperson_id：可选业务员 ID，有值时仅统计该业务员负责的发票（17.4-D4）
    pub async fn get_aging_report(
        &self,
        customer_id: Option<i32>,
        baseline_date: Option<chrono::NaiveDate>,
        salesperson_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // v14 P0-2 修复：使用 SQL CASE WHEN + SUM + COUNT 在数据库层完成分桶聚合
        // 避免全表数据加载到应用层导致内存溢出风险（原实现 .all() 加载全部发票到内存）
        // 规则 12 合规：customer_id 使用参数化绑定，禁止字符串拼接
        let today = baseline_date.unwrap_or_else(|| Utc::now().date_naive());
        let (sql, params) = Self::build_aging_sql_and_params(customer_id, today, salesperson_id);

        let result: Option<sea_orm::QueryResult> = self
            .db
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::database(format!("账龄报表聚合查询失败: {e}")))?;

        let row = result.ok_or_else(|| AppError::database("账龄报表聚合查询无结果".to_string()))?;

        let (not_due, bucket_0_30, bucket_31_60, bucket_61_90, bucket_90_plus, invoice_count) =
            Self::parse_aging_row(&row)?;

        Ok(Self::build_aging_response(
            not_due,
            bucket_0_30,
            bucket_31_60,
            bucket_61_90,
            bucket_90_plus,
            invoice_count,
        ))
    }

    /// 构建账龄报表 SQL 与参数（按 customer_id / salesperson_id 是否存在分支；
    /// $1=today，排除门 NOT IN 绑定 common 词表常量 $2/$3，其余条件从 $4 顺延）
    pub fn build_aging_sql_and_params(
        customer_id: Option<i32>,
        today: NaiveDate,
        salesperson_id: Option<i32>,
    ) -> (&'static str, Vec<sea_orm::Value>) {
        // 根据 customer_id 和 salesperson_id 的组合选择不同 SQL
        match (customer_id, salesperson_id) {
            (Some(cid), Some(sid)) => (
                r#"
                SELECT
                    COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                    COALESCE(SUM(CASE WHEN due_date < $1 AND (CURRENT_DATE - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                    COUNT(*) AS invoice_count
                FROM ar_invoices
                WHERE status NOT IN ($2, $3)
                  AND unpaid_amount > 0
                  AND customer_id = $4
                  AND salesperson_id = $5
                "#,
                vec![
                    today.into(),
                    crate::models::status::common::STATUS_CANCELLED.into(),
                    crate::models::status::common::STATUS_DRAFT.into(),
                    cid.into(),
                    sid.into(),
                ],
            ),
            (Some(cid), None) => (
                r#"
                SELECT
                    COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                    COALESCE(SUM(CASE WHEN due_date < $1 AND (CURRENT_DATE - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                    COUNT(*) AS invoice_count
                FROM ar_invoices
                WHERE status NOT IN ($2, $3)
                  AND unpaid_amount > 0
                  AND customer_id = $4
                "#,
                vec![
                    today.into(),
                    crate::models::status::common::STATUS_CANCELLED.into(),
                    crate::models::status::common::STATUS_DRAFT.into(),
                    cid.into(),
                ],
            ),
            (None, Some(sid)) => (
                r#"
                SELECT
                    COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                    COALESCE(SUM(CASE WHEN due_date < $1 AND (CURRENT_DATE - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                    COUNT(*) AS invoice_count
                FROM ar_invoices
                WHERE status NOT IN ($2, $3)
                  AND unpaid_amount > 0
                  AND salesperson_id = $4
                "#,
                vec![
                    today.into(),
                    crate::models::status::common::STATUS_CANCELLED.into(),
                    crate::models::status::common::STATUS_DRAFT.into(),
                    sid.into(),
                ],
            ),
            (None, None) => (
                r#"
                SELECT
                    COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                    COALESCE(SUM(CASE WHEN due_date < $1 AND (CURRENT_DATE - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                    COUNT(*) AS invoice_count
                FROM ar_invoices
                WHERE status NOT IN ($2, $3)
                  AND unpaid_amount > 0
                "#,
                vec![
                    today.into(),
                    crate::models::status::common::STATUS_CANCELLED.into(),
                    crate::models::status::common::STATUS_DRAFT.into(),
                ],
            ),
        }
    }

    /// 解析账龄报表查询结果行（按索引读取 6 个聚合字段，解码错误向上传播）
    fn parse_aging_row(
        row: &sea_orm::QueryResult,
    ) -> Result<(Decimal, Decimal, Decimal, Decimal, Decimal, i64), AppError> {
        let not_due: Decimal = row.try_get_by_index::<Decimal>(0)?;
        let bucket_0_30: Decimal = row.try_get_by_index::<Decimal>(1)?;
        let bucket_31_60: Decimal = row.try_get_by_index::<Decimal>(2)?;
        let bucket_61_90: Decimal = row.try_get_by_index::<Decimal>(3)?;
        let bucket_90_plus: Decimal = row.try_get_by_index::<Decimal>(4)?;
        let invoice_count: i64 = row.try_get_by_index::<i64>(5)?;
        Ok((
            not_due,
            bucket_0_30,
            bucket_31_60,
            bucket_61_90,
            bucket_90_plus,
            invoice_count,
        ))
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

    /// P2-7：按业务员维度 GROUP BY 的账龄报表
    /// 每个业务员返回一组账龄分桶统计
    pub async fn get_aging_by_salesperson(
        &self,
        baseline_date: Option<chrono::NaiveDate>,
    ) -> Result<serde_json::Value, AppError> {
        let today = baseline_date.unwrap_or_else(|| Utc::now().date_naive());

        let sql = r#"
            SELECT
                salesperson_id,
                COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                COALESCE(SUM(CASE WHEN due_date < $1 AND ($1 - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                COALESCE(SUM(CASE WHEN ($1 - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                COALESCE(SUM(CASE WHEN ($1 - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                COALESCE(SUM(CASE WHEN ($1 - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                COUNT(*) AS invoice_count
            FROM ar_invoices
            WHERE status NOT IN ($2, $3)
              AND unpaid_amount > 0
              AND salesperson_id IS NOT NULL
            GROUP BY salesperson_id
            ORDER BY salesperson_id
        "#;

        let params: Vec<sea_orm::Value> = vec![
            today.into(),
            crate::models::status::common::STATUS_CANCELLED.into(),
            crate::models::status::common::STATUS_DRAFT.into(),
        ];

        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::database(format!("业务员账龄聚合查询失败: {e}")))?;

        // 解码错误必须向上传播，禁止 unwrap_or 静默归零
        let result: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| -> Result<serde_json::Value, AppError> {
                let salesperson_id: i32 = row.try_get_by_index::<i32>(0)?;
                let (
                    not_due,
                    bucket_0_30,
                    bucket_31_60,
                    bucket_61_90,
                    bucket_90_plus,
                    invoice_count,
                ) = Self::parse_aging_row(&row)?;
                let total_overdue = bucket_0_30 + bucket_31_60 + bucket_61_90 + bucket_90_plus;
                Ok(json!({
                    "salesperson_id": salesperson_id,
                    "not_due": not_due.to_string(),
                    "bucket_0_30": bucket_0_30.to_string(),
                    "bucket_31_60": bucket_31_60.to_string(),
                    "bucket_61_90": bucket_61_90.to_string(),
                    "bucket_90_plus": bucket_90_plus.to_string(),
                    "total_overdue": total_overdue.to_string(),
                    "invoice_count": invoice_count,
                }))
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(json!({
            "baseline_date": today.to_string(),
            "by_salesperson": result,
        }))
    }
}
