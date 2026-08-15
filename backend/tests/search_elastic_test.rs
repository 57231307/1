use bingxi_backend::decs;
use bingxi_backend::search::elastic::*;
use bingxi_backend::search::elastic_ops::client_ops::*;
use bingxi_backend::search::elastic_ops::syncer_ops::*;
use bingxi_backend::search::elastic_ops::types_ops::*;
use bingxi_backend::services::five_dimension_service::*;
use bingxi_backend::ymd;
use serde_json::Value;
use std::sync::Arc;

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
        created_at: bingxi_backend::ymd!(2026, 6, 17)
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc(),
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
        price: 0.0,
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
        created_at: bingxi_backend::ymd!(2026, 6, 17)
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc(),
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
            created_at: bingxi_backend::ymd!(2026, 6, 17)
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            items: vec![],
        };
        let value = serde_json::to_value(&doc).unwrap();
        client
            .index_doc(indices::SALES_ORDERS, &format!("SO-{:03}", i), &value)
            .await
            .unwrap();
    }
    let query = SearchQuery::new().with_keyword("客户");
    let result: SearchResult<serde_json::Value> =
        client.search(indices::SALES_ORDERS, &query).await.unwrap();
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
    client
        .index_doc(indices::CUSTOMERS, "1", &value)
        .await
        .unwrap();
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
                price: 0.0,
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
        created_at: bingxi_backend::ymd!(2026, 6, 17)
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc(),
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
    assert!(result.is_err(), "try_real 必须拒绝 localhost 主机名");
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
    assert!(result.is_err(), "try_real 必须拒绝格式无效的 URL");
}

/// 测试 try_real 拒绝 IPv6 loopback（::1）
#[test]
fn test_try_real_reject_ipv6_loopback() {
    let result = ElasticClient::try_real("http://[::1]:9200".to_string());
    assert!(result.is_err(), "try_real 必须拒绝 IPv6 loopback（::1）");
}

/// 测试 try_real 拒绝 .local 后缀主机名（mDNS）
#[test]
fn test_try_real_reject_local_suffix() {
    let result = ElasticClient::try_real("http://es.local:9200".to_string());
    assert!(result.is_err(), "try_real 必须拒绝 .local 后缀主机名");
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
    assert!(result.is_err(), "ensure_indices 必须拒绝 localhost 主机名");
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
    assert!(result.is_err(), "ensure_indices 必须拒绝格式无效的 URL");
}
