#[cfg(test)]
mod tests {
    use bingxi_backend::routes::search_api::SearchParams;
    use bingxi_backend::search::{indices, SearchQuery};

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

    #[test]
    fn test_search_indices_constants() {
        // 验证索引常量
        assert_eq!(indices::SALES_ORDERS, "sales_orders");
        assert_eq!(indices::CUSTOMERS, "customers");
        assert_eq!(indices::PRODUCTS, "products");
    }

    #[test]
    fn test_search_query_with_keyword() {
        let query = SearchQuery::new().with_keyword("test");
        assert_eq!(query.q, Some("test".to_string()));
        assert_eq!(query.from, 0);
        assert_eq!(query.size, 20);
    }

    #[test]
    fn test_search_query_with_filter() {
        let query = SearchQuery::new().with_filter("status", "approved");
        assert_eq!(query.filters.get("status"), Some(&"approved".to_string()));
    }
}
