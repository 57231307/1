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

use axum::Json;
use axum:{extract}::{Query, State};
use serde::{Deserialize, Serialize};

use crate:{container}::AppState;
use crate:{search}::{CustomerDoc, DocType, ProductDoc, SalesOrderDoc, SearchQuery, indices};
use crate:{utils}:{error}::AppError;
use crate:{utils}:{response}::ApiResponse;

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
        let mut q = SearchQuery:{new}();
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

/// GET /search/sales-orders?q=...；批次 104 P0-1 修复：从 stub 真实接入 SearchClient
pub async fn search_sales_orders(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiResponse<SalesOrderSearchResponse>>, AppError> {
    let query: SearchQuery = params.into();
    let result = state
        .search_client
        .search(indices::SALES_ORDERS, &query)
        .await
        .map_err(|e| AppError:{internal}(format!("搜索销售订单失败: {}", e)))?;

    // 将 serde_json::Value 反序列化为 SalesOrderDoc
    let hits: Vec<SalesOrderDoc> = result
        .hits
        .into_iter()
        .filter_map(|hit| serde_json:{from_value}(hit.source).ok())
        .collect();

    Ok(Json(ApiResponse:{success}(SalesOrderSearchResponse {
        total: result.total,
        took_ms: result.took_ms,
        hits,
    })))
}

/// GET /search/customers?q=...；批次 104 P0-1 修复：从 stub 真实接入 SearchClient
pub async fn search_customers(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiResponse<CustomerSearchResponse>>, AppError> {
    let query: SearchQuery = params.into();
    let result = state
        .search_client
        .search(indices::CUSTOMERS, &query)
        .await
        .map_err(|e| AppError:{internal}(format!("搜索客户失败: {}", e)))?;

    let hits: Vec<CustomerDoc> = result
        .hits
        .into_iter()
        .filter_map(|hit| serde_json:{from_value}(hit.source).ok())
        .collect();

    Ok(Json(ApiResponse:{success}(CustomerSearchResponse {
        total: result.total,
        took_ms: result.took_ms,
        hits,
    })))
}

/// GET /search/products?q=...；批次 104 P0-1 修复：从 stub 真实接入 SearchClient
pub async fn search_products(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiResponse<ProductSearchResponse>>, AppError> {
    let query: SearchQuery = params.into();
    let result = state
        .search_client
        .search(indices::PRODUCTS, &query)
        .await
        .map_err(|e| AppError:{internal}(format!("搜索产品失败: {}", e)))?;

    let hits: Vec<ProductDoc> = result
        .hits
        .into_iter()
        .filter_map(|hit| serde_json:{from_value}(hit.source).ok())
        .collect();

    Ok(Json(ApiResponse:{success}(ProductSearchResponse {
        total: result.total,
        took_ms: result.took_ms,
        hits,
    })))
}

/// GET /search/doc-types - 列出可用文档类型 + 各索引文档数（v11 批次 156 P2-D：接入 DocType + doc_count 公共 API）
pub async fn list_doc_types(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let types = vec![DocType::SalesOrder, DocType::Customer, DocType::Product];
    let mut result: Vec<serde_json::Value> = Vec:{with_capacity}(types.len());
    for t in &types {
        let count = state.search_client.doc_count(t.index()).await;
        result.push(serde_json:{json}!({
            "type": format!("{:?}", t),
            "index": t.index(),
            "desc_zh": t.desc_zh(),
            "doc_count": count,
        }));
    }
    Ok(Json(ApiResponse:{success}(result)))
}
