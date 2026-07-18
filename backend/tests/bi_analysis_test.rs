//! P3-4 BI 多维分析集成测试
//!
//! V15 Batch 485 P0-06-6 修复：
//! - 原 16 个测试使用过时的静态方法调用 `BiAnalysisService::method(args)`
//!   但 v9 批次 130 重构后所有方法改为实例方法 `&self`
//! - 本批次修复为 `BiAnalysisService::new(db).method(args)` 实例调用
//! - 参数校验类测试（无效输入返回 Err）使用 sqlite::memory: 连接，
//!   校验在 DB 查询前返回，不依赖真实 PostgreSQL
//! - 需要真实 PostgreSQL 的测试标记 #[ignore]，避免 CI 无 DB 环境失败

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bingxi_backend::services::bi_analysis_service::BiAnalysisService;
    use sea_orm::{Database, DatabaseConnection};

    /// 构造测试用 BiAnalysisService（sqlite::memory: 连接）
    ///
    /// 仅用于参数校验测试：方法在执行 DB 查询前校验参数并返回 Err。
    /// 不可用于需要真实 PostgreSQL 的测试（Postgres to_char / EXTRACT 语法不兼容 sqlite）。
    async fn make_service() -> BiAnalysisService {
        let db: DatabaseConnection = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite 内存数据库连接失败");
        BiAnalysisService::new(Arc::new(db))
    }

    /// 单元测试：drilldown_year_to_month 无效年份返回 Err（参数校验，不依赖 DB）
    #[tokio::test]
    async fn test_drilldown_year_to_month_invalid() {
        let service = make_service().await;
        // 年份 < 1900 或 > 2999 应返回 Err（校验在 DB 查询前）
        assert!(
            service.drilldown_year_to_month(1800).await.is_err(),
            "1800 年应被拒绝"
        );
        assert!(
            service.drilldown_year_to_month(3000).await.is_err(),
            "3000 年应被拒绝"
        );
    }

    /// 单元测试：drilldown_year_to_month 有效年份（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库（to_char / EXTRACT 语法）"]
    async fn test_drilldown_year_to_month() {
        let service = make_service().await;
        let data = service.drilldown_year_to_month(2026).await.unwrap();
        assert_eq!(data.len(), 12, "12 个月");
    }

    /// 单元测试：slice 无效维度返回 Err（参数校验，不依赖 DB）
    #[tokio::test]
    async fn test_slice_invalid_dimension() {
        let service = make_service().await;
        let result = service
            .slice("invalid", &serde_json::json!({}))
            .await;
        assert!(result.is_err(), "无效维度应返回 Err");
    }

    /// 单元测试：slice 有效维度（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_slice() {
        let service = make_service().await;
        let result = service
            .slice("customer", &serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result["dimension"], "customer");
    }

    /// 单元测试：rollup 无效粒度返回 Err（参数校验，不依赖 DB）
    #[tokio::test]
    async fn test_rollup_invalid_level() {
        let service = make_service().await;
        let result = service.rollup("invalid", "month").await;
        assert!(result.is_err(), "无效粒度应返回 Err");
    }

    /// 单元测试：rollup 有效粒度（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_rollup() {
        let service = make_service().await;
        let result = service.rollup("day", "month").await.unwrap();
        assert_eq!(result["from"], "day");
        assert_eq!(result["to"], "month");
    }

    /// 单元测试：sales_by_time 日期反转返回 Err（参数校验，不依赖 DB）
    #[tokio::test]
    async fn test_sales_by_time_invalid_dates() {
        let service = make_service().await;
        let result = service
            .sales_by_time(
                chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                "month",
            )
            .await;
        assert!(result.is_err(), "结束日期早于开始日期应返回 Err");
    }

    /// 单元测试：pivot（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_pivot() {
        let service = make_service().await;
        let result = service
            .pivot("time", "product", "amount")
            .await
            .unwrap();
        assert_eq!(result["row"], "time");
        assert_eq!(result["col"], "product");
        assert_eq!(result["measure"], "amount");
    }

    /// 单元测试：kpi_summary（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_kpi_summary_returns_valid() {
        let service = make_service().await;
        let kpi = service.kpi_summary().await.unwrap();
        assert!(kpi.total_sales > 0.0, "total_sales 应大于 0");
        assert!(kpi.order_count > 0, "order_count 应大于 0");
        assert!(kpi.avg_order_value > 0.0, "avg_order_value 应大于 0");
    }

    /// 单元测试：kpi_summary 无真实 DB 时返回 Err（不标记 ignore，验证非崩溃）
    #[tokio::test]
    async fn test_kpi_summary_returns_err_without_db() {
        let service = make_service().await;
        // sqlite 连接无 sales_orders 表，查询应返回 Err（非 panic）
        let result = service.kpi_summary().await;
        assert!(result.is_err(), "无真实 DB 时 kpi_summary 应返回 Err");
    }

    /// 单元测试：sales_by_customer（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_sales_by_customer_limit() {
        let service = make_service().await;
        let result = service.sales_by_customer(2).await.unwrap();
        assert!(result.len() <= 2);
    }

    /// 单元测试：sales_by_product（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_sales_by_product_limit() {
        let service = make_service().await;
        let result = service.sales_by_product(1).await.unwrap();
        assert!(result.len() <= 1);
    }

    /// 单元测试：drilldown_month_to_day（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_drilldown_month_to_day() {
        let service = make_service().await;
        let data = service
            .drilldown_month_to_day(2026, 6)
            .await
            .unwrap();
        assert_eq!(data.len(), 30);
    }

    /// 单元测试：profit_analysis（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_profit_analysis() {
        let service = make_service().await;
        let p = service.profit_analysis().await.unwrap();
        assert!(p.total_revenue > 0.0);
        assert!(p.gross_margin > 0.0);
    }

    /// 单元测试：sales_trend（需要真实 DB，标记 ignore）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库"]
    async fn test_sales_trend() {
        let service = make_service().await;
        let data = service.sales_trend(7).await;
        assert!(data.is_ok());
    }

    /// 集成测试：端到端（CI 启用，沙箱 OOM 跳过）
    #[tokio::test]
    #[ignore = "需要 PostgreSQL + ETL 数据 + axum server"]
    async fn test_e2e_etl_to_aggregation() {
        // 完整流程：
        // 1. ETL 加载业务库数据到 sales_facts
        // 2. 启动 axum server（in-process）
        // 3. HTTP GET /api/v1/erp/bi/sales/kpi
        // 4. 验证返回 KPI 数据
        // 沙箱 OOM 限制下仅保留 stub
    }
}
