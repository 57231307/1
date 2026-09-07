//! ES 集成 facade：类型定义 + 构造函数 + 纯函数，业务方法迁移至 elastic_ops 子模块
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// SSRF 防护守卫：对 ES base_url 做协议白名单 + 主机名黑名单 + IP 黑名单 + DNS 解析校验
use crate::utils::ssrf_guard;

/// 20 个业务索引（缺陷 5 修复：3 → 20，覆盖全业务域主数据/单据）
pub mod indices {
    /// 销售订单索引
    pub const SALES_ORDERS: &str = "sales_orders";
    /// 客户索引
    pub const CUSTOMERS: &str = "customers";
    /// 产品索引
    pub const PRODUCTS: &str = "products";
    /// 供应商索引
    pub const SUPPLIERS: &str = "suppliers";
    /// 仓库索引
    pub const WAREHOUSES: &str = "warehouses";
    /// 采购订单索引
    pub const PURCHASE_ORDERS: &str = "purchase_orders";
    /// 库存批次索引
    pub const INVENTORY_BATCHES: &str = "inventory_batches";
    /// 生产工单索引
    pub const PRODUCTION_ORDERS: &str = "production_orders";
    /// 染色批次索引
    pub const DYE_BATCHES: &str = "dye_batches";
    /// 染色配方索引
    pub const DYE_RECIPES: &str = "dye_recipes";
    /// 色卡索引
    pub const COLOR_CARDS: &str = "color_cards";
    /// 色号价格索引
    pub const COLOR_PRICES: &str = "color_prices";
    /// BOM 索引
    pub const BOMS: &str = "boms";
    /// 定制订单索引
    pub const CUSTOM_ORDERS: &str = "custom_orders";
    /// 财务凭证索引
    pub const VOUCHERS: &str = "vouchers";
    /// 应收发票索引
    pub const AR_INVOICES: &str = "ar_invoices";
    /// 应付发票索引
    pub const AP_INVOICES: &str = "ap_invoices";
    /// 销售合同索引
    pub const SALES_CONTRACTS: &str = "sales_contracts";
    /// 采购合同索引
    pub const PURCHASE_CONTRACTS: &str = "purchase_contracts";
    /// 会计期间索引
    pub const ACCOUNTING_PERIODS: &str = "accounting_periods";

    /// 全部索引列表（ensure_indices 幂等创建用）
    pub const ALL: &[&str] = &[
        SALES_ORDERS,
        CUSTOMERS,
        PRODUCTS,
        SUPPLIERS,
        WAREHOUSES,
        PURCHASE_ORDERS,
        INVENTORY_BATCHES,
        PRODUCTION_ORDERS,
        DYE_BATCHES,
        DYE_RECIPES,
        COLOR_CARDS,
        COLOR_PRICES,
        BOMS,
        CUSTOM_ORDERS,
        VOUCHERS,
        AR_INVOICES,
        AP_INVOICES,
        SALES_CONTRACTS,
        PURCHASE_CONTRACTS,
        ACCOUNTING_PERIODS,
    ];
}

/// 文档类型（通过 /search/doc-types 端点暴露公共 API）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocType {
    SalesOrder,
    Customer,
    Product,
    Supplier,
    Warehouse,
    PurchaseOrder,
    InventoryBatch,
    ProductionOrder,
    DyeBatch,
    DyeRecipe,
    ColorCard,
    ColorPrice,
    Bom,
    CustomOrder,
    Voucher,
    ArInvoice,
    ApInvoice,
    SalesContract,
    PurchaseContract,
    AccountingPeriod,
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
            eprintln!("Elasticsearch URL SSRF 校验失败: {}，服务无法启动", e);
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
    let (host, safe_addrs) = ssrf_guard::validate_url_and_resolve(base_url)
        .map_err(|e| SearchError::Connection(format!("ES base_url SSRF 校验失败: {}", e)))?;

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none()) // SSRF 防护：禁止跟随重定向
        .resolve_to_addrs(&host, &safe_addrs) // SSRF 防护：固定连接 IP，消除 DNS Rebinding
        .build()
        .map_err(|e| SearchError::Connection(format!("reqwest 客户端创建失败: {}", e)))?;

    for (index, mapping) in indices::ALL.iter().map(|i| match *i {
        indices::SALES_ORDERS => (*i, sales_orders_mapping()),
        indices::CUSTOMERS => (*i, customers_mapping()),
        indices::PRODUCTS => (*i, products_mapping()),
        _ => (*i, generic_mapping()),
    }) {
        let url = format!("{}/{}", base_url, index);
        let resp = http.put(&url).json(&mapping).send().await.map_err(|e| {
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

/// 通用索引 mapping（缺陷 5：17 个新增业务索引共享，覆盖常见业务字段）
fn generic_mapping() -> serde_json::Value {
    serde_json::json!({
        "mappings": {
            "properties": {
                "id": { "type": "keyword" },
                "code": { "type": "keyword" },
                "no": { "type": "keyword" },
                "order_no": { "type": "keyword" },
                "batch_no": { "type": "keyword" },
                "color_no": { "type": "keyword" },
                "name": { "type": "text", "analyzer": "standard" },
                "title": { "type": "text", "analyzer": "standard" },
                "status": { "type": "keyword" },
                "grade": { "type": "keyword" },
                "customer_id": { "type": "integer" },
                "product_id": { "type": "integer" },
                "warehouse_id": { "type": "integer" },
                "amount": { "type": "double" },
                "total_amount": { "type": "double" },
                "quantity": { "type": "double" },
                "created_at": { "type": "date" },
                "updated_at": { "type": "date" }
            }
        }
    })
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
