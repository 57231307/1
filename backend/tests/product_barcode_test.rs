//! 产品条码（products.barcode）链路测试
//!
//! 条码是扫码取数的入口，本文件按层锁住它，缺任何一层即红：
//! 1. 写入：`CreateProductRequest` / `UpdateProductRequest` 反序列化拿到 barcode，
//!    长度校验与 `products.barcode VARCHAR(100)` 对齐（超长按校验拒绝，不落库报 500）；
//!    未提交 barcode 时字段为 None，更新不得把已有值抹掉。
//! 2. 读取：`product::Model` 在 JSON 上以 `barcode` 键出参（前端列表列与表单回源取该键）。
//! 3. 检索：关键词条件为「名称 OR 编码 OR 条码」三列 LIKE，列清单见
//!    `ProductService::PRODUCT_KEYWORD_COLUMNS`。
//!
//! 建产品带条码 → 详情返回该值 → 用条码作 keyword 搜到 → 改条码 的完整链路
//! 需要真实 PostgreSQL schema，见文件末 `#[ignore]` 用例。

use bingxi_backend::handlers::product_handler::{CreateProductRequest, UpdateProductRequest};
use bingxi_backend::models::product;
use bingxi_backend::models::status::master_data;
use bingxi_backend::services::product_service::ProductService;
use validator::Validate;

/// 13 位 EAN-13 样例条码
const SAMPLE_BARCODE: &str = "6901234567892";

/// 把检索条件渲染为 SQL 串（借 SELECT 取 WHERE 之后的谓词文本，校验谓词形态）
fn condition_sql(condition: &sea_orm::sea_query::Condition) -> String {
    use sea_orm::sea_query::{Cond, PostgresQueryBuilder, Query};
    let sql = Query::select()
        .expr(1i32)
        .cond_where(Cond::all().add(condition.clone()))
        .to_string(PostgresQueryBuilder);
    sql.split_once(" WHERE ")
        .map(|(_, after)| after.to_string())
        .unwrap_or_default()
}

/// 创建请求拿到条码字段（此前无该字段，serde 静默丢弃，用户填的条码落不了库）
#[test]
fn test_create_product_request_accepts_barcode() {
    let req: CreateProductRequest = serde_json::from_str(&format!(
        r#"{{"name":"纯棉坯布","code":"FAB-0001","unit":"米","barcode":"{SAMPLE_BARCODE}"}}"#
    ))
    .expect("创建产品请求应能反序列化 barcode");
    assert_eq!(req.barcode.as_deref(), Some(SAMPLE_BARCODE));
    req.validate().expect("13 位条码应通过长度校验");
}

/// 更新请求拿到条码；未提交条码时为 None，由更新逻辑跳过该列而不是抹掉已有值
#[test]
fn test_update_product_request_barcode_semantics() {
    let with_barcode: UpdateProductRequest =
        serde_json::from_str(&format!(r#"{{"barcode":"{SAMPLE_BARCODE}"}}"#))
            .expect("更新产品请求应能反序列化 barcode");
    assert_eq!(with_barcode.barcode.as_deref(), Some(SAMPLE_BARCODE));

    let without_barcode: UpdateProductRequest =
        serde_json::from_str(r#"{"name":"纯棉坯布"}"#).expect("更新产品请求反序列化失败");
    assert!(
        without_barcode.barcode.is_none(),
        "请求未携带 barcode 时应为 None"
    );
}

/// 条码长度上限与列定义 VARCHAR(100) 一致：101 字符按校验失败拒绝
#[test]
fn test_barcode_length_limited_to_column_width() {
    let too_long = "9".repeat(101);
    let req: CreateProductRequest = serde_json::from_str(&format!(
        r#"{{"name":"纯棉坯布","code":"FAB-0002","barcode":"{too_long}"}}"#
    ))
    .expect("创建产品请求反序列化失败");
    let err = req.validate().expect_err("101 字符条码应被长度校验拒绝");
    assert!(
        err.field_errors().contains_key("barcode"),
        "校验失败应定位到 barcode 字段，实际: {err:?}"
    );
}

/// 出参在 JSON 上带 barcode 键（产品列表「条码」列与编辑回源都取这个键）
#[test]
fn test_product_model_returns_barcode() {
    let model: product::Model = serde_json::from_value(serde_json::json!({
        "id": 1,
        "name": "纯棉坯布",
        "code": "FAB-0003",
        "barcode": SAMPLE_BARCODE,
        "unit": "米",
        "status": master_data::ACTIVE,
        "is_deleted": false,
        "product_type": "坯布",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
    }))
    .expect("产品实体应含 barcode 字段");
    assert_eq!(model.barcode.as_deref(), Some(SAMPLE_BARCODE));

    let json = serde_json::to_value(&model).expect("产品实体序列化失败");
    assert_eq!(json["barcode"].as_str(), Some(SAMPLE_BARCODE));
}

/// 关键词检索覆盖名称/编码/条码三列，且三列之间是 OR
#[test]
fn test_keyword_search_covers_barcode() {
    let sql = condition_sql(&ProductService::build_product_keyword_condition(&format!(
        "%{SAMPLE_BARCODE}%"
    )));
    for fragment in ["\"name\" LIKE", "\"code\" LIKE", "\"barcode\" LIKE"] {
        assert!(
            sql.contains(fragment),
            "检索条件应含 {fragment}（列清单见 PRODUCT_KEYWORD_COLUMNS），实际: {sql}"
        );
    }
    assert_eq!(
        sql.matches(" LIKE ").count(),
        ProductService::PRODUCT_KEYWORD_COLUMNS.len(),
        "每列各一次 LIKE，实际: {sql}"
    );
    assert!(sql.contains(" OR "), "三列之间应为 OR，实际: {sql}");
}

// ===== 真实 PostgreSQL 链路（建 → 详情 → 按条码搜 → 改条码）=====

/// 条码全链路：建产品带条码 → 详情返回该值 → 用条码作 keyword 命中 → 改条码后按新值命中
///
/// 需要 PostgreSQL 测试库（products 表含 barcode 列）：
/// `TEST_DATABASE_URL=postgres://... cargo test --test product_barcode_test -- --ignored`
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试库与 products schema（CI 未设 TEST_DATABASE_URL）"]
async fn test_barcode_create_read_search_chain() {
    use bingxi_backend::search::{ElasticClient, SearchClient};
    use bingxi_backend::services::product_service::{CreateProductArgs, UpdateProductArgs};
    use std::sync::Arc;

    let db_url = std::env::var("TEST_DATABASE_URL").expect("需设置 TEST_DATABASE_URL 环境变量");
    let db = sea_orm::Database::connect(&db_url)
        .await
        .expect("测试库连接失败");
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = ProductService::new(Arc::new(db), search_client);

    // 编码与条码按纳秒时间戳生成，避免与并发分片上的既有数据撞唯一键
    let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let barcode = format!("69{stamp}");
    let new_barcode = format!("69{stamp}1");

    let created = service
        .create_product(CreateProductArgs {
            name: "条码链路测试坯布".to_string(),
            code: format!("FAB-BC{stamp}"),
            barcode: Some(barcode.clone()),
            category_id: None,
            specification: None,
            unit: "米".to_string(),
            standard_price: None,
            cost_price: None,
            description: None,
            status: master_data::ACTIVE.to_string(),
            product_type: "坯布".to_string(),
            fabric_composition: None,
            yarn_count: None,
            density: None,
            width: None,
            gram_weight: None,
            structure: None,
            finish: None,
            min_order_quantity: None,
            lead_time: None,
            execution_standard: None,
            factory_name: None,
            factory_address: None,
            product_grade: None,
            meters_per_piece: None,
            meters_per_roll: None,
        })
        .await
        .expect("建产品（带条码）应成功");
    assert_eq!(created.barcode.as_deref(), Some(barcode.as_str()));

    let detail = service
        .get_product(created.id)
        .await
        .expect("产品详情查询应成功");
    assert_eq!(detail.barcode.as_deref(), Some(barcode.as_str()));

    let (hits, total) = service
        .list_products(1, 10, None, None, Some(barcode.clone()))
        .await
        .expect("按条码做关键词检索应成功");
    assert!(
        total >= 1 && hits.iter().any(|p| p.id == created.id),
        "按条码检索应命中新建产品 id={}（total={total}）",
        created.id
    );

    service
        .update_product(UpdateProductArgs {
            id: created.id,
            name: None,
            barcode: Some(new_barcode.clone()),
            specification: None,
            unit: None,
            standard_price: None,
            cost_price: None,
            description: None,
            status: None,
            product_type: None,
            fabric_composition: None,
            yarn_count: None,
            density: None,
            width: None,
            gram_weight: None,
            structure: None,
            finish: None,
            min_order_quantity: None,
            lead_time: None,
            execution_standard: None,
            factory_name: None,
            factory_address: None,
            product_grade: None,
            meters_per_piece: None,
            meters_per_roll: None,
            user_id: 1,
        })
        .await
        .expect("改条码应成功");

    let detail = service
        .get_product(created.id)
        .await
        .expect("改后详情查询应成功");
    assert_eq!(detail.barcode.as_deref(), Some(new_barcode.as_str()));

    let (hits, _) = service
        .list_products(1, 10, None, None, Some(new_barcode.clone()))
        .await
        .expect("按新条码检索应成功");
    assert!(
        hits.iter().any(|p| p.id == created.id),
        "按新条码检索应命中该产品"
    );

    service
        .delete_product(created.id, 1)
        .await
        .expect("清理测试产品应成功");
}
