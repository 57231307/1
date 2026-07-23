//! 销售聚合分析 Service impl 子模块（bi_analysis_ops/sales）
//!
//! 批次 490 D10-3a 拆分：从原 `bi_analysis_service.rs` L342-706 迁移。
//! 包含 BiAnalysisService 的 6 个公开方法 + 5 个私有 helper：
//! - sales_by_time / sales_by_customer / sales_by_product / sales_by_region
//!   / sales_by_category / sales_trend（6 公开方法）
//! - build_period_expr / map_time_series_rows / query_customer_rank_rows
//!   / query_total_sales / build_customer_ranks（5 私有 helper）
//!
//! 业务规则：
//! - 排除 CANCELLED 和 DRAFT 状态的订单
//! - 利润 = 销售额 - 成本，成本 = SUM(sales_order_items.quantity * products.cost_price)
//! - V15 P0-B10：所有查询注入行级数据权限过滤

use sea_orm::{DatabaseConnection, FromQueryResult, Statement};

use crate::services::bi_analysis_ops::types::{
    CategoryStatRow, CustomerRank, CustomerRankRow, ProductRank, ProductRankRow, RegionStat,
    RegionStatRow, TimeSeriesPoint, TimeSeriesRow, TotalRow, CategoryStat,
};
use crate::services::bi_analysis_service::{dec_to_f64, BiAnalysisService};
use crate::utils::data_scope::{build_data_scope_sql, DataScopeContext};
use crate::utils::error::AppError;

impl BiAnalysisService {
    /// 按时间聚合销售
    ///
    /// 根据 granularity（day/week/month/quarter/year）分组聚合销售额、订单数、数量、利润。
    /// 利润 = 销售额 - 成本，成本 = SUM(sales_order_items.quantity * products.cost_price)。
    pub async fn sales_by_time(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
        granularity: &str,
    ) -> Result<Vec<TimeSeriesPoint>, AppError> {
        if end_date < start_date {
            return Err(AppError::validation("结束日期不能早于开始日期"));
        }

        let period_expr = Self::build_period_expr(granularity);

        // V15 P0-B10：注入数据范围过滤（sales_orders 别名为 s，已有 $1/$2 两个参数）
        let (scope_sql, scope_values) = self.scope_sql("s", 3);

        let sql = format!(
            r#"
            SELECT
                {period} as period,
                SUM(s.total_amount) as total_amount,
                COUNT(*) as order_count,
                COALESCE(SUM((
                    SELECT SUM(si.quantity) FROM sales_order_items si WHERE si.order_id = s.id
                )), 0) as quantity,
                COALESCE(SUM((
                    SELECT SUM(si.quantity * COALESCE(p.cost_price, 0))
                    FROM sales_order_items si
                    LEFT JOIN products p ON p.id = si.product_id
                    WHERE si.order_id = s.id
                )), 0) as profit_amount
            FROM sales_orders s
            WHERE s.order_date >= $1 AND s.order_date <= $2
              AND s.status NOT IN ('CANCELLED', 'DRAFT')
              {scope_sql}
            GROUP BY period
            ORDER BY period ASC
            "#,
            period = period_expr,
            scope_sql = scope_sql,
        );

        let mut values = vec![start_date.into(), end_date.into()];
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );

        let rows = TimeSeriesRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;

        Ok(Self::map_time_series_rows(rows))
    }

    /// 根据 granularity 返回 Postgres to_char 分组表达式
    fn build_period_expr(granularity: &str) -> &'static str {
        match granularity {
            "day" => "to_char(order_date, 'YYYY-MM-DD')",
            "week" => "to_char(order_date, 'IYYY-IW')",
            "month" => "to_char(order_date, 'YYYY-MM')",
            "quarter" => "to_char(order_date, 'YYYY-Q')",
            "year" => "to_char(order_date, 'YYYY')",
            _ => "to_char(order_date, 'YYYY-MM')",
        }
    }

    /// 将查询行映射为 TimeSeriesPoint（profit_amount = 收入 - 成本）
    fn map_time_series_rows(rows: Vec<TimeSeriesRow>) -> Vec<TimeSeriesPoint> {
        rows.into_iter()
            .map(|r| {
                let revenue = dec_to_f64(r.total_amount);
                let cost = dec_to_f64(r.profit_amount);
                TimeSeriesPoint {
                    period: r.period,
                    total_amount: revenue,
                    order_count: r.order_count.unwrap_or(0),
                    quantity: dec_to_f64(r.quantity),
                    profit_amount: revenue - cost,
                }
            })
            .collect()
    }

    /// 按客户聚合销售
    ///
    /// 返回销售额 TOP N 客户排行，percentage = 客户销售额 / 全部销售额 * 100。
    pub async fn sales_by_customer(
        &self,
        limit: i64,
    ) -> Result<Vec<CustomerRank>, AppError> {
        let limit = limit.clamp(1, 100);
        let rows = Self::query_customer_rank_rows(&*self.db, &self.data_scope, limit).await?;
        let total_sales = Self::query_total_sales(&*self.db, &self.data_scope).await?;
        Ok(Self::build_customer_ranks(rows, total_sales))
    }

    /// 查询客户销售排行原始行（应用数据范围过滤）
    async fn query_customer_rank_rows(
        db: &DatabaseConnection,
        scope_ctx: &DataScopeContext,
        limit: i64,
    ) -> Result<Vec<CustomerRankRow>, AppError> {
        // V15 P0-B10：注入数据范围过滤（LEFT JOIN sales_orders s，过滤条件加在 WHERE）
        // 注：将过滤加到 WHERE 会把 LEFT JOIN 变为 INNER JOIN 效果，
        //     即只返回有符合数据范围订单的客户（业务期望：员工只看到自己客户的排行）
        let (scope_sql, scope_values) = build_data_scope_sql(scope_ctx, "s", 2);
        let sql = format!(
            r#"
            SELECT
                c.id as customer_id,
                c.customer_name,
                COALESCE(SUM(s.total_amount), 0) as total_amount,
                COUNT(s.id) as order_count
            FROM customers c
            LEFT JOIN sales_orders s ON s.customer_id = c.id
                AND s.status NOT IN ('CANCELLED', 'DRAFT')
            WHERE 1=1 {scope_sql}
            GROUP BY c.id, c.customer_name
            ORDER BY total_amount DESC
            LIMIT $1
            "#,
            scope_sql = scope_sql,
        );
        let mut values = vec![limit.into()];
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );
        Ok(CustomerRankRow::find_by_statement(stmt).all(db).await?)
    }

    /// 查询全部销售额（应用数据范围过滤，用于 percentage 计算）
    async fn query_total_sales(
        db: &DatabaseConnection,
        scope_ctx: &DataScopeContext,
    ) -> Result<f64, AppError> {
        let (total_scope_sql, total_scope_values) = build_data_scope_sql(scope_ctx, "sales_orders", 1);
        let total_sql = format!(
            r#"SELECT COALESCE(SUM(total_amount), 0) as total FROM sales_orders
               WHERE status NOT IN ('CANCELLED', 'DRAFT') {scope_sql}"#,
            scope_sql = total_scope_sql,
        );
        let mut total_values: Vec<sea_orm::Value> = Vec::new();
        total_values.extend(total_scope_values);
        let total_stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            total_sql,
            total_values,
        );
        let total_row: Option<TotalRow> = TotalRow::find_by_statement(total_stmt)
            .one(db)
            .await?;
        Ok(total_row.map(|r| dec_to_f64(r.total)).unwrap_or(0.0))
    }

    /// 将原始行转换为客户排行结果（计算 percentage）
    fn build_customer_ranks(rows: Vec<CustomerRankRow>, total_sales: f64) -> Vec<CustomerRank> {
        rows.into_iter()
            .map(|r| {
                let amount = dec_to_f64(r.total_amount);
                let percentage = if total_sales > 0.0 {
                    amount / total_sales * 100.0
                } else {
                    0.0
                };
                CustomerRank {
                    customer_id: r.customer_id as i64,
                    customer_name: r.customer_name,
                    total_amount: amount,
                    order_count: r.order_count.unwrap_or(0),
                    percentage,
                }
            })
            .collect()
    }

    /// 按产品聚合销售
    ///
    /// 返回销售额 TOP N 产品排行，关联 product_categories 获取品类名。
    pub async fn sales_by_product(
        &self,
        limit: i64,
    ) -> Result<Vec<ProductRank>, AppError> {
        let limit = limit.clamp(1, 100);

        // V15 P0-B10：注入数据范围过滤（LEFT JOIN sales_orders s）
        let (scope_sql, scope_values) = self.scope_sql("s", 2);

        let sql = format!(
            r#"
            SELECT
                p.id as product_id,
                p.name as product_name,
                p.code as product_code,
                COALESCE(pc.name, '未分类') as category,
                COALESCE(SUM(si.total_amount), 0) as total_amount,
                COALESCE(SUM(si.quantity), 0) as quantity,
                COUNT(DISTINCT si.order_id) as order_count
            FROM products p
            LEFT JOIN sales_order_items si ON si.product_id = p.id
            LEFT JOIN sales_orders s ON s.id = si.order_id
                AND s.status NOT IN ('CANCELLED', 'DRAFT')
            WHERE 1=1 {scope_sql}
            GROUP BY p.id, p.name, p.code, category
            ORDER BY total_amount DESC
            LIMIT $1
            "#,
            scope_sql = scope_sql,
        );

        let mut values = vec![limit.into()];
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );

        let rows = ProductRankRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;

        let results = rows
            .into_iter()
            .map(|r| ProductRank {
                product_id: r.product_id as i64,
                product_name: r.product_name,
                product_code: r.product_code,
                category: r.category.unwrap_or("未分类".to_string()),
                total_amount: dec_to_f64(r.total_amount),
                quantity: dec_to_f64(r.quantity),
                order_count: r.order_count.unwrap_or(0),
            })
            .collect();

        Ok(results)
    }

    /// 按区域聚合销售
    ///
    /// 按客户所在省份聚合销售额、订单数、客户数。
    pub async fn sales_by_region(&self) -> Result<Vec<RegionStat>, AppError> {
        // V15 P0-B10：注入数据范围过滤（sales_orders 别名为 s）
        let (scope_sql, scope_values) = self.scope_sql("s", 1);

        let sql = format!(
            r#"
            SELECT
                COALESCE(c.province, '未知') as region,
                COALESCE(SUM(s.total_amount), 0) as total_amount,
                COUNT(s.id) as order_count,
                COUNT(DISTINCT c.id) as customer_count
            FROM sales_orders s
            LEFT JOIN customers c ON c.id = s.customer_id
            WHERE s.status NOT IN ('CANCELLED', 'DRAFT')
            {scope_sql}
            GROUP BY region
            ORDER BY total_amount DESC
            "#,
            scope_sql = scope_sql,
        );

        let mut values: Vec<sea_orm::Value> = Vec::new();
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );

        let rows = RegionStatRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;

        let results = rows
            .into_iter()
            .map(|r| RegionStat {
                region: r.region,
                total_amount: dec_to_f64(r.total_amount),
                order_count: r.order_count.unwrap_or(0),
                customer_count: r.customer_count.unwrap_or(0),
            })
            .collect();

        Ok(results)
    }

    /// 按品类聚合销售
    ///
    /// 按 product_categories.name 聚合销售额，percentage = 品类销售额 / 全部销售额 * 100。
    pub async fn sales_by_category(&self) -> Result<Vec<CategoryStat>, AppError> {
        // V15 P0-B10：注入数据范围过滤（sales_orders 别名为 s）
        let (scope_sql, scope_values) = self.scope_sql("s", 1);

        let sql = format!(
            r#"
            SELECT
                COALESCE(pc.name, '未分类') as category,
                COALESCE(SUM(si.total_amount), 0) as total_amount
            FROM sales_order_items si
            INNER JOIN sales_orders s ON s.id = si.order_id
                AND s.status NOT IN ('CANCELLED', 'DRAFT')
            LEFT JOIN products p ON p.id = si.product_id
            LEFT JOIN product_categories pc ON pc.id = p.category_id
            WHERE 1=1 {scope_sql}
            GROUP BY category
            ORDER BY total_amount DESC
            "#,
            scope_sql = scope_sql,
        );

        let mut values: Vec<sea_orm::Value> = Vec::new();
        values.extend(scope_values);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );

        let rows = CategoryStatRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;

        let total: f64 = rows
            .iter()
            .map(|r| dec_to_f64(r.total_amount))
            .sum();

        let results = rows
            .into_iter()
            .map(|r| {
                let amount = dec_to_f64(r.total_amount);
                let percentage = if total > 0.0 {
                    amount / total * 100.0
                } else {
                    0.0
                };
                CategoryStat {
                    category: r.category,
                    total_amount: amount,
                    percentage,
                }
            })
            .collect();

        Ok(results)
    }

    /// 销售趋势（时间序列）
    ///
    /// 返回最近 N 天的按日聚合销售数据。
    pub async fn sales_trend(&self, days: i32) -> Result<Vec<TimeSeriesPoint>, AppError> {
        let days = days.clamp(1, 365);
        let end = chrono::Local::now().date_naive();
        let start = end - chrono::Duration::days(days as i64);
        self.sales_by_time(start, end, "day").await
    }
}
