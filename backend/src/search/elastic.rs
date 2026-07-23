//! ES 集成 facade：类型定义 + 构造函数 + 纯函数，业务方法迁移至 elastic_ops 子模块
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// SSRF 防护守卫：对 ES base_url 做协议白名单 + 主机名黑名单 + IP 黑名单 + DNS 解析校验
use crate::utils::ssrf_guard;

/// 3 个核心索引
pub mod indices {
    /// 销售订单索引
    pub const SALES_ORDERS: &str = "sales_orders";
    /// 客户索引
    pub const CUSTOMERS: &str = "customers";
    /// 产品索引
    pub const PRODUCTS: &str = "products";
}

/// 文档类型（通过 /search/doc-types 端点暴露公共 API）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocType {
    SalesOrder,
    Customer,
    Product,
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
    /// 是否高亮
    pub highlight: bool,
}

impl SearchQuery {
    /// 创建默认查询（from=0, size=20）
    pub fn new() -> Self {
        Self {
            from: 0,
            size: 20,
            ..Default::default()
        }
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

/// ES 客户端 trait（全部方法使用 serde_json::Value 避免泛型导致 dyn 不兼容）
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

    /// 已索引文档数
    async fn doc_count(&self, index: &str) -> usize;
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

/// ES 客户端（支持 mock 内存存储和 real reqwest 直连 ES REST API 两种模式）
pub struct ElasticClient {
    /// 客户端内部实现（mock 或 real），pub(crate) 供 elastic_ops 子模块访问
    pub(crate) inner: ClientInner,
}

/// 客户端内部实现枚举（pub(crate) 供 elastic_ops 子模块模式匹配）
pub(crate) enum ClientInner {
    /// Mock 模式：内存 HashMap 存储，用于开发/测试/CI 环境
    Mock(Arc<Mutex<HashMap<String, HashMap<String, serde_json::Value>>>>),
    /// Real 模式：reqwest 直连 ES REST API，用于生产环境
    Real {
        base_url: String,
        http: reqwest::Client,
    },
}

impl ElasticClient {
    /// 创建 mock 客户端
    pub fn mock() -> Self {
        Self {
            inner: ClientInner::Mock(Arc::new(Mutex::new(HashMap::new()))),
        }
    }

    /// 创建真实客户端（reqwest 直连 ES REST API，SSRF 校验失败则 fail-fast 退出）
    pub fn real(url: String) -> Self {
        Self::try_real(url).unwrap_or_else(|e| {
            eprintln!(
                "Elasticsearch URL SSRF 校验失败: {}，服务无法启动",
                e
            );
            std::process::exit(1);
        })
    }

    /// 创建真实客户端（可失败版本，用于测试和精细化错误处理）
    ///
    /// 与 [`real`] 的区别在于返回 Result，调用方可校验 URL 是否通过 SSRF 防护。
    /// 生产代码使用 [`real`] fail-fast，单元测试使用本方法验证 SSRF 拦截逻辑。
    pub fn try_real(url: String) -> Result<Self, crate::utils::error::AppError> {
        // SSRF 校验：解析 URL → 协议白名单 → 主机名黑名单 → IP 黑名单 → DNS 解析 + IP 校验
        // 返回 (host, safe_addrs)，调用方使用 resolve_to_addrs 固定连接 IP
        let (host, safe_addrs) = ssrf_guard::validate_url_and_resolve(&url)?;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none()) // SSRF 防护：禁止跟随重定向
            .resolve_to_addrs(&host, &safe_addrs) // SSRF 防护：固定连接 IP，消除 DNS Rebinding
            .build()
            .map_err(|e| {
                crate::utils::error::AppError::internal(format!(
                    "Elasticsearch HTTP 客户端构建失败: {}",
                    e
                ))
            })?;
        Ok(Self {
            inner: ClientInner::Real {
                base_url: url.trim_end_matches('/').to_string(),
                http,
            },
        })
    }
}

/// 确保所有索引存在（幂等创建，已存在则忽略）
///
/// 启动时调用，PUT 3 个索引的 mapping。ES 返回 400 表示索引已存在，视为成功（幂等）。
/// 独立 async 函数接受 base_url 参数，在 main.rs async 上下文中调用。
pub async fn ensure_indices(base_url: &str) -> Result<(), SearchError> {
    let base_url = base_url.trim_end_matches('/');

    // SSRF 校验：解析 URL → 协议白名单 → 主机名黑名单 → IP 黑名单 → DNS 解析 + IP 校验
    // 返回 (host, safe_addrs)，调用方使用 resolve_to_addrs 固定连接 IP
    let (host, safe_addrs) = ssrf_guard::validate_url_and_resolve(base_url).map_err(|e| {
        SearchError::Connection(format!("ES base_url SSRF 校验失败: {}", e))
    })?;

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none()) // SSRF 防护：禁止跟随重定向
        .resolve_to_addrs(&host, &safe_addrs) // SSRF 防护：固定连接 IP，消除 DNS Rebinding
        .build()
        .map_err(|e| SearchError::Connection(format!("reqwest 客户端创建失败: {}", e)))?;

    for (index, mapping) in [
        (indices::SALES_ORDERS, sales_orders_mapping()),
        (indices::CUSTOMERS, customers_mapping()),
        (indices::PRODUCTS, products_mapping()),
    ] {
        let url = format!("{}/{}", base_url, index);
        let resp = http
            .put(&url)
            .json(&mapping)
            .send()
            .await
            .map_err(|e| {
                SearchError::Connection(format!(
                    "ES ensure_indices 请求失败 (index={}): {}",
                    index, e
                ))
            })?;

        let status = resp.status();
        // 200 表示创建成功，400 表示索引已存在
        if !status.is_success() && status.as_u16() != 400 {
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(
                index = index,
                status = status.as_u16(),
                body = %body,
                "ES 索引创建失败（可能已存在），忽略继续"
            );
        } else {
            tracing::info!(index = index, "ES 索引确保完成（已存在或创建成功）");
        }
    }
    Ok(())
}

/// sales_orders 索引 mapping 定义
fn sales_orders_mapping() -> serde_json::Value {
    serde_json::json!({
        "mappings": {
            "properties": {
                "order_no": { "type": "keyword" },
                "customer_id": { "type": "integer" },
                "customer_name": { "type": "text", "analyzer": "standard" },
                "total_amount": { "type": "double" },
                "status": { "type": "keyword" },
                "created_at": { "type": "date" },
                "items": {
                    "type": "nested",
                    "properties": {
                        "product_id": { "type": "integer" },
                        "product_name": { "type": "text", "analyzer": "standard" },
                        "quantity": { "type": "double" },
                        "unit_price": { "type": "double" },
                        "color_no": { "type": "keyword" },
                        "pantone_code": { "type": "keyword" }
                    }
                }
            }
        }
    })
}

/// customers 索引 mapping 定义
fn customers_mapping() -> serde_json::Value {
    serde_json::json!({
        "mappings": {
            "properties": {
                "id": { "type": "integer" },
                "code": { "type": "keyword" },
                "name": { "type": "text", "analyzer": "standard" },
                "contact_person": { "type": "text", "analyzer": "standard" },
                "phone": { "type": "keyword" },
                "email": { "type": "keyword" },
                "address": { "type": "text", "analyzer": "standard" },
                "tier": { "type": "keyword" }
            }
        }
    })
}

/// products 索引 mapping 定义
fn products_mapping() -> serde_json::Value {
    serde_json::json!({
        "mappings": {
            "properties": {
                "id": { "type": "integer" },
                "code": { "type": "keyword" },
                "name": { "type": "text", "analyzer": "standard" },
                "category": { "type": "keyword" },
                "spec": { "type": "text", "analyzer": "standard" },
                "unit": { "type": "keyword" },
                "color_no": { "type": "keyword" },
                "pantone_code": { "type": "keyword" },
                "price": { "type": "double" }
            }
        }
    })
}

/// 业务同步器：将 PG 写入同步到 ES
pub struct SearchSyncer {
    /// ES 客户端，pub(crate) 供 elastic_ops 子模块访问
    pub(crate) client: Arc<dyn SearchClient>,
}

impl SearchSyncer {
    /// 创建同步器
    pub fn new(client: Arc<dyn SearchClient>) -> Self {
        Self { client }
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
        };
        syncer.sync_customer(&customer).await.unwrap();
        assert_eq!(client.doc_count(indices::CUSTOMERS).await, 1);
    }

    // ============ 批次 321 v9 复审 M-5 修复：SSRF 校验测试 ============

    /// 测试 try_real 拒绝 loopback IP（127.0.0.1）
    #[test]
    fn test_try_real_reject_loopback_ip() {
        let result = ElasticClient::try_real("http://127.0.0.1:9200".to_string());
        assert!(
            result.is_err(),
            "try_real 必须拒绝 loopback IP（127.0.0.1）"
        );
    }

    /// 测试 try_real 拒绝 localhost 主机名
    #[test]
    fn test_try_real_reject_localhost() {
        let result = ElasticClient::try_real("http://localhost:9200".to_string());
        assert!(
            result.is_err(),
            "try_real 必须拒绝 localhost 主机名"
        );
    }

    /// 测试 try_real 拒绝 RFC1918 私有网络 IP
    #[test]
    fn test_try_real_reject_rfc1918() {
        assert!(
            ElasticClient::try_real("http://10.0.0.1:9200".to_string()).is_err(),
            "try_real 必须拒绝 10.0.0.0/8"
        );
        assert!(
            ElasticClient::try_real("http://172.16.0.1:9200".to_string()).is_err(),
            "try_real 必须拒绝 172.16.0.0/12"
        );
        assert!(
            ElasticClient::try_real("http://192.168.1.1:9200".to_string()).is_err(),
            "try_real 必须拒绝 192.168.0.0/16"
        );
    }

    /// 测试 try_real 拒绝云元数据服务 IP（169.254.169.254）
    #[test]
    fn test_try_real_reject_metadata_service() {
        let result = ElasticClient::try_real("http://169.254.169.254:9200".to_string());
        assert!(
            result.is_err(),
            "try_real 必须拒绝云元数据服务 IP（169.254.169.254）"
        );
    }

    /// 测试 try_real 拒绝非 http/https 协议（file://、gopher://）
    #[test]
    fn test_try_real_reject_disallowed_scheme() {
        assert!(
            ElasticClient::try_real("file:///etc/passwd".to_string()).is_err(),
            "try_real 必须拒绝 file:// 协议"
        );
        assert!(
            ElasticClient::try_real("gopher://example.com:9200".to_string()).is_err(),
            "try_real 必须拒绝 gopher:// 协议"
        );
    }

    /// 测试 try_real 拒绝格式无效的 URL
    #[test]
    fn test_try_real_reject_invalid_url() {
        let result = ElasticClient::try_real("not-a-url".to_string());
        assert!(
            result.is_err(),
            "try_real 必须拒绝格式无效的 URL"
        );
    }

    /// 测试 try_real 拒绝 IPv6 loopback（::1）
    #[test]
    fn test_try_real_reject_ipv6_loopback() {
        let result = ElasticClient::try_real("http://[::1]:9200".to_string());
        assert!(
            result.is_err(),
            "try_real 必须拒绝 IPv6 loopback（::1）"
        );
    }

    /// 测试 try_real 拒绝 .local 后缀主机名（mDNS）
    #[test]
    fn test_try_real_reject_local_suffix() {
        let result = ElasticClient::try_real("http://es.local:9200".to_string());
        assert!(
            result.is_err(),
            "try_real 必须拒绝 .local 后缀主机名"
        );
    }

    /// 测试 ensure_indices 拒绝 loopback IP
    #[tokio::test]
    async fn test_ensure_indices_reject_loopback_ip() {
        let result = ensure_indices("http://127.0.0.1:9200").await;
        assert!(
            result.is_err(),
            "ensure_indices 必须拒绝 loopback IP（127.0.0.1）"
        );
        // 验证错误类型为 Connection（SSRF 校验失败）
        let err = result.unwrap_err();
        assert!(
            matches!(err, SearchError::Connection(_)),
            "SSRF 校验失败应返回 Connection 错误，实际: {:?}",
            err
        );
    }

    /// 测试 ensure_indices 拒绝 localhost 主机名
    #[tokio::test]
    async fn test_ensure_indices_reject_localhost() {
        let result = ensure_indices("http://localhost:9200").await;
        assert!(
            result.is_err(),
            "ensure_indices 必须拒绝 localhost 主机名"
        );
    }

    /// 测试 ensure_indices 拒绝 RFC1918 私有网络 IP
    #[tokio::test]
    async fn test_ensure_indices_reject_rfc1918() {
        assert!(
            ensure_indices("http://10.0.0.1:9200").await.is_err(),
            "ensure_indices 必须拒绝 10.0.0.0/8"
        );
        assert!(
            ensure_indices("http://192.168.1.1:9200").await.is_err(),
            "ensure_indices 必须拒绝 192.168.0.0/16"
        );
    }

    /// 测试 ensure_indices 拒绝云元数据服务 IP
    #[tokio::test]
    async fn test_ensure_indices_reject_metadata_service() {
        let result = ensure_indices("http://169.254.169.254:9200").await;
        assert!(
            result.is_err(),
            "ensure_indices 必须拒绝云元数据服务 IP（169.254.169.254）"
        );
    }

    /// 测试 ensure_indices 拒绝非 http/https 协议
    #[tokio::test]
    async fn test_ensure_indices_reject_disallowed_scheme() {
        assert!(
            ensure_indices("file:///etc/passwd").await.is_err(),
            "ensure_indices 必须拒绝 file:// 协议"
        );
    }

    /// 测试 ensure_indices 拒绝格式无效的 URL
    #[tokio::test]
    async fn test_ensure_indices_reject_invalid_url() {
        let result = ensure_indices("not-a-url").await;
        assert!(
            result.is_err(),
            "ensure_indices 必须拒绝格式无效的 URL"
        );
    }
}
