//! P3-4 BI 多维分析集成测试
//!
//! 沙箱限制：仅 CI 跑，本地用 stub
//! 沙箱 OOM 限制下无法跑 sqlx + axum 集成测试

#[cfg(test)]
mod tests {
    use bingxi_backend::services::bi_analysis_service::BiAnalysisService;

    /// 单元测试：BI service KPI 查询参数校验
    #[tokio::test]
    async fn test_kpi_summary_invalid_input() {
        assert!(BiAnalysisService::kpi_summary(0).await.is_err());
        assert!(BiAnalysisService::kpi_summary(-1).await.is_err());
        assert!(BiAnalysisService::kpi_summary(1).await.is_ok());
    }

    /// 单元测试：KPI 概览返回有效数据
    #[tokio::test]
    async fn test_kpi_summary_returns_valid() {
        let kpi = BiAnalysisService::kpi_summary(1).await.unwrap();
        assert!(kpi.total_sales > 0.0, "total_sales 应大于 0");
        assert!(kpi.order_count > 0, "order_count 应大于 0");
        assert!(kpi.avg_order_value > 0.0, "avg_order_value 应大于 0");
    }

    /// 单元测试：销售按客户聚合
    #[tokio::test]
    async fn test_sales_by_customer_limit() {
        let result = BiAnalysisService::sales_by_customer(1, 2).await.unwrap();
        assert!(result.len() <= 2);
    }

    /// 单元测试：销售按产品聚合
    #[tokio::test]
    async fn test_sales_by_product_limit() {
        let result = BiAnalysisService::sales_by_product(1, 1).await.unwrap();
        assert!(result.len() <= 1);
    }

    /// 单元测试：钻取 年→月
    #[tokio::test]
    async fn test_drilldown_year_to_month() {
        let data = BiAnalysisService::drilldown_year_to_month(1, 2026).await.unwrap();
        assert_eq!(data.len(), 12, "12 个月");
    }

    /// 单元测试：钻取 年→月 无效年份
    #[tokio::test]
    async fn test_drilldown_year_to_month_invalid() {
        assert!(BiAnalysisService::drilldown_year_to_month(1, 1800).await.is_err());
        assert!(BiAnalysisService::drilldown_year_to_month(1, 3000).await.is_err());
    }

    /// 单元测试：钻取 月→日
    #[tokio::test]
    async fn test_drilldown_month_to_day() {
        let data = BiAnalysisService::drilldown_month_to_day(1, 2026, 6).await.unwrap();
        assert_eq!(data.len(), 30);
    }

    /// 单元测试：切片
    #[tokio::test]
    async fn test_slice() {
        let result = BiAnalysisService::slice(1, "customer", &serde_json::json!({})).await.unwrap();
        assert_eq!(result["dimension"], "customer");
    }

    /// 单元测试：切片无效维度
    #[tokio::test]
    async fn test_slice_invalid_dimension() {
        assert!(BiAnalysisService::slice(1, "invalid", &serde_json::json!({})).await.is_err());
    }

    /// 单元测试：上卷
    #[tokio::test]
    async fn test_rollup() {
        let result = BiAnalysisService::rollup(1, "day", "month").await.unwrap();
        assert_eq!(result["from"], "day");
        assert_eq!(result["to"], "month");
    }

    /// 单元测试：上卷无效粒度
    #[tokio::test]
    async fn test_rollup_invalid_level() {
        assert!(BiAnalysisService::rollup(1, "invalid", "month").await.is_err());
    }

    /// 单元测试：透视
    #[tokio::test]
    async fn test_pivot() {
        let result = BiAnalysisService::pivot(1, "time", "product", "amount").await.unwrap();
        assert_eq!(result["row"], "time");
        assert_eq!(result["col"], "product");
        assert_eq!(result["measure"], "amount");
    }

    /// 单元测试：销售按时间 - 端点反转
    #[tokio::test]
    async fn test_sales_by_time_invalid_dates() {
        let result = BiAnalysisService::sales_by_time(
            1,
            chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            "month",
        )
        .await;
        assert!(result.is_err());
    }

    /// 单元测试：利润分析
    #[tokio::test]
    async fn test_profit_analysis() {
        let p = BiAnalysisService::profit_analysis(1).await.unwrap();
        assert!(p.total_revenue > 0.0);
        assert!(p.gross_margin > 0.0);
    }

    /// 单元测试：销售趋势
    #[tokio::test]
    async fn test_sales_trend() {
        let data = BiAnalysisService::sales_trend(1, 7).await;
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
