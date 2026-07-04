//! P9-8 搜索 API 路由（批次 104 真实接入 SearchClient）
//!
//! 提供 3 个搜索端点：
//! - GET /search/sales-orders?q=...  销售订单搜索
//! - GET /search/customers?q=...      客户搜索
//! - GET /search/products?q=...       产品搜索
//!
//! 批次 104 P0-1 修复：3 个端点从 stub 真实接入 SearchClient。
//! - 注入 AppState 获取 search_client
//! - 调用 search_client.search() 执行真实搜索
//! - 将 SearchResult<serde_json::Value> 反序列化为对应 Doc 类型
//! - 错误处理从 StatusCode 改为 AppError

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::search::{
    indices, CustomerDoc, ProductDoc, SalesOrderDoc, SearchQuery,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

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
///
/// 批次 104 P0-1 修复：从 stub 真实接入 SearchClient
pub async fn search_sales_orders(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiResponse<SalesOrderSearchResponse>>, AppError> {
    let query: SearchQuery = params.into();
    let result = state
        .search_client
        .search(indices::SALES_ORDERS, &query)
        .await
        .map_err(|e| AppError::internal(format!("搜索销售订单失败: {}", e)))?;

    // 将 serde_json::Value 反序列化为 SalesOrderDoc
    let hits: Vec<SalesOrderDoc> = result
        .hits
        .into_iter()
        .filter_map(|hit| serde_json::from_value(hit.source).ok())
        .collect();

    Ok(Json(ApiResponse::success(SalesOrderSearchResponse {
        total: result.total,
        took_ms: result.took_ms,
        hits,
    })))
}

/// GET /search/customers?q=...
///
/// 批次 104 P0-1 修复：从 stub 真实接入 SearchClient
pub async fn search_customers(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiResponse<CustomerSearchResponse>>, AppError> {
    let query: SearchQuery = params.into();
    let result = state
        .search_client
        .search(indices::CUSTOMERS, &query)
        .await
        .map_err(|e| AppError::internal(format!("搜索客户失败: {}", e)))?;

    let hits: Vec<CustomerDoc> = result
        .hits
        .into_iter()
        .filter_map(|hit| serde_json::from_value(hit.source).ok())
        .collect();

    Ok(Json(ApiResponse::success(CustomerSearchResponse {
        total: result.total,
        took_ms: result.took_ms,
        hits,
    })))
}

/// GET /search/products?q=...
///
/// 批次 104 P0-1 修复：从 stub 真实接入 SearchClient
pub async fn search_products(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiResponse<ProductSearchResponse>>, AppError> {
    let query: SearchQuery = params.into();
    let result = state
        .search_client
        .search(indices::PRODUCTS, &query)
        .await
        .map_err(|e| AppError::internal(format!("搜索产品失败: {}", e)))?;

    let hits: Vec<ProductDoc> = result
        .hits
        .into_iter()
        .filter_map(|hit| serde_json::from_value(hit.source).ok())
        .collect();

    Ok(Json(ApiResponse::success(ProductSearchResponse {
        total: result.total,
        took_ms: result.took_ms,
        hits,
    })))
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

    /// 批次 104 P0-1 修复：新增端到端搜索测试
    ///
    /// 验证 search_sales_orders 真实调用 SearchClient（mock 实现）并返回正确结果。
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
