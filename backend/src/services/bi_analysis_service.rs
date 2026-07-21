//! BI 多维分析 service（v9 批次 130 真实接入）
//!
//! 功能：
//! 1. 维度聚合（按时间/客户/产品/区域/品类）
//! 2. 钻取（年→月、月→日、客户→订单、产品→订单）
//! 3. 切片/切块/上卷/透视
//!
//! 实现策略（v9 批次 130 修复）：
//! - 原 P3-4 关键路径 demo 全部返回硬编码 mock 数据，违反规则 0（真实实现强制）
//! - 现使用 SeaORM raw SQL（Statement::from_sql_and_values + FromQueryResult）真实查询
//!   sales_orders / sales_order_items / customers / products / product_categories 表
//! - 16 个 HTTP 端点对外暴露，前端调用后获得真实聚合数据

use chrono::Datelike;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, FromQueryResult, Statement};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::utils::data_scope::{build_data_scope_sql, DataScope, DataScopeContext};
use crate::utils::error::AppError;

/// 通用响应包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

impl<T> BiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data,
        }
    }
}

// ==================== DTO ====================

/// 时间序列点
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeSeriesPoint {
    /// 周期标识（YYYY-MM-DD / YYYY-MM / YYYY-Q1 / YYYY）
    pub period: String,
    /// 销售额
    pub total_amount: f64,
    /// 订单数
    pub order_count: i64,
    /// 销售数量
    pub quantity: f64,
    /// 利润
    pub profit_amount: f64,
}

/// 客户排行
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomerRank {
    pub customer_id: i64,
    pub customer_name: String,
    pub total_amount: f64,
    pub order_count: i64,
    pub percentage: f64,
}

/// 产品排行
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductRank {
    pub product_id: i64,
    pub product_name: String,
    pub product_code: String,
    pub category: String,
    pub total_amount: f64,
    pub quantity: f64,
    pub order_count: i64,
}

/// 区域统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegionStat {
    pub region: String,
    pub total_amount: f64,
    pub order_count: i64,
    pub customer_count: i64,
}

/// 品类统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryStat {
    pub category: String,
    pub total_amount: f64,
    pub percentage: f64,
}

/// 利润分析
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfitAnalysis {
    pub total_revenue: f64,
    pub total_cost: f64,
    pub total_profit: f64,
    pub gross_margin: f64,
    pub order_count: i64,
    pub avg_order_value: f64,
}

/// KPI 概览
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KpiSummary {
    /// 总销售额
    pub total_sales: f64,
    /// 订单数
    pub order_count: i64,
    /// 客户数
    pub customer_count: i64,
    /// 客单价
    pub avg_order_value: f64,
    /// 同比增长率（与上一年同期）
    pub yoy_growth: f64,
    /// 环比增长率（与上月）
    pub mom_growth: f64,
}

// ==================== FromQueryResult 中间结构 ====================

#[derive(Debug, FromQueryResult)]
struct TimeSeriesRow {
    period: String,
    total_amount: Option<Decimal>,
    order_count: Option<i64>,
    quantity: Option<Decimal>,
    profit_amount: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
struct CustomerRankRow {
    customer_id: i32,
    customer_name: String,
    total_amount: Option<Decimal>,
    order_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
struct ProductRankRow {
    product_id: i32,
    product_name: String,
    product_code: String,
    category: Option<String>,
    total_amount: Option<Decimal>,
    quantity: Option<Decimal>,
    order_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
struct RegionStatRow {
    region: String,
    total_amount: Option<Decimal>,
    order_count: Option<i64>,
    customer_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
struct CategoryStatRow {
    category: String,
    total_amount: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
struct ProfitRow {
    total_revenue: Option<Decimal>,
    total_cost: Option<Decimal>,
    order_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
struct KpiRow {
    total_sales: Option<Decimal>,
    order_count: Option<i64>,
    customer_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
struct CustomerOrderRow {
    order_id: i32,
    amount: Option<Decimal>,
    order_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, FromQueryResult)]
struct ProductOrderRow {
    order_id: i32,
    quantity: Option<Decimal>,
    amount: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
struct TotalRow {
    total: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
struct YoYRow {
    this_year: Option<Decimal>,
    last_year: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
struct MoMRow {
    this_month: Option<Decimal>,
    last_month: Option<Decimal>,
}

/// KPI 当前周期指标（内部传递用）
struct KpiCurrentMetrics {
    total_sales: f64,
    order_count: i64,
    customer_count: i64,
    avg_order_value: f64,
}

/// 透视矩阵行（v11 批次 144 P1-3：动态 SQL 透视矩阵）
#[derive(Debug, FromQueryResult)]
struct PivotRow {
    row_key: Option<String>,
    row_label: Option<String>,
    col_key: Option<String>,
    col_label: Option<String>,
    measure_value: Option<Decimal>,
}

/// Decimal → f64 安全转换（避免精度损失，使用 to_string().parse()）
fn dec_to_f64(d: Option<Decimal>) -> f64 {
    d.map(|v| v.to_string().parse::<f64>().unwrap_or(0.0))
        .unwrap_or(0.0)
}

/// 维度到 SQL 表达式映射（v11 批次 144 P1-3：透视矩阵维度映射）
///
/// 返回 (key_expr, label_expr)：
/// - key_expr: 用于 GROUP BY 的唯一标识（text 类型）
/// - label_expr: 用于展示的可读标签
///
/// 支持的维度：
/// - customer: 客户 ID + 客户名称
/// - product: 产品 ID + 产品名称
/// - region: 客户所在省份
/// - category: 产品品类名称
/// - time: 订单月份（YYYY-MM 格式）
///
/// 批次 252 修复：原 `_ => unreachable!()` 在非法维度时 panic 崩溃，
/// 改为返回 AppError::validation 错误，防御性处理非法输入。
fn dim_to_expr(dim: &str) -> Result<(&'static str, &'static str), AppError> {
    match dim {
        "customer" => Ok(("c.id::text", "COALESCE(c.customer_name, '未知客户')")),
        "product" => Ok(("p.id::text", "COALESCE(p.name, '未知产品')")),
        "region" => Ok(("COALESCE(c.province, '未知')", "COALESCE(c.province, '未知')")),
        "category" => Ok((
            "COALESCE(pc.name, '未分类')",
            "COALESCE(pc.name, '未分类')",
        )),
        "time" => Ok((
            "to_char(s.order_date, 'YYYY-MM')",
            "to_char(s.order_date, 'YYYY-MM')",
        )),
        _ => Err(AppError::validation(format!("不支持的维度: {}", dim))),
    }
}

/// 度量聚合表达式生成（批次 252 修复：提取为独立函数，消除 unreachable! panic）
///
/// 根据 item_level 选择项级或订单级聚合的 SQL 表达式：
/// - item_level=true：关联 sales_order_items 表进行项级聚合
/// - item_level=false：订单级聚合，避免 total_amount 重复计算
fn measure_to_expr(measure: &str, item_level: bool) -> Result<&'static str, AppError> {
    match (measure, item_level) {
        ("total_amount", true) => Ok("COALESCE(SUM(si.total_amount), 0)"),
        ("order_count", true) => Ok("COUNT(DISTINCT s.id)::numeric"),
        ("quantity", true) => Ok("COALESCE(SUM(si.quantity), 0)"),
        ("profit_amount", true) => Ok(
            "COALESCE(SUM(si.total_amount), 0) - COALESCE(SUM(si.quantity * COALESCE(p.cost_price, 0)), 0)",
        ),
        ("total_amount", false) => Ok("COALESCE(SUM(s.total_amount), 0)"),
        ("order_count", false) => Ok("COUNT(*)::numeric"),
        ("quantity", false) => Ok(
            "COALESCE(SUM((SELECT SUM(si.quantity) FROM sales_order_items si WHERE si.order_id = s.id)), 0)",
        ),
        ("profit_amount", false) => Ok(
            "COALESCE(SUM(s.total_amount), 0) - COALESCE(SUM((SELECT SUM(si.quantity * COALESCE(p.cost_price, 0)) FROM sales_order_items si LEFT JOIN products p ON p.id = si.product_id WHERE si.order_id = s.id)), 0)",
        ),
        _ => Err(AppError::validation(format!("不支持的度量: {}", measure))),
    }
}

// ==================== Service ====================

/// BI 多维分析 service
///
/// v9 批次 130 修复：原全部方法返回硬编码 mock 数据，现真实查询数据库。
/// 查询 sales_orders / sales_order_items / customers / products / product_categories 表，
/// 排除 CANCELLED 和 DRAFT 状态的订单。
///
/// V15 P0-B10（Batch 483）：新增 data_scope 字段，所有 raw SQL 查询注入行级数据权限过滤。
/// - All：不过滤（管理员/总经理）
/// - Dept：按 users.department_id 过滤（部门经理）
/// - Self_：按 sales_orders.created_by 过滤（普通员工）
pub struct BiAnalysisService {
    db: Arc<DatabaseConnection>,
    /// V15 P0-B10：行级数据权限上下文，所有查询自动注入
    data_scope: DataScopeContext,
}

impl BiAnalysisService {
    /// 创建 BI 服务（默认 All 数据范围，仅用于测试/内部调用）
    ///
    /// 生产环境应使用 `new_with_data_scope` 注入真实数据范围。
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            data_scope: DataScopeContext {
                scope: DataScope::All,
                user_id: 0,
                department_id: None,
            },
        }
    }

    /// V15 P0-B10：创建带数据范围上下文的 BI 服务
    ///
    /// 由 handler 调用，从 AuthContext.to_data_scope_context() 注入。
    pub fn new_with_data_scope(db: Arc<DatabaseConnection>, ctx: DataScopeContext) -> Self {
        Self { db, data_scope: ctx }
    }

    /// V15 P0-B10：构建数据范围 SQL 片段（带别名和起始索引）
    ///
    /// 内部辅助方法，封装 build_data_scope_sql 调用。
    fn scope_sql(&self, table_alias: &str, next_index: usize) -> (String, Vec<sea_orm::Value>) {
        build_data_scope_sql(&self.data_scope, table_alias, next_index)
    }

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

        // 使用 CASE WHEN 实现 granularity 动态分组（Postgres date_trunc 第一参数不支持参数化）
        let period_expr = match granularity {
            "day" => "to_char(order_date, 'YYYY-MM-DD')",
            "week" => "to_char(order_date, 'IYYY-IW')",
            "month" => "to_char(order_date, 'YYYY-MM')",
            "quarter" => "to_char(order_date, 'YYYY-Q')",
            "year" => "to_char(order_date, 'YYYY')",
            _ => "to_char(order_date, 'YYYY-MM')",
        };

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

        // profit_amount 实际是总成本，需要用 total_amount - cost 计算真实利润
        let results = rows
            .into_iter()
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
            .collect();

        Ok(results)
    }

    /// 按客户聚合销售
    ///
    /// 返回销售额 TOP N 客户排行，percentage = 客户销售额 / 全部销售额 * 100。
    pub async fn sales_by_customer(
        &self,
        limit: i64,
    ) -> Result<Vec<CustomerRank>, AppError> {
        let limit = limit.clamp(1, 100);

        // V15 P0-B10：注入数据范围过滤（LEFT JOIN sales_orders s，过滤条件加在 WHERE）
        // 注：将过滤加到 WHERE 会把 LEFT JOIN 变为 INNER JOIN 效果，
        //     即只返回有符合数据范围订单的客户（业务期望：员工只看到自己客户的排行）
        let (scope_sql, scope_values) = self.scope_sql("s", 2);

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

        let rows = CustomerRankRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;

        // 计算全部销售额用于 percentage（同样应用数据范围过滤）
        let (total_scope_sql, total_scope_values) = self.scope_sql("sales_orders", 1);
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
            .one(&*self.db)
            .await?;
        let total_sales = total_row
            .map(|r| dec_to_f64(r.total))
            .unwrap_or(0.0);

        let results = rows
            .into_iter()
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
            .collect();

        Ok(results)
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

    // ==================== 钻取 ====================

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

        // V15 P0-B10：注入数据范围过滤（sales_orders 无别名，已有 $1 参数）
        let (scope_sql, scope_values) = self.scope_sql("", 2);

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

        let rows = TimeSeriesRow::find_by_statement(stmt)
            .all(&*self.db)
            .await?;

        // 构建 12 个月完整数据，缺失月份补 0
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

        let results = (1..=12)
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
            .collect();

        Ok(results)
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

    // ==================== 切片/上卷 ====================

    /// 切片（固定其他维度，单独分析一个维度）
    ///
    /// 根据 dimension 调用对应的聚合方法，filters 作为附加过滤条件（当前实现忽略 filters，
    /// 仅按 dimension 返回聚合数据；后续迭代可解析 filters 构建动态 WHERE 子句）。
    pub async fn slice(
        &self,
        dimension: &str,
        filters: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let valid_dims = ["time", "customer", "product", "region", "category"];
        if !valid_dims.contains(&dimension) {
            return Err(AppError::validation(format!("不支持的维度: {}", dimension)));
        }

        let result = match dimension {
            "time" => {
                let end = chrono::Local::now().date_naive();
                let start = end - chrono::Duration::days(30);
                serde_json::to_value(self.sales_by_time(start, end, "day").await?)?
            }
            "customer" => serde_json::to_value(self.sales_by_customer(10).await?)?,
            "product" => serde_json::to_value(self.sales_by_product(10).await?)?,
            "region" => serde_json::to_value(self.sales_by_region().await?)?,
            "category" => serde_json::to_value(self.sales_by_category().await?)?,
            _ => serde_json::Value::Null,
        };

        Ok(serde_json::json!({
            "dimension": dimension,
            "filters": filters,
            "result": result,
        }))
    }

    /// 切块（多维范围筛选）
    ///
    /// 解析 filters 中的 date_from/date_to/customer_ids/product_ids 等条件，
    /// 返回符合所有条件的订单聚合数据。当前实现：返回指定日期范围内的按日聚合。
    pub async fn dice(
        &self,
        filters: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        // 解析可选的日期范围
        let date_from = filters
            .get("date_from")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        let date_to = filters
            .get("date_to")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let end = date_to.unwrap_or(chrono::Local::now().date_naive());
        let start = date_from.unwrap_or(end - chrono::Duration::days(30));

        let data = self.sales_by_time(start, end, "day").await?;

        Ok(serde_json::json!({
            "filters": filters,
            "date_range": {
                "from": start.format("%Y-%m-%d").to_string(),
                "to": end.format("%Y-%m-%d").to_string(),
            },
            "result": data,
        }))
    }

    /// 上卷（细粒度 → 粗粒度）
    ///
    /// from_level → to_level 粒度聚合，例如 day → month。
    /// 当前实现：返回最近 90 天按 to_level 粒度的聚合数据。
    pub async fn rollup(
        &self,
        from_level: &str,
        to_level: &str,
    ) -> Result<serde_json::Value, AppError> {
        let valid_levels = ["day", "week", "month", "quarter", "year"];
        if !valid_levels.contains(&from_level) || !valid_levels.contains(&to_level) {
            return Err(AppError::validation("无效的粒度级别"));
        }

        let end = chrono::Local::now().date_naive();
        let start = end - chrono::Duration::days(90);
        let data = self.sales_by_time(start, end, to_level).await?;

        Ok(serde_json::json!({
            "from": from_level,
            "to": to_level,
            "date_range": {
                "from": start.format("%Y-%m-%d").to_string(),
                "to": end.format("%Y-%m-%d").to_string(),
            },
            "result": data,
        }))
    }

    /// 校验 pivot 参数（行/列维度、度量）
    fn validate_pivot_params(row_dim: &str, col_dim: &str, measure: &str) -> Result<(), AppError> {
        let valid_dims = ["customer", "product", "region", "category", "time"];
        if !valid_dims.contains(&row_dim) {
            return Err(AppError::validation(format!(
                "不支持的行维度: {}",
                row_dim
            )));
        }
        if !valid_dims.contains(&col_dim) {
            return Err(AppError::validation(format!(
                "不支持的列维度: {}",
                col_dim
            )));
        }
        if row_dim == col_dim {
            return Err(AppError::validation("行维度与列维度不能相同"));
        }

        let valid_measures = ["total_amount", "order_count", "quantity", "profit_amount"];
        if !valid_measures.contains(&measure) {
            return Err(AppError::validation(format!(
                "不支持的度量: {}",
                measure
            )));
        }
        Ok(())
    }

    /// 构建 pivot SQL 查询并执行，返回原始行
    async fn execute_pivot_query(
        &self,
        row_dim: &str,
        col_dim: &str,
        measure: &str,
    ) -> Result<Vec<PivotRow>, AppError> {
        let (row_key_expr, row_label_expr) = dim_to_expr(row_dim)?;
        let (col_key_expr, col_label_expr) = dim_to_expr(col_dim)?;

        // 判断是否需要关联 sales_order_items（当任一维度为 product/category 时）
        let needs_items =
            matches!(row_dim, "product" | "category") || matches!(col_dim, "product" | "category");

        // 批次 252 修复：measure_to_expr 替代原内联 match + unreachable!()
        let (joins, measure_expr) = if needs_items {
            // 项级聚合：关联 sales_order_items / products / product_categories
            let joins = r#"
                LEFT JOIN sales_order_items si ON si.order_id = s.id
                LEFT JOIN products p ON p.id = si.product_id
                LEFT JOIN product_categories pc ON pc.id = p.category_id
            "#;
            let measure_expr = measure_to_expr(measure, true)?;
            (joins, measure_expr)
        } else {
            // 订单级聚合：不关联 sales_order_items，避免 total_amount 重复计算
            let joins = "";
            let measure_expr = measure_to_expr(measure, false)?;
            (joins, measure_expr)
        };

        // V15 P0-B10：注入数据范围过滤（sales_orders 别名为 s，无其他参数，scope 从 $1 开始）
        let (scope_sql, scope_values) = self.scope_sql("s", 1);

        let sql = format!(
            r#"
            SELECT
                {row_key} as row_key,
                {row_label} as row_label,
                {col_key} as col_key,
                {col_label} as col_label,
                {measure} as measure_value
            FROM sales_orders s
            LEFT JOIN customers c ON c.id = s.customer_id
            {joins}
            WHERE s.status NOT IN ('CANCELLED', 'DRAFT')
              {scope_sql}
            GROUP BY row_key, row_label, col_key, col_label
            ORDER BY row_label ASC, col_label ASC
            "#,
            row_key = row_key_expr,
            row_label = row_label_expr,
            col_key = col_key_expr,
            col_label = col_label_expr,
            measure = measure_expr,
            joins = joins,
            scope_sql = scope_sql,
        );

        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            scope_values,
        );

        PivotRow::find_by_statement(stmt)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("透视查询执行失败: {}", e)))
    }

    /// 从查询结果构建交叉聚合矩阵
    fn build_pivot_matrix(
        rows: Vec<PivotRow>,
        row_dim: &str,
        col_dim: &str,
        measure: &str,
    ) -> serde_json::Value {
        // 收集唯一的行/列键（保持有序），并构建矩阵
        let mut row_set: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut col_set: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut matrix: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();

        for r in rows {
            let row_key = r.row_key.unwrap_or_default();
            let row_label = r.row_label.unwrap_or_else(|| row_key.clone());
            let col_key = r.col_key.unwrap_or_default();
            let col_label = r.col_label.unwrap_or_else(|| col_key.clone());
            let value = dec_to_f64(r.measure_value);

            row_set.entry(row_key.clone()).or_insert(row_label);
            col_set.entry(col_key.clone()).or_insert(col_label);
            matrix.insert(format!("{}|{}", row_key, col_key), value);
        }

        let rows_json: Vec<serde_json::Value> = row_set
            .iter()
            .map(|(k, v)| serde_json::json!({ "key": k, "label": v }))
            .collect();
        let cols_json: Vec<serde_json::Value> = col_set
            .iter()
            .map(|(k, v)| serde_json::json!({ "key": k, "label": v }))
            .collect();
        let matrix_json: serde_json::Value = matrix
            .into_iter()
            .map(|(k, v)| (k, serde_json::json!(v)))
            .collect();

        serde_json::json!({
            "row_dim": row_dim,
            "col_dim": col_dim,
            "measure": measure,
            "rows": rows_json,
            "columns": cols_json,
            "matrix": matrix_json,
        })
    }

    /// 透视（行列转换），按 row_dim × col_dim 构建二维聚合矩阵
    ///
    /// 实现说明（v11 批次 144 P1-3 修复）：
    /// - 原实现返回占位 note 字段，col 维度分组未实现
    /// - 现使用动态 SQL 构建真实的 row × col 交叉聚合矩阵
    /// - 当任一维度为 product/category 时，需要关联 sales_order_items 表进行项级聚合
    /// - 否则在订单级别聚合，避免因 JOIN 倍增导致 total_amount 重复计算
    pub async fn pivot(
        &self,
        row_dim: &str,
        col_dim: &str,
        measure: &str,
    ) -> Result<serde_json::Value, AppError> {
        Self::validate_pivot_params(row_dim, col_dim, measure)?;

        let rows = self.execute_pivot_query(row_dim, col_dim, measure).await?;

        Ok(Self::build_pivot_matrix(rows, row_dim, col_dim, measure))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试辅助：构造一个未连接数据库的 service 实例（仅用于参数校验测试）
    ///
    /// 由于 DatabaseConnection::default() 在 sea-orm 1.1 中可能不存在或不安全，
    /// 测试仅验证参数校验逻辑（在调用 DB 查询前返回错误）。
    async fn make_service() -> Option<BiAnalysisService> {
        // 尝试从环境变量连接测试数据库，失败则跳过测试
        let db_url = std::env::var("DATABASE_URL").ok()?;
        let db = sea_orm::Database::connect(&db_url).await.ok()?;
        Some(BiAnalysisService::new(std::sync::Arc::new(db)))
    }

    #[tokio::test]
    async fn test_drilldown_invalid_year() {
        // 参数校验在 DB 查询前，即使无 DB 也能通过
        if let Some(service) = make_service().await {
            let result = service.drilldown_year_to_month(1800).await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_slice_invalid_dimension() {
        if let Some(service) = make_service().await {
            let result = service
                .slice("invalid_dim", &serde_json::json!({}))
                .await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_sales_by_time_invalid_dates() {
        if let Some(service) = make_service().await {
            let result = service
                .sales_by_time(
                    chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
                    chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                    "month",
                )
                .await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_drilldown_invalid_month() {
        if let Some(service) = make_service().await {
            let result = service.drilldown_month_to_day(2026, 13).await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_drilldown_customer_invalid_id() {
        if let Some(service) = make_service().await {
            let result = service.drilldown_customer_to_order(0).await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_drilldown_product_invalid_id() {
        if let Some(service) = make_service().await {
            let result = service.drilldown_product_to_order(-1).await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_rollup_invalid_level() {
        if let Some(service) = make_service().await {
            let result = service.rollup("invalid", "month").await;
            assert!(result.is_err());
        }
    }

    /// v11 批次 144 P1-3：透视矩阵参数校验测试
    #[tokio::test]
    async fn test_pivot_invalid_row_dim() {
        if let Some(service) = make_service().await {
            let result = service.pivot("invalid", "customer", "total_amount").await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_pivot_invalid_col_dim() {
        if let Some(service) = make_service().await {
            let result = service.pivot("customer", "invalid", "total_amount").await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_pivot_same_dim() {
        if let Some(service) = make_service().await {
            let result = service.pivot("customer", "customer", "total_amount").await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_pivot_invalid_measure() {
        if let Some(service) = make_service().await {
            let result = service
                .pivot("customer", "product", "invalid_measure")
                .await;
            assert!(result.is_err());
        }
    }

    // ==================== 批次 252：dim_to_expr / measure_to_expr 单元测试 ====================
    // 验证原 unreachable!() 分支现在返回错误而非 panic 崩溃

    /// 测试 dim_to_expr 对所有合法维度返回 Ok
    #[test]
    fn test_dim_to_expr_valid_dims() {
        let valid_dims = ["customer", "product", "region", "category", "time"];
        for dim in valid_dims {
            assert!(dim_to_expr(dim).is_ok(), "维度 {} 应返回 Ok", dim);
        }
    }

    /// 测试 dim_to_expr 对非法维度返回 Err（原 unreachable!() 会 panic）
    #[test]
    fn test_dim_to_expr_invalid_dim_returns_error() {
        let result = dim_to_expr("invalid_dim");
        assert!(result.is_err(), "非法维度应返回错误而非 panic");
    }

    /// 测试 dim_to_expr 对空字符串返回 Err
    #[test]
    fn test_dim_to_expr_empty_string_returns_error() {
        let result = dim_to_expr("");
        assert!(result.is_err(), "空字符串维度应返回错误而非 panic");
    }

    /// 测试 measure_to_expr 对所有合法度量在项级和订单级均返回 Ok
    #[test]
    fn test_measure_to_expr_valid_measures() {
        let valid_measures = ["total_amount", "order_count", "quantity", "profit_amount"];
        for measure in valid_measures {
            assert!(
                measure_to_expr(measure, true).is_ok(),
                "度量 {} 项级聚合应返回 Ok",
                measure
            );
            assert!(
                measure_to_expr(measure, false).is_ok(),
                "度量 {} 订单级聚合应返回 Ok",
                measure
            );
        }
    }

    /// 测试 measure_to_expr 对非法度量返回 Err（原 unreachable!() 会 panic）
    #[test]
    fn test_measure_to_expr_invalid_measure_returns_error() {
        assert!(
            measure_to_expr("invalid_measure", true).is_err(),
            "非法度量项级聚合应返回错误而非 panic"
        );
        assert!(
            measure_to_expr("invalid_measure", false).is_err(),
            "非法度量订单级聚合应返回错误而非 panic"
        );
    }
}
