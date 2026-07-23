//! 下钻分析 Service impl 子模块（bi_analysis_ops/drilldown）
//!
//! 批次 490 D10-3a 拆分：从原 `bi_analysis_service.rs` L943-1269 迁移。
//! 包含 BiAnalysisService 的 4 个公开方法 + 6 个私有 helper：
//! - drilldown_year_to_month / drilldown_month_to_day
//!   / drilldown_customer_to_order / drilldown_product_to_order（4 公开方法）
//! - query_year_month_rows / fill_year_month_points / build_month_drilldown_sql
//!   / compute_days_in_month / rows_to_period_map / fill_missing_days_with_zeros（6 私有 helper）
//!
//! 业务规则：
//! - 年→月：返回 12 个月完整序列，缺失月份补 0
//! - 月→日：返回当月每日完整序列，缺失日期补 0
//! - 客户/产品→订单：返回最近 100 笔订单明细
//! - V15 P0-B10：所有查询注入行级数据权限过滤

use chrono::Datelike;
use sea_orm::{DatabaseConnection, FromQueryResult, Statement};

use crate::services::bi_analysis_ops::types::{
    CustomerOrderRow, ProductOrderRow, TimeSeriesPoint, TimeSeriesRow,
};
use crate::services::bi_analysis_service::{dec_to_f64, BiAnalysisService};
use crate::utils::data_scope::{build_data_scope_sql, DataScopeContext};
use crate::utils::error::AppError;

impl BiAnalysisService {
    /// 钻取：年 → 月
    ///
    /// 返回指定年份 12 个月的销售聚合，缺失月份补 0。
    pub async fn drilldown_year_to_month(
        &self,
        year: i32,
    ) -> Result<Vec<TimeSeriesPoint>, AppError> {
        if !(1900..=2999).contains(&year) {
            return Err(AppError::validation("年份无效"));
        }
        let rows = Self::query_year_month_rows(&*self.db, &self.data_scope, year).await?;
        Ok(Self::fill_year_month_points(rows, year))
    }

    /// 查询指定年份的月度销售聚合行（应用数据范围过滤）
    async fn query_year_month_rows(
        db: &DatabaseConnection,
        scope_ctx: &DataScopeContext,
        year: i32,
    ) -> Result<Vec<TimeSeriesRow>, AppError> {
        // V15 P0-B10：注入数据范围过滤（sales_orders 无别名，已有 $1 参数）
        let (scope_sql, scope_values) = build_data_scope_sql(scope_ctx, "", 2);
        let sql = format!(
            r#"
            SELECT
                to_char(order_date, 'YYYY-MM') as period,
                SUM(total_amount) as total_amount,
                COUNT(*) as order_count,
                COALESCE(SUM((
                    SELECT SUM(si.quantity) FROM sales_order_items si WHERE si.order_id = sales_orders.id
                )), 0) as quantity,
                COALESCE(SUM((
                    SELECT SUM(si.quantity * COALESCE(p.cost_price, 0))
                    FROM sales_order_items si
                    LEFT JOIN products p ON p.id = si.product_id
                    WHERE si.order_id = sales_orders.id
                )), 0) as profit_amount
            FROM sales_orders
            WHERE EXTRACT(YEAR FROM order_date) = $1
              AND status NOT IN ('CANCELLED', 'DRAFT')
              {scope_sql}
            GROUP BY period
            ORDER BY period ASC
            "#,
            scope_sql = scope_sql,
        );
        let mut values = vec![(year as i64).into()];
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );
        Ok(TimeSeriesRow::find_by_statement(stmt).all(db).await?)
    }

    /// 构建 12 个月完整时间序列，缺失月份补 0
    fn fill_year_month_points(rows: Vec<TimeSeriesRow>, year: i32) -> Vec<TimeSeriesPoint> {
        let mut period_map: std::collections::HashMap<String, TimeSeriesPoint> = std::collections::HashMap::new();
        for r in rows {
            let revenue = dec_to_f64(r.total_amount);
            let cost = dec_to_f64(r.profit_amount);
            period_map.insert(
                r.period.clone(),
                TimeSeriesPoint {
                    period: r.period,
                    total_amount: revenue,
                    order_count: r.order_count.unwrap_or(0),
                    quantity: dec_to_f64(r.quantity),
                    profit_amount: revenue - cost,
                },
            );
        }
        (1..=12)
            .map(|m| {
                let period = format!("{}-{:02}", year, m);
                match period_map.remove(&period) {
                    Some(point) => point,
                    None => TimeSeriesPoint {
                        period,
                        total_amount: 0.0,
                        order_count: 0,
                        quantity: 0.0,
                        profit_amount: 0.0,
                    },
                }
            })
            .collect()
    }

    /// 钻取：月 → 日
    ///
    /// 返回指定月份每日的销售聚合，缺失日期补 0。
    pub async fn drilldown_month_to_day(
        &self,
        year: i32,
        month: u32,
    ) -> Result<Vec<TimeSeriesPoint>, AppError> {
        if !(1..=12).contains(&month) {
            return Err(AppError::validation("月份无效"));
        }
        // V15 P0-B10：注入数据范围过滤（sales_orders 无别名，已有 $1/$2 参数）
        let (scope_sql, scope_values) = self.scope_sql("", 3);
        let sql = Self::build_month_drilldown_sql(&scope_sql);
        let mut values = vec![(year as i64).into(), (month as i64).into()];
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );
        let rows = TimeSeriesRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;
        let days_in_month = Self::compute_days_in_month(year, month);
        let period_map = Self::rows_to_period_map(rows);
        let results = Self::fill_missing_days_with_zeros(period_map, year, month, days_in_month);
        Ok(results)
    }

    /// 构建月→日钻取的 SQL（含数据范围过滤片段）
    fn build_month_drilldown_sql(scope_sql: &str) -> String {
        format!(
            r#"
            SELECT
                to_char(order_date, 'YYYY-MM-DD') as period,
                SUM(total_amount) as total_amount,
                COUNT(*) as order_count,
                COALESCE(SUM((
                    SELECT SUM(si.quantity) FROM sales_order_items si WHERE si.order_id = sales_orders.id
                )), 0) as quantity,
                COALESCE(SUM((
                    SELECT SUM(si.quantity * COALESCE(p.cost_price, 0))
                    FROM sales_order_items si
                    LEFT JOIN products p ON p.id = si.product_id
                    WHERE si.order_id = sales_orders.id
                )), 0) as profit_amount
            FROM sales_orders
            WHERE EXTRACT(YEAR FROM order_date) = $1
              AND EXTRACT(MONTH FROM order_date) = $2
              AND status NOT IN ('CANCELLED', 'DRAFT')
              {scope_sql}
            GROUP BY period
            ORDER BY period ASC
            "#,
            scope_sql = scope_sql,
        )
    }

    /// 计算指定月份的天数（处理 12 月跨年）
    fn compute_days_in_month(year: i32, month: u32) -> u32 {
        chrono::NaiveDate::from_ymd_opt(
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 },
            1,
        )
        .map(|next_month_first| (next_month_first - chrono::Duration::days(1)).day())
        .unwrap_or(30)
    }

    /// 将查询行转换为 period→TimeSeriesPoint 映射
    fn rows_to_period_map(
        rows: Vec<TimeSeriesRow>,
    ) -> std::collections::HashMap<String, TimeSeriesPoint> {
        let mut period_map: std::collections::HashMap<String, TimeSeriesPoint> =
            std::collections::HashMap::new();
        for r in rows {
            let revenue = dec_to_f64(r.total_amount);
            let cost = dec_to_f64(r.profit_amount);
            period_map.insert(
                r.period.clone(),
                TimeSeriesPoint {
                    period: r.period,
                    total_amount: revenue,
                    order_count: r.order_count.unwrap_or(0),
                    quantity: dec_to_f64(r.quantity),
                    profit_amount: revenue - cost,
                },
            );
        }
        period_map
    }

    /// 按日填充缺失日期为零值点，返回完整每日序列
    fn fill_missing_days_with_zeros(
        mut period_map: std::collections::HashMap<String, TimeSeriesPoint>,
        year: i32,
        month: u32,
        days_in_month: u32,
    ) -> Vec<TimeSeriesPoint> {
        (1..=days_in_month)
            .map(|d| {
                let period = format!("{}-{:02}-{:02}", year, month, d);
                match period_map.remove(&period) {
                    Some(point) => point,
                    None => TimeSeriesPoint {
                        period,
                        total_amount: 0.0,
                        order_count: 0,
                        quantity: 0.0,
                        profit_amount: 0.0,
                    },
                }
            })
            .collect()
    }

    /// 钻取：客户 → 订单
    ///
    /// 返回指定客户的最近 100 笔订单明细。
    pub async fn drilldown_customer_to_order(
        &self,
        customer_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        if customer_id <= 0 {
            return Err(AppError::validation("客户 ID 无效"));
        }

        // V15 P0-B10：注入数据范围过滤（sales_orders 无别名，已有 $1 参数）
        let (scope_sql, scope_values) = self.scope_sql("", 2);

        let sql = format!(
            r#"
            SELECT
                id as order_id,
                total_amount as amount,
                order_date::DATE as order_date
            FROM sales_orders
            WHERE customer_id = $1
              AND status NOT IN ('CANCELLED', 'DRAFT')
              {scope_sql}
            ORDER BY order_date DESC
            LIMIT 100
            "#,
            scope_sql = scope_sql,
        );

        let mut values = vec![customer_id.into()];
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );

        let rows = CustomerOrderRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;

        let orders: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "order_id": r.order_id,
                    "amount": dec_to_f64(r.amount),
                    "date": r.order_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
                })
            })
            .collect();

        Ok(serde_json::json!({
            "customer_id": customer_id,
            "orders": orders,
        }))
    }

    /// 钻取：产品 → 订单
    ///
    /// 返回包含指定产品的最近 100 笔订单明细。
    pub async fn drilldown_product_to_order(
        &self,
        product_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        if product_id <= 0 {
            return Err(AppError::validation("产品 ID 无效"));
        }

        // V15 P0-B10：注入数据范围过滤（sales_orders 别名为 s，已有 $1 参数）
        let (scope_sql, scope_values) = self.scope_sql("s", 2);

        let sql = format!(
            r#"
            SELECT
                si.order_id,
                si.quantity,
                si.total_amount as amount
            FROM sales_order_items si
            INNER JOIN sales_orders s ON s.id = si.order_id
                AND s.status NOT IN ('CANCELLED', 'DRAFT')
            WHERE si.product_id = $1
            {scope_sql}
            ORDER BY s.order_date DESC
            LIMIT 100
            "#,
            scope_sql = scope_sql,
        );

        let mut values = vec![product_id.into()];
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );

        let rows = ProductOrderRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;

        let orders: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "order_id": r.order_id,
                    "quantity": dec_to_f64(r.quantity),
                    "amount": dec_to_f64(r.amount),
                })
            })
            .collect();

        Ok(serde_json::json!({
            "product_id": product_id,
            "orders": orders,
        }))
    }
}
