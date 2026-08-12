#[cfg(test)]
mod tests {
    // 批次 415：index_doc/search 是 SearchClient trait 方法，测试需导入
    use bingxi_backend::search::SearchClient;

    #[test]
    fn test_search_params_to_query() {
        let params = SearchParams {
            q: Some("ACME".to_string()),
            from: Some(0),
            size: Some(50),
            status: Some("approved".to_string()),
            tier: None,
            category: None,
        };
        let query: SearchQuery = params.into();
        assert_eq!(query.q, Some("ACME".to_string()));
        assert_eq!(query.size, 50);
        assert_eq!(query.filters.get("status"), Some(&"approved".to_string()));
    }

    #[test]
    fn test_search_params_empty() {
        let params = SearchParams {
            q: None,
            from: None,
            size: None,
            status: None,
            tier: None,
            category: None,
        };
        let query: SearchQuery = params.into();
        assert_eq!(query.from, 0);
        assert_eq!(query.size, 20);
    }

    /// 批次 104 P0-1 修复：新增端到端搜索测试；验证 search_sales_orders 真实调用 SearchClient（mock 实现）并返回正确结果。
    #[tokio::test]
    async fn test_search_sales_orders_with_mock_client() {
        use std::sync::Arc;

        // 构造 mock 客户端并预置数据
        let client = Arc::new(crate::search::ElasticClient::mock());
        let doc = serde_json::json!({
            "order_no": "SO-2026-001",
            "customer_id": 1,
            "customer_name": "ACME 公司",
            "total_amount": 10000.0,
            "status": "approved",
            "created_at": "2026-07-04T00:00:00Z",
            "items": []
        });
        client
            .index_doc(indices::SALES_ORDERS, "1", &doc)
            .await
            .expect("索引文档不应失败");

        // 构造 query 搜索 "ACME"
        let query = SearchQuery::new().with_keyword("ACME");
        let result = client
            .search(indices::SALES_ORDERS, &query)
            .await
            .expect("搜索不应失败");

        assert_eq!(result.total, 1);
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].id, "1");
    }
}