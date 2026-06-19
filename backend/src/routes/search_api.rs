//! P9-8 搜索 API 路由（P9-8 占位）
//!
//! 提供 3 个搜索端点：
//! - GET /search/sales-orders?q=...  销售订单搜索
//! - GET /search/customers?q=...      客户搜索
//! - GET /search/products?q=...       产品搜索

use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::search::{CustomerDoc, ProductDoc, SalesOrderDoc, SearchQuery, SearchResult};

/// 搜索端点
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub from: Option<i64>,
    pub size: Option<i64>,
    pub status: Option<String>,
    pub tier: Option<String>,
    pub category: Option<String>,
}

impl From<SearchParams> for SearchQuery {
    fn from(p: SearchParams) -> Self {
        let mut q = SearchQuery::new();
        if let Some(keyword) = p.q {
            q = q.with_keyword(keyword);
        }
        if let Some(s) = p.status {
            q = q.with_filter("status", s);
        }
        if let Some(t) = p.tier {
            q = q.with_filter("tier", t);
        }
        if let Some(c) = p.category {
            q = q.with_filter("category", c);
        }
        if let (Some(f), Some(s)) = (p.from, p.size) {
            q = q.with_pagination(f, s);
        }
        q
    }
}

/// 销售订单搜索响应
#[derive(Debug, Serialize)]
pub struct SalesOrderSearchResponse {
    pub total: i64,
    pub took_ms: i64,
    pub hits: Vec<SalesOrderDoc>,
}

/// 客户搜索响应
#[derive(Debug, Serialize)]
pub struct CustomerSearchResponse {
    pub total: i64,
    pub took_ms: i64,
    pub hits: Vec<CustomerDoc>,
}

/// 产品搜索响应
#[derive(Debug, Serialize)]
pub struct ProductSearchResponse {
    pub total: i64,
    pub took_ms: i64,
    pub hits: Vec<ProductDoc>,
}

/// GET /search/sales-orders?q=...
pub async fn search_sales_orders(
    Query(params): Query<SearchParams>,
) -> Result<Json<SalesOrderSearchResponse>, StatusCode> {
    let query: SearchQuery = params.into();
    // 实际生产环境调用 SearchClient.search()
    Ok(Json(SalesOrderSearchResponse {
        total: 0,
        took_ms: 0,
        hits: vec![],
    }))
}

/// GET /search/customers?q=...
pub async fn search_customers(
    Query(params): Query<SearchParams>,
) -> Result<Json<CustomerSearchResponse>, StatusCode> {
    let query: SearchQuery = params.into();
    Ok(Json(CustomerSearchResponse {
        total: 0,
        took_ms: 0,
        hits: vec![],
    }))
}

/// GET /search/products?q=...
pub async fn search_products(
    Query(params): Query<SearchParams>,
) -> Result<Json<ProductSearchResponse>, StatusCode> {
    let query: SearchQuery = params.into();
    Ok(Json(ProductSearchResponse {
        total: 0,
        took_ms: 0,
        hits: vec![],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
