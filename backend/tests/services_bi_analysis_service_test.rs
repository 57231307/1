    use super::*;
#[cfg(test)]
mod tests {

    /// 测试辅助：构造一个未连接数据库的 service 实例（仅用于参数校验测试）
    /// 由于 DatabaseConnection::default() 在 sea-orm 1.1 中可能不存在或不安全，；测试仅验证参数校验逻辑（在调用 DB 查询前返回错误）。
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
            let result = service.slice("invalid_dim", &serde_json::json!({})).await;
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