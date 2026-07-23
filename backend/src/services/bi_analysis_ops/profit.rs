//! 利润分析与 KPI Service impl 子模块（bi_analysis_ops/profit）
//!
//! 批次 490 D10-3a 拆分：从原 `bi_analysis_service.rs` L708-941 迁移。
//! 包含 BiAnalysisService 的 2 个公开方法 + 3 个私有 helper：
//! - profit_analysis / kpi_summary（2 公开方法）
//! - fetch_current_kpi / fetch_yoy_growth / fetch_mom_growth（3 私有 helper）
//!
//! 业务规则：
//! - 利润 = 销售额 - 成本，成本 = SUM(sales_order_items.quantity * products.cost_price)
//! - KPI 同比增长率 = (本月销售额 - 去年同月销售额) / 去年同月销售额 * 100
//! - KPI 环比增长率 = (本月销售额 - 上月销售额) / 上月销售额 * 100
//! - V15 P0-B10：所有查询注入行级数据权限过滤

use chrono::Datelike;
use sea_orm::{FromQueryResult, Statement};

use crate::services::bi_analysis_ops::types::{
    KpiCurrentMetrics, KpiRow, KpiSummary, MoMRow, ProfitAnalysis, ProfitRow, YoYRow,
};
use crate::services::bi_analysis_service::{dec_to_f64, BiAnalysisService};
use crate::utils::error::AppError;

impl BiAnalysisService {
    /// 利润分析
    ///
    /// 聚合全部有效订单的销售额、成本、利润。
    /// 利润 = 销售额 - 成本，成本 = SUM(sales_order_items.quantity * products.cost_price)。
    pub async fn profit_analysis(&self) -> Result<ProfitAnalysis, AppError> {
        // V15 P0-B10：注入数据范围过滤（sales_orders 别名为 s）
        let (scope_sql, scope_values) = self.scope_sql("s", 1);

        let sql = format!(
            r#"
            SELECT
                COALESCE(SUM(s.total_amount), 0) as total_revenue,
                COALESCE(SUM((
                    SELECT SUM(si.quantity * COALESCE(p.cost_price, 0))
                    FROM sales_order_items si
                    LEFT JOIN products p ON p.id = si.product_id
                    WHERE si.order_id = s.id
                )), 0) as total_cost,
                COUNT(*) as order_count
            FROM sales_orders s
            WHERE s.status NOT IN ('CANCELLED', 'DRAFT')
            {scope_sql}
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

        let row: Option<ProfitRow> = ProfitRow::find_by_statement(stmt)
            .one(&*self.db)
            .await?;

        let row = row.unwrap_or(ProfitRow {
            total_revenue: None,
            total_cost: None,
            order_count: None,
        });

        let total_revenue = dec_to_f64(row.total_revenue);
        let total_cost = dec_to_f64(row.total_cost);
        let total_profit = total_revenue - total_cost;
        let order_count = row.order_count.unwrap_or(0);
        let gross_margin = if total_revenue > 0.0 {
            total_profit / total_revenue * 100.0
        } else {
            0.0
        };
        let avg_order_value = if order_count > 0 {
            total_revenue / order_count as f64
        } else {
            0.0
        };

        Ok(ProfitAnalysis {
            total_revenue,
            total_cost,
            total_profit,
            gross_margin,
            order_count,
            avg_order_value,
        })
    }

    /// 核心 KPI：聚合总销售额/订单数/客户数/客单价，并计算同比/环比增长率
    pub async fn kpi_summary(&self) -> Result<KpiSummary, AppError> {
        let now = chrono::Utc::now();
        let current = self.fetch_current_kpi().await?;
        let yoy_growth = self.fetch_yoy_growth(now).await?;
        let mom_growth = self.fetch_mom_growth(now).await?;
        Ok(KpiSummary {
            total_sales: current.total_sales,
            order_count: current.order_count,
            customer_count: current.customer_count,
            avg_order_value: current.avg_order_value,
            yoy_growth,
            mom_growth,
        })
    }

    /// 查询当前周期 KPI（总销售额/订单数/客户数/客单价），注入数据范围过滤
    async fn fetch_current_kpi(&self) -> Result<KpiCurrentMetrics, AppError> {
        let (scope_sql, scope_values) = self.scope_sql("", 1);
        let sql = format!(
            r#"
            SELECT
                COALESCE(SUM(total_amount), 0) as total_sales,
                COUNT(*) as order_count,
                COUNT(DISTINCT customer_id) as customer_count
            FROM sales_orders
            WHERE status NOT IN ('CANCELLED', 'DRAFT')
            {scope_sql}
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
        let row: KpiRow = KpiRow::find_by_statement(stmt)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::internal("KPI 查询失败"))?;
        let total_sales = dec_to_f64(row.total_sales);
        let order_count = row.order_count.unwrap_or(0);
        let customer_count = row.customer_count.unwrap_or(0);
        let avg_order_value = if order_count > 0 {
            total_sales / order_count as f64
        } else {
            0.0
        };
        Ok(KpiCurrentMetrics {
            total_sales,
            order_count,
            customer_count,
            avg_order_value,
        })
    }

    /// 查询同比增长率（本月 vs 去年同月），注入数据范围过滤
    async fn fetch_yoy_growth(&self, now: chrono::DateTime<chrono::Utc>) -> Result<f64, AppError> {
        let this_year = now.format("%Y").to_string();
        let last_year = (this_year.parse::<i32>().unwrap_or(2026) - 1).to_string();
        let month = now.format("%m").to_string();
        // 同比 2 个子查询，每个注入数据范围过滤（参数 $3/$4 起）
        let (scope_sql_1, scope_values_1) = self.scope_sql("", 3);
        let (scope_sql_2, scope_values_2) = self.scope_sql("", 4);
        let sql = format!(
            r#"
            SELECT
                (SELECT COALESCE(SUM(total_amount), 0) FROM sales_orders
                 WHERE status NOT IN ('CANCELLED', 'DRAFT')
                   AND to_char(order_date, 'YYYY') = $1
                   AND to_char(order_date, 'MM') = $2
                   {scope_sql_1}) as this_year,
                (SELECT COALESCE(SUM(total_amount), 0) FROM sales_orders
                 WHERE status NOT IN ('CANCELLED', 'DRAFT')
                   AND to_char(order_date, 'YYYY') = $3
                   AND to_char(order_date, 'MM') = $2
                   {scope_sql_2}) as last_year
            "#,
            scope_sql_1 = scope_sql_1,
            scope_sql_2 = scope_sql_2,
        );
        let mut values = vec![this_year.into(), month.into(), last_year.into()];
        values.extend(scope_values_1);
        values.extend(scope_values_2);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );
        let row: Option<YoYRow> = YoYRow::find_by_statement(stmt)
            .one(&*self.db)
            .await?;
        let (this_year_sales, last_year_sales) = if let Some(r) = row {
            (dec_to_f64(r.this_year), dec_to_f64(r.last_year))
        } else {
            (0.0, 0.0)
        };
        let growth = if last_year_sales > 0.0 {
            (this_year_sales - last_year_sales) / last_year_sales * 100.0
        } else {
            0.0
        };
        Ok(growth)
    }

    /// 查询环比增长率（本月 vs 上月），注入数据范围过滤
    async fn fetch_mom_growth(&self, now: chrono::DateTime<chrono::Utc>) -> Result<f64, AppError> {
        let this_year = now.format("%Y").to_string();
        let month = now.format("%m").to_string();
        let last_month = if now.month() == 1 { 12 } else { now.month() - 1 };
        let last_month_year = if now.month() == 1 {
            (this_year.parse::<i32>().unwrap_or(2026) - 1).to_string()
        } else {
            this_year.clone()
        };
        // 环比 2 个子查询，每个注入数据范围过滤（参数 $5/$6 起）
        let (scope_sql_1, scope_values_1) = self.scope_sql("", 5);
        let (scope_sql_2, scope_values_2) = self.scope_sql("", 6);
        let sql = format!(
            r#"
            SELECT
                (SELECT COALESCE(SUM(total_amount), 0) FROM sales_orders
                 WHERE status NOT IN ('CANCELLED', 'DRAFT')
                   AND to_char(order_date, 'YYYY') = $1
                   AND to_char(order_date, 'MM') = $2
                   {scope_sql_1}) as this_month,
                (SELECT COALESCE(SUM(total_amount), 0) FROM sales_orders
                 WHERE status NOT IN ('CANCELLED', 'DRAFT')
                   AND to_char(order_date, 'YYYY') = $3
                   AND to_char(order_date, 'MM') = $4
                   {scope_sql_2}) as last_month
            "#,
            scope_sql_1 = scope_sql_1,
            scope_sql_2 = scope_sql_2,
        );
        let mut values = vec![
            this_year.into(),
            month.into(),
            last_month_year.into(),
            format!("{:02}", last_month).into(),
        ];
        values.extend(scope_values_1);
        values.extend(scope_values_2);
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            values,
        );
        let row: Option<MoMRow> = MoMRow::find_by_statement(stmt)
            .one(&*self.db)
            .await?;
        let (this_month_sales, last_month_sales) = if let Some(r) = row {
            (dec_to_f64(r.this_month), dec_to_f64(r.last_month))
        } else {
            (0.0, 0.0)
        };
        let growth = if last_month_sales > 0.0 {
            (this_month_sales - last_month_sales) / last_month_sales * 100.0
        } else {
            0.0
        };
        Ok(growth)
    }
}
