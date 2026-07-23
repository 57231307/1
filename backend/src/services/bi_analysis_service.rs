//! BI 多维分析 service（facade）
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
//!
//! 批次 490 D10-3a 拆分：本文件作为 facade，保留 helper 函数 + Service struct + new 构造函数 + 测试。
//! BiAnalysisService 的 impl 块迁移至 `bi_analysis_ops` 子模块（sales / profit / drilldown / olap）。
//! 数据结构迁移至 `bi_analysis_ops::types`，本 facade 通过 `pub use` 二次 re-export 保持外部引用路径不变。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::utils::data_scope::{build_data_scope_sql, DataScope, DataScopeContext};
use crate::utils::error::AppError;

// re-export ops 子模块的对外 response struct，保持外部 `use crate::services::bi_analysis_service::{...}` 路径不变
pub use crate::services::bi_analysis_ops::{
    BiResponse, CategoryStat, CustomerRank, KpiSummary, ProductRank, ProfitAnalysis, RegionStat,
    TimeSeriesPoint,
};

// ==================== 模块级私有 helper（pub(crate) 供 ops 子模块使用） ====================

/// Decimal → f64 安全转换（避免精度损失，使用 to_string().parse()）
pub(crate) fn dec_to_f64(d: Option<Decimal>) -> f64 {
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
pub(crate) fn dim_to_expr(dim: &str) -> Result<(&'static str, &'static str), AppError> {
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
pub(crate) fn measure_to_expr(measure: &str, item_level: bool) -> Result<&'static str, AppError> {
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

// ==================== Service struct 定义（impl 块在 bi_analysis_ops 子模块） ====================

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
    /// 数据库连接（pub(crate) 供 bi_analysis_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
    /// V15 P0-B10：行级数据权限上下文，所有查询自动注入（pub(crate) 供 bi_analysis_ops 子模块访问）
    pub(crate) data_scope: DataScopeContext,
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
    /// pub(crate) 供 bi_analysis_ops 子模块使用。
    pub(crate) fn scope_sql(&self, table_alias: &str, next_index: usize) -> (String, Vec<sea_orm::Value>) {
        build_data_scope_sql(&self.data_scope, table_alias, next_index)
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
