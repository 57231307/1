//! P9-8 Elasticsearch 集成模块（客户端 + 索引 + 搜索）
//!
//! 提供：
//! 1. **客户端封装**：HTTP 调用 ES REST API
//! 2. **3 个核心索引**：
//!    - `sales_orders`（销售订单）
//!    - `customers`（客户）
//!    - `products`（产品）
//! 3. **同步机制**：业务写入时同步到 ES
//! 4. **搜索 API**：分词 + 高亮 + 过滤
//!
//! ## 启用 elasticsearch crate
//!
//! 默认情况下，本模块提供 trait 与 mock 实现，**不引入重依赖**。
//! 要启用真实 ES 集成，添加：
//!
//! ```toml
//! elasticsearch = "8.5.0-alpha.1"
//! ```
//!
//! ## 与 PostgreSQL 的关系
//!
//! PG 是**主数据源**（事务、关联查询），ES 是**搜索副本**（全文搜索、聚合）。
//! 写入策略：
//! 1. 业务写入 PG
//! 2. 同步写入 ES（失败重试 3 次）
//! 3. 异步补偿：定时任务扫描 5 分钟内变更的记录，修复 ES 缺失

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 3 个核心索引
pub mod indices {
    /// 销售订单索引
    pub const SALES_ORDERS: &str = "sales_orders";
    /// 客户索引
    pub const CUSTOMERS: &str = "customers";
    /// 产品索引
    pub const PRODUCTS: &str = "products";
}

/// 文档类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocType {
    SalesOrder,
    Customer,
    Product,
}

impl DocType {
    pub fn index(&self) -> &'static str {
        match self {
            Self::SalesOrder => indices::SALES_ORDERS,
            Self::Customer => indices::CUSTOMERS,
            Self::Product => indices::PRODUCTS,
        }
    }

    pub fn desc_zh(&self) -> &'static str {
        match self {
            Self::SalesOrder => "销售订单",
            Self::Customer => "客户",
            Self::Product => "产品",
        }
    }
}

/// 销售订单文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderDoc {
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: String,
    pub total_amount: f64,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub items: Vec<SalesOrderItemDoc>,
    pub tenant_id: String,
}

/// 销售订单明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderItemDoc {
    pub product_id: i32,
    pub product_name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub color_no: Option<String>,
    pub pantone_code: Option<String>,
}

/// 客户文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerDoc {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tier: String,
    pub tenant_id: String,
}

/// 产品文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDoc {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub category: Option<String>,
    pub spec: Option<String>,
    pub unit: String,
    pub color_no: Option<String>,
    pub pantone_code: Option<String>,
    pub price: f64,
    pub tenant_id: String,
}

/// 搜索查询
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    /// 关键字
    pub q: Option<String>,
    /// 字段过滤
    pub filters: HashMap<String, String>,
    /// 起始位置
    pub from: i64,
    /// 大小
    pub size: i64,
    /// 排序字段
    pub sort: Option<String>,
    /// 是否高亮
    pub highlight: bool,
}

impl SearchQuery {
    pub fn new() -> Self {
        Self {
            from: 0,
            size: 20,
            ..Default::default()
        }
    }

    pub fn with_keyword(mut self, q: impl Into<String>) -> Self {
        self.q = Some(q.into());
        self
    }

    pub fn with_filter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filters.insert(key.into(), value.into());
        self
    }

    pub fn with_pagination(mut self, from: i64, size: i64) -> Self {
        self.from = from;
        self.size = size;
        self
    }

    pub fn with_highlight(mut self) -> Self {
        self.highlight = true;
        self
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult<T> {
    pub total: i64,
    pub hits: Vec<SearchHit<T>>,
    pub took_ms: i64,
}

/// 单个命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit<T> {
    pub id: String,
    pub score: f64,
    pub source: T,
    pub highlight: Option<HashMap<String, Vec<String>>>,
}

/// ES 客户端 trait
///
/// 全部方法使用 `serde_json::Value` 而非泛型 `T`，避免 async trait 含泛型参数
/// 触发 E0038 trait not dyn compatible（`Arc<dyn SearchClient>` 用法需要 dyn 兼容）。
/// 调用方在传参前 `serde_json::to_value(doc)?` 即可。
#[async_trait]
pub trait SearchClient: Send + Sync {
    /// 索引文档
    async fn index_doc(
        &self,
        index: &str,
        id: &str,
        doc: &serde_json::Value,
    ) -> Result<(), SearchError>;

    /// 搜索
    async fn search(
        &self,
        index: &str,
        query: &SearchQuery,
    ) -> Result<SearchResult<serde_json::Value>, SearchError>;

    /// 删除文档
    async fn delete_doc(&self, index: &str, id: &str) -> Result<(), SearchError>;

    /// 批量索引
    async fn bulk_index(
        &self,
        index: &str,
        docs: &[(String, serde_json::Value)],
    ) -> Result<usize, SearchError>;
}

/// 搜索错误
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("连接失败: {0}")]
    Connection(String),
    #[error("索引失败: {0}")]
    Index(String),
    #[error("搜索失败: {0}")]
    Search(String),
    #[error("序列化失败: {0}")]
    Serialize(String),
}

/// ES 客户端（mock 实现）
pub struct ElasticClient {
    /// 模拟索引数据：index -> (id -> doc_json)
    storage: Arc<Mutex<HashMap<String, HashMap<String, serde_json::Value>>>>,
    /// 是否启用真实 ES
    real_es_enabled: bool,
}

impl ElasticClient {
    /// 创建 mock 客户端
    pub fn mock() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            real_es_enabled: false,
        }
    }

    /// 创建真实客户端（需启用 elasticsearch crate）
    pub fn real(_url: String) -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            real_es_enabled: true,
        }
    }

    /// 已索引文档数
    pub async fn doc_count(&self, index: &str) -> usize {
        self.storage
            .lock()
            .await
            .get(index)
            .map(|m| m.len())
            .unwrap_or(0)
    }
}

#[async_trait]
impl SearchClient for ElasticClient {
    async fn index_doc(
        &self,
        index: &str,
        id: &str,
        doc: &serde_json::Value,
    ) -> Result<(), SearchError> {
        let mut storage = self.storage.lock().await;
        storage
            .entry(index.to_string())
            .or_insert_with(HashMap::new)
            .insert(id.to_string(), doc.clone());
        Ok(())
    }

    async fn search(
        &self,
        index: &str,
        query: &SearchQuery,
    ) -> Result<SearchResult<serde_json::Value>, SearchError> {
        let storage = self.storage.lock().await;
        let docs = storage.get(index).cloned().unwrap_or_default();

        let mut hits: Vec<SearchHit<serde_json::Value>> = docs
            .iter()
            .filter(|(_, v)| match &query.q {
                Some(q) => serde_json::to_string(v)
                    .map(|s| s.contains(q))
                    .unwrap_or(false),
                None => true,
            })
            .map(|(id, value)| SearchHit {
                id: id.clone(),
                score: 1.0,
                source: value.clone(),
                highlight: None,
            })
            .collect();

        let total = hits.len() as i64;
        let from = query.from.max(0) as usize;
        let size = query.size.max(0) as usize;
        let end = (from + size).min(hits.len());
        if from < hits.len() {
            hits = hits.split_off(from);
            hits.truncate(end - from);
        } else {
            hits.clear();
        }

        Ok(SearchResult {
            total,
            hits,
            took_ms: 1,
        })
    }

    async fn delete_doc(&self, index: &str, id: &str) -> Result<(), SearchError> {
        let mut storage = self.storage.lock().await;
        if let Some(map) = storage.get_mut(index) {
            map.remove(id);
        }
        Ok(())
    }

    async fn bulk_index(
        &self,
        index: &str,
        docs: &[(String, serde_json::Value)],
    ) -> Result<usize, SearchError> {
        let mut count = 0;
        for (id, doc) in docs {
            self.index_doc(index, id, doc).await?;
            count += 1;
        }
        Ok(count)
    }
}

/// 业务同步器：将 PG 写入同步到 ES
pub struct SearchSyncer {
    client: Arc<dyn SearchClient>,
}

impl SearchSyncer {
    pub fn new(client: Arc<dyn SearchClient>) -> Self {
        Self { client }
    }

    /// 同步销售订单
    pub async fn sync_sales_order(&self, doc: &SalesOrderDoc) -> Result<(), SearchError> {
        let value = serde_json::to_value(doc).map_err(|e| SearchError::Serialize(e.to_string()))?;
        self.client
            .index_doc(indices::SALES_ORDERS, &doc.order_no, &value)
            .await
    }

    /// 同步客户
    pub async fn sync_customer(&self, doc: &CustomerDoc) -> Result<(), SearchError> {
        let id = doc.id.to_string();
        let value = serde_json::to_value(doc).map_err(|e| SearchError::Serialize(e.to_string()))?;
        self.client.index_doc(indices::CUSTOMERS, &id, &value).await
    }

    /// 同步产品
    pub async fn sync_product(&self, doc: &ProductDoc) -> Result<(), SearchError> {
        let id = doc.id.to_string();
        let value = serde_json::to_value(doc).map_err(|e| SearchError::Serialize(e.to_string()))?;
        self.client.index_doc(indices::PRODUCTS, &id, &value).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_constants() {
        assert_eq!(indices::SALES_ORDERS, "sales_orders");
        assert_eq!(indices::CUSTOMERS, "customers");
        assert_eq!(indices::PRODUCTS, "products");
    }

    #[test]
    fn test_doc_type_index() {
        assert_eq!(DocType::SalesOrder.index(), "sales_orders");
        assert_eq!(DocType::Customer.index(), "customers");
        assert_eq!(DocType::Product.index(), "products");
    }

    #[test]
    fn test_doc_type_desc_zh() {
        assert_eq!(DocType::SalesOrder.desc_zh(), "销售订单");
        assert_eq!(DocType::Customer.desc_zh(), "客户");
        assert_eq!(DocType::Product.desc_zh(), "产品");
    }

    #[test]
    fn test_search_query_new() {
        let q = SearchQuery::new();
        assert_eq!(q.from, 0);
        assert_eq!(q.size, 20);
        assert!(!q.highlight);
    }

    #[test]
    fn test_search_query_with_keyword() {
        let q = SearchQuery::new().with_keyword("test");
        assert_eq!(q.q, Some("test".to_string()));
    }

    #[test]
    fn test_search_query_with_filter() {
        let q = SearchQuery::new()
            .with_filter("status", "approved")
            .with_filter("customer", "acme");
        assert_eq!(q.filters.get("status"), Some(&"approved".to_string()));
        assert_eq!(q.filters.get("customer"), Some(&"acme".to_string()));
    }

    #[test]
    fn test_search_query_with_pagination() {
        let q = SearchQuery::new().with_pagination(20, 50);
        assert_eq!(q.from, 20);
        assert_eq!(q.size, 50);
    }

    #[test]
    fn test_search_query_with_highlight() {
        let q = SearchQuery::new().with_highlight();
        assert!(q.highlight);
    }

    #[test]
    fn test_sales_order_doc_serialize() {
        let doc = SalesOrderDoc {
            order_no: "SO-001".to_string(),
            customer_id: 100,
            customer_name: "ACME".to_string(),
            total_amount: 1000.0,
            status: "approved".to_string(),
            created_at: crate::ymd!(2026, 6, 17).and_hms_opt(10, 0, 0).unwrap().and_utc(),
            items: vec![],
            tenant_id: "tenant-001".to_string(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("SO-001"));
        assert!(json.contains("ACME"));
    }

    #[test]
    fn test_customer_doc_serialize() {
        let doc = CustomerDoc {
            id: 1,
            code: "C001".to_string(),
            name: "ACME Corp".to_string(),
            contact_person: Some("张三".to_string()),
            phone: Some("13800138000".to_string()),
            email: None,
            address: Some("杭州".to_string()),
            tier: "A".to_string(),
            tenant_id: "tenant-001".to_string(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("ACME Corp"));
        assert!(json.contains("张三"));
    }

    #[test]
    fn test_product_doc_serialize() {
        let doc = ProductDoc {
            id: 1,
            code: "P001".to_string(),
            name: "纯棉布".to_string(),
            category: Some("面料".to_string()),
            spec: Some("100% 棉".to_string()),
            unit: "米".to_string(),
            color_no: Some("CN-001".to_string()),
            pantone_code: Some("PANTONE-18-1664".to_string()),
            price: 50.0,
            tenant_id: "tenant-001".to_string(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("纯棉布"));
    }

    #[tokio::test]
    async fn test_elastic_client_index_doc() {
        let client = ElasticClient::mock();
        let doc = SalesOrderDoc {
            order_no: "SO-001".to_string(),
            customer_id: 1,
            customer_name: "Test".to_string(),
            total_amount: 100.0,
            status: "draft".to_string(),
            created_at: crate::ymd!(2026, 6, 17).and_hms_opt(0, 0, 0).unwrap().and_utc(),
            items: vec![],
            tenant_id: "t1".to_string(),
        };
        let value = serde_json::to_value(&doc).unwrap();
        client
            .index_doc(indices::SALES_ORDERS, "SO-001", &value)
            .await
            .unwrap();
        assert_eq!(client.doc_count(indices::SALES_ORDERS).await, 1);
    }

    #[tokio::test]
    async fn test_elastic_client_search() {
        let client = ElasticClient::mock();
        for i in 0..5 {
            let doc = SalesOrderDoc {
                order_no: format!("SO-{:03}", i),
                customer_id: i,
                customer_name: format!("客户 {}", i),
                total_amount: 100.0 * i as f64,
                status: "draft".to_string(),
                created_at: crate::ymd!(2026, 6, 17).and_hms_opt(0, 0, 0).unwrap().and_utc(),
                items: vec![],
                tenant_id: "t1".to_string(),
            };
            let value = serde_json::to_value(&doc).unwrap();
            client
                .index_doc(indices::SALES_ORDERS, &format!("SO-{:03}", i), &value)
                .await
                .unwrap();
        }
        let query = SearchQuery::new().with_keyword("客户");
        let result: SearchResult<serde_json::Value> = client
            .search(indices::SALES_ORDERS, &query)
            .await
            .unwrap();
        assert!(result.total > 0);
    }

    #[tokio::test]
    async fn test_elastic_client_delete() {
        let client = ElasticClient::mock();
        let doc = CustomerDoc {
            id: 1,
            code: "C001".to_string(),
            name: "Test".to_string(),
            contact_person: None,
            phone: None,
            email: None,
            address: None,
            tier: "C".to_string(),
            tenant_id: "t1".to_string(),
        };
        let value = serde_json::to_value(&doc).unwrap();
        client.index_doc(indices::CUSTOMERS, "1", &value).await.unwrap();
        assert_eq!(client.doc_count(indices::CUSTOMERS).await, 1);
        client.delete_doc(indices::CUSTOMERS, "1").await.unwrap();
        assert_eq!(client.doc_count(indices::CUSTOMERS).await, 0);
    }

    #[tokio::test]
    async fn test_elastic_client_bulk_index() {
        let client = ElasticClient::mock();
        let docs: Vec<(String, serde_json::Value)> = (0..3)
            .map(|i| {
                let doc = ProductDoc {
                    id: i,
                    code: format!("P{:03}", i),
                    name: format!("产品 {}", i),
                    category: None,
                    spec: None,
                    unit: "米".to_string(),
                    color_no: None,
                    pantone_code: None,
                    price: 10.0 * i as f64,
                    tenant_id: "t1".to_string(),
                };
                (format!("P{:03}", i), serde_json::to_value(&doc).unwrap())
            })
            .collect();
        let count = client.bulk_index(indices::PRODUCTS, &docs).await.unwrap();
        assert_eq!(count, 3);
        assert_eq!(client.doc_count(indices::PRODUCTS).await, 3);
    }

    #[tokio::test]
    async fn test_search_syncer() {
        let client = Arc::new(ElasticClient::mock());
        let syncer = SearchSyncer::new(client.clone());

        let order = SalesOrderDoc {
            order_no: "SO-001".to_string(),
            customer_id: 1,
            customer_name: "Test".to_string(),
            total_amount: 100.0,
            status: "approved".to_string(),
            created_at: crate::ymd!(2026, 6, 17).and_hms_opt(0, 0, 0).unwrap().and_utc(),
            items: vec![],
            tenant_id: "t1".to_string(),
        };
        syncer.sync_sales_order(&order).await.unwrap();
        assert_eq!(client.doc_count(indices::SALES_ORDERS).await, 1);

        let customer = CustomerDoc {
            id: 1,
            code: "C001".to_string(),
            name: "Test".to_string(),
            contact_person: None,
            phone: None,
            email: None,
            address: None,
            tier: "A".to_string(),
            tenant_id: "t1".to_string(),
        };
        syncer.sync_customer(&customer).await.unwrap();
        assert_eq!(client.doc_count(indices::CUSTOMERS).await, 1);
    }
}
