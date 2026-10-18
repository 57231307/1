//! 链来源写入口与转采来源落库行为锁（真库 PostgreSQL，CI 常规分区可跑）
//!
//! ## 本文件钉住的真实行为（三条链地基的写侧与读侧）
//! 1. 销售订单建单走**真实 service**（`SalesService::create_order`）：
//!    - 明细行提交 `fulfillment_source` = 词表值 ⇒ 列值原样落库，出参键恒存在且等值；
//!    - 明细行不提交该键 ⇒ 列保持 NULL（未定档），服务端不得代填默认值；
//!    - 明细行提交词表外取值 ⇒ 整笔拒绝（机器码 `VALIDATION_ERROR`）、文案点名允许清单、
//!      文案不含任何数字形态的记录 ID，且订单与明细**零落库**（事务回滚不留半行）。
//! 2. 门控只管词表外的非 NULL 值：三个合法值各自放行、缺省放行；大小写漂移、尾随空白、
//!    空串、前缀匹配值（`productionx`）一律拒——放行这些就等于把行为真相源写脏。
//! 3. DB 侧兜底与词表同源：绕过服务层的裸 SQL 直写词表外值必须被 CHECK
//!    `chk_sales_order_item_fulfillment_source` 拒（按 SQLSTATE 23514 + 约束名双归因，
//!    只断 is_err 会把列宽超限/类型不符也当成"拒绝生效"）；NULL 与三值必须放得过
//!    （同模板对照探针，防夹具本身写坏导致拒绝集体假绿）。
//! 4. 转采来源真正入库：`purchase_orders.source_sales_order_id` 建单后能从库读回，并经
//!    真实读取链出现在出参 DTO；NULL = 普通采购单；指向不存在的销售订单必须撞 FK
//!    （23503 + 约束名）；来源销售订单被删除时采购单不被连带删除、该列被置回 NULL
//!    （SET NULL 的实际行为，既非 RESTRICT 也非 CASCADE）。
//! 5. 接线不悬空：静态核对门控确实挂在两条写链共用的明细漏斗上、表头构造器确实把请求
//!    字段写进列（任一被摘，上面的行为断言即红；静态面同时点名位置，便于归因）。
//!
//! ## 数据库口径
//! 夹具 `setup_test_db()` 连的是已跑完迁移的 PostgreSQL（`TEST_DATABASE_URL` 缺失或指
//! sqlite 时夹具直接 panic，不静默降级、不回退），形态照
//! `tests/contract_wave4_sales_order_error_and_date_test.rs` 与
//! `tests/contract_wave11_po_so_reason_readback_test.rs`：业务表逐例 TRUNCATE，
//! `suppliers` 属封存参照表跨例累积 ⇒ 自增插入并回读真实主键，不用显式 id。
//! 集成测试彼此由 `backend/.config/nextest.toml` 的 db-integration 组串行化
//! （夹具是整库 TRUNCATE，并发必互清）。
//!
//! ## 为什么不直冲 `PurchaseOrderService::create_order`
//! 该入口在携带 `source_sales_order_id` 时必然走 SKU 对照翻译门控（无对照即拒单），并按
//! 供应商黑名单/资质分级门控阻断——这些都与本列无关，把断言绑到无关门控上，门控口径一变
//! 本锁就以无关原因红。故采购侧照既有活库范式（见 `tests/contract_wave5_trade_fields_
//! roundtrip_test.rs` 的 attachment_urls 段）：以与表头构造器同一列集写入真行，再用真实
//! 读取链 `get_order` 回读 DTO；「请求字段 → 列」的接线由第 5 条静态核对钉死。

mod test_common;

use bingxi_backend::models::customer;
use bingxi_backend::models::purchase_order;
use bingxi_backend::models::sales_order;
use bingxi_backend::models::sales_order::Entity as SalesOrderEntity;
use bingxi_backend::models::sales_order_item;
use bingxi_backend::models::status::purchase_order as po_status;
use bingxi_backend::models::status::sales::fulfillment_source;
use bingxi_backend::models::supplier;
use bingxi_backend::models::user;
use bingxi_backend::models::warehouse;
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::po::order::PurchaseOrderService;
use bingxi_backend::services::so::CreateSalesOrderRequest;
use bingxi_backend::services::so::SalesOrderItemRequest;
use bingxi_backend::services::so::order::SalesService;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, RuntimeErr, Set, Statement, Value,
};
use serde_json::{Value as Json, json};
use std::sync::Arc;
use test_common::setup_test_db;

/// 操作人主键：users 逐例被夹具清空，可显式固定
/// （sales_orders.created_by 带 FK 指向 users，建单必须落在真实操作人上）
const OPERATOR_ID: i32 = 9873;
/// 本文件专用产品主键带：一行带链来源、一行不带（products 逐例清空，显式 id 不与他例冲突）
const PRODUCT_STOCK: i32 = 987_301;
const PRODUCT_NULL_SOURCE: i32 = 987_302;
/// CHECK 与 FK 的约束名（与迁移原文逐字符一致，任一侧改名即红）
const CHECK_NAME: &str = "chk_sales_order_item_fulfillment_source";
const FK_NAME: &str = "fk_purchase_orders_source_sales_order";

fn now() -> DateTime<Utc> {
    Utc::now()
}

/// 可空文本参数的显式构造：`None` 必须是 SQL NULL，而不是空串
/// （空串是词表外的第四个取值，两者混淆会让 NULL 探针失去意义）。
fn text_param(value: Option<&str>) -> Value {
    Value::String(value.map(str::to_string))
}

/// 驱动上报的 SQLSTATE（按码归因，不做 `to_string().contains` 含混匹配）
fn sqlstate_of(err: &DbErr) -> String {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned()))
            .unwrap_or_default(),
        other => panic!("期望执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 驱动上报的约束名：把 23514/23503 归因到本列这一条约束，而不是表上任意一条
fn constraint_of(err: &DbErr) -> String {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string()))
            .unwrap_or_default(),
        other => panic!("期望执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 从 Result 取 DbErr：Ok 即判红（拒绝类断言不许只写 is_err 形态的模糊通过）
fn expect_db_err(res: Result<Json, DbErr>, probe: &str) -> DbErr {
    match res {
        Ok(_) => panic!("旁路探针 {probe} 本应被数据库拒掉，却写入成功"),
        Err(err) => err,
    }
}

async fn seed_operator(db: &sea_orm::DatabaseConnection) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("flow3_lineage_operator".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("流转链地基锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子操作人插入失败（建单 created_by 的前置）");
}

/// suppliers 属封存参照表不被清空 ⇒ 自增插入并回读真实主键（显式 id 会跨例撞主键）
async fn seed_supplier(db: &sea_orm::DatabaseConnection, tag: &str) -> i32 {
    let s = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-FLOW3-{tag}")),
        supplier_name: Set(format!("流转链地基锁供应商{tag}")),
        supplier_short_name: Set("地供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set(format!("91330000FLOW3{tag}X")),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000001".to_string()),
        contact_phone: Set("13800000001".to_string()),
        is_processor: Set(false),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子供应商插入失败");
    s.id
}

async fn seed_customer(db: &sea_orm::DatabaseConnection) -> i32 {
    let c = customer::ActiveModel {
        customer_code: Set("CUS-FLOW3-LINEAGE".to_string()),
        customer_name: Set("流转链地基锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set(bingxi_backend::constants::customer_type::RETAIL.to_string()),
        owner_id: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子客户插入失败（sales_orders.customer_id FK 前置）");
    c.id
}

async fn seed_warehouse(db: &sea_orm::DatabaseConnection) -> i32 {
    let w = warehouse::ActiveModel {
        warehouse_code: Set("WH-FLOW3-LINEAGE".to_string()),
        name: Set("流转链地基锁仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子仓库插入失败");
    w.id
}

/// 两行产品（存在性门控前置）。products 的 warehouse_id FK 可空，省略即 NULL。
async fn seed_products(db: &sea_orm::DatabaseConnection) {
    for id in [PRODUCT_STOCK, PRODUCT_NULL_SOURCE] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO products (id, code, name) VALUES ($1, $2, $3)",
            vec![
                id.into(),
                format!("P-{id}").into(),
                format!("流转链地基锁产品{id}").into(),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子产品 {id} 插入失败: {e}"));
    }
}

/// 种一张**不带明细行**的销售订单，返回真实主键（采购侧 FK 前置与 ON DELETE 探针用）。
/// 表头金额列写全：`sales_order::Model` 声明 subtotal/tax_amount/.../balance_amount 为
/// 非 Option，缺列即整行解码失败（活库夹具既有教训，照 wave4 种子列集）。
/// 不带明细是刻意的：带明细时删除该销售订单会先撞明细行 FK，归因就不再唯一指向本列 FK。
async fn seed_sales_order_without_items(db: &sea_orm::DatabaseConnection, order_no: &str) -> i32 {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"INSERT INTO sales_orders (order_no, customer_id, order_date, subtotal,
                                        tax_amount, discount_amount, shipping_cost,
                                        total_amount, paid_amount, balance_amount, created_by)
               VALUES ($1, (SELECT id FROM customers ORDER BY id LIMIT 1),
                       $2::timestamptz, 0, 0, 0, 0, 0, 0, 0, $3)
               RETURNING id"#,
            vec![
                order_no.to_string().into(),
                now().to_rfc3339().into(),
                OPERATOR_ID.into(),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子销售订单 {order_no} 插入失败: {e}"))
        .unwrap_or_else(|| panic!("种子销售订单 {order_no} 的 RETURNING id 未回一行"));
    row.try_get_by_index::<i32>(0)
        .expect("RETURNING id 应可解码为 i32")
}

fn sales_service(db: &sea_orm::DatabaseConnection) -> SalesService {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    SalesService::new(Arc::new(db.clone()), search_client)
}

/// 真实建单请求体：明细的 `fulfillment_source` 键可选，缺省即不带该键（前端契约同形）
fn create_request(customer_id: i32, items: Vec<Json>) -> CreateSalesOrderRequest {
    let payload = json!({
        "customer_id": customer_id,
        "items": items,
    });
    serde_json::from_value(payload).expect("建单请求体必须可反序列化为 CreateSalesOrderRequest")
}

fn item_payload(product_id: i32, source: Option<&str>) -> Json {
    let mut item = json!({
        "product_id": product_id,
        "quantity": "1",
        "unit_price": "10",
    });
    if let Some(source) = source {
        item["fulfillment_source"] = json!(source);
    }
    item
}

/// 按订单 + 产品取那一行明细的链来源列值（直读实体，不经出参加工）
async fn item_source_by_product(
    order_id: i32,
    product_id: i32,
    db: &sea_orm::DatabaseConnection,
) -> Option<String> {
    let item = sales_order_item::Entity::find()
        .filter(sales_order_item::Column::OrderId.eq(order_id))
        .filter(sales_order_item::Column::ProductId.eq(product_id))
        .one(db)
        .await
        .expect("按订单与产品读取明细行失败")
        .expect("明细行必须已落库");
    item.fulfillment_source
}

/// 与表头构造器同一列集写入一行采购单；`source_sales_order_id` 为 None = 普通采购单
async fn insert_purchase_order(
    db: &sea_orm::DatabaseConnection,
    supplier_id: i32,
    warehouse_id: i32,
    source_sales_order_id: Option<i32>,
    tag: &str,
) -> Result<purchase_order::Model, DbErr> {
    purchase_order::ActiveModel {
        order_no: Set(format!("POFLOW3{tag}")),
        supplier_id: Set(supplier_id),
        order_date: Set(Utc::now().date_naive()),
        warehouse_id: Set(warehouse_id),
        department_id: Set(1),
        purchaser_id: Set(OPERATOR_ID),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(Decimal::ZERO),
        total_amount_foreign: Set(Decimal::ZERO),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(po_status::DRAFT.to_string()),
        source_sales_order_id: Set(source_sales_order_id),
        created_by: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
}

/// 行为锁一：合法链来源经真实 service 落库并原样读回；未提交保持 NULL，服务端不代填。
#[tokio::test]
async fn valid_source_is_persisted_and_absent_key_stays_null() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    let customer_id = seed_customer(&db).await;
    seed_products(&db).await;

    let req = create_request(
        customer_id,
        vec![
            item_payload(PRODUCT_STOCK, Some(fulfillment_source::STOCK)),
            item_payload(PRODUCT_NULL_SOURCE, None),
        ],
    );
    let detail = sales_service(&db)
        .create_order(req, OPERATOR_ID)
        .await
        .expect("合法链来源的建单必须成功");

    let persisted_stock = item_source_by_product(detail.id, PRODUCT_STOCK, &db).await;
    assert_eq!(
        persisted_stock.as_deref(),
        Some(fulfillment_source::STOCK),
        "提交 stock 必须原样落 fulfillment_source 列（归一或丢弃都会让行级链真相源失效）"
    );
    let persisted_null = item_source_by_product(detail.id, PRODUCT_NULL_SOURCE, &db).await;
    assert_eq!(
        persisted_null, None,
        "未提交链来源必须保持 NULL（未定档）：代填任何默认值都会把「没决定」读成「已决定」"
    );

    // 出参面：键恒存在且等值（列表与详情共用同一装配函数）
    let rendered = serde_json::to_value(&detail.items).expect("明细出参必须可序列化");
    let rows = rendered.as_array().expect("明细出参必须是数组形态");
    let stock_row = rows
        .iter()
        .find(|row| row["product_id"] == json!(PRODUCT_STOCK))
        .expect("出参必须含该行明细");
    let raw_source = stock_row.get("fulfillment_source");
    assert!(
        raw_source.is_some(),
        "出参必须带 fulfillment_source 键（无 skip_serializing_if），否则前端无法区分「未定档」与「字段缺失」"
    );
    assert_eq!(
        raw_source.cloned(),
        Some(json!(fulfillment_source::STOCK)),
        "出参链来源必须等于落库值，实得: {raw_source:?}"
    );
}

/// 行为锁二：词表外取值整笔拒绝、文案点名允许清单、订单与明细零落库。
#[tokio::test]
async fn unknown_source_is_rejected_with_allowed_list_and_zero_write() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    let customer_id = seed_customer(&db).await;
    seed_products(&db).await;

    let req = create_request(customer_id, vec![item_payload(PRODUCT_STOCK, Some("自产"))]);
    let err = match sales_service(&db).create_order(req, OPERATOR_ID).await {
        Ok(_) => panic!("词表外链来源必须被拒，放行即把脏值写进行为真相源列"),
        Err(err) => err,
    };
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "词表外取值必须归参数校验族，实得: {}",
        err.error_code()
    );
    let message = err.to_response().message;
    assert!(
        message.contains(&fulfillment_source::ALL.join("/")),
        "拒绝文案必须点名允许清单（用户要看到该填什么），实得文案: {message}"
    );
    assert!(
        message.contains("自产"),
        "拒绝文案必须点出被拒的那个提交值，实得文案: {message}"
    );
    assert!(
        !message.chars().any(|c| c.is_ascii_digit()),
        "拒绝文案不得外显任何数字形态的记录 ID/内部标识，实得文案: {message}"
    );
    assert_eq!(
        sales_order::Entity::find()
            .count(&db)
            .await
            .expect("统计订单行数失败"),
        0,
        "被拒的建单必须零落库（事务回滚，不留半张单）"
    );
    assert_eq!(
        sales_order_item::Entity::find()
            .count(&db)
            .await
            .expect("统计明细行数失败"),
        0,
        "被拒的建单必须零明细落库"
    );
}

/// 行为锁三：门控只认词表内的非 NULL 值。合法三值与缺省放行；漂移形态一律拒。
/// 这一条不连库（门控是写入前的纯判定），保证「拒绝面」在没有活库时也被钉住。
#[test]
fn gate_admits_vocab_and_null_and_refuses_drift_forms() {
    let base = json!({
        "product_id": 1,
        "quantity": "1",
        "unit_price": "10",
    });

    for token in fulfillment_source::ALL {
        let mut value = base.clone();
        value["fulfillment_source"] = json!(token);
        let item: SalesOrderItemRequest =
            serde_json::from_value(value).expect("明细请求体必须可反序列化");
        if let Err(err) = item.validate_fulfillment_source() {
            panic!(
                "合法取值 {token} 必须放行，实得: {}",
                err.to_response().message
            );
        }
    }

    let absent: SalesOrderItemRequest =
        serde_json::from_value(base.clone()).expect("缺省形态必须可反序列化");
    assert_eq!(absent.fulfillment_source, None, "缺省即未定档");
    absent
        .validate_fulfillment_source()
        .expect("NULL（未提交）必须原样放行，不得被当成非法值");

    // 漂移形态：大写、尾随空白、空串、前缀匹配、词表外自造值
    for bad in ["PRODUCTION", "production ", "", "productionx", "made_up"] {
        let mut value = base.clone();
        value["fulfillment_source"] = json!(bad);
        let item: SalesOrderItemRequest =
            serde_json::from_value(value).expect("明细请求体必须可反序列化");
        let err = match item.validate_fulfillment_source() {
            Ok(_) => panic!("形态 {bad:?} 不得被放行（放行即 CHECK 与入参校验两套判定分裂）"),
            Err(err) => err,
        };
        assert_eq!(err.error_code(), "VALIDATION_ERROR");
        assert!(
            err.to_response()
                .message
                .contains(&fulfillment_source::ALL.join("/")),
            "每个被拒形态都要点名允许清单，实得: {}",
            err.to_response().message
        );
    }
}

/// 行为锁四：DB CHECK 真实拦住旁路直写，同时放过 NULL 与三个合法值（同模板对照）。
#[tokio::test]
async fn db_check_refuses_bypass_write_but_admits_null_and_vocab_tokens() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    seed_customer(&db).await;
    seed_products(&db).await;
    let order_id = seed_sales_order_without_items(&db, "SO-CHECK-PROBE").await;

    let mut admitted: Vec<Option<&str>> =
        fulfillment_source::ALL.iter().copied().map(Some).collect();
    admitted.push(None);
    for probe in admitted {
        let sql = r#"INSERT INTO sales_order_items
                 (order_id, product_id, quantity, unit_price, subtotal, fulfillment_source)
               VALUES ($1, $2, 1, 10, 10, $3)"#;
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            vec![order_id.into(), PRODUCT_STOCK.into(), text_param(probe)],
        ))
        .await
        .unwrap_or_else(|e| panic!("合法形态 {probe:?} 必须能通过 CHECK，实得: {e}"));
    }

    let reject_sql = r#"INSERT INTO sales_order_items
                 (order_id, product_id, quantity, unit_price, subtotal, fulfillment_source)
               VALUES ($1, $2, 1, 10, 10, $3)"#;
    for bad in ["made_up", "PRODUCTION", ""] {
        let res = db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                reject_sql,
                vec![
                    order_id.into(),
                    PRODUCT_NULL_SOURCE.into(),
                    text_param(Some(bad)),
                ],
            ))
            .await
            .map(|_| Json::Null);
        let err = expect_db_err(res, bad);
        assert_eq!(
            sqlstate_of(&err),
            "23514",
            "必须按 check_violation 归因（列宽超限 22001/类型不符 42804 也长得像被拒），实得: {}",
            sqlstate_of(&err)
        );
        assert_eq!(
            constraint_of(&err),
            CHECK_NAME,
            "23514 必须归因到本列约束 {CHECK_NAME}，实得约束名: {}",
            constraint_of(&err)
        );
    }
}

/// 行为锁五：转采来源销售订单 ID 真实入库，并经真实读取链回显；NULL = 普通采购单。
#[tokio::test]
async fn po_source_sales_order_id_is_persisted_and_read_back() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    let supplier_id = seed_supplier(&db, "LINK").await;
    let warehouse_id = seed_warehouse(&db).await;
    let source_id = seed_sales_order_without_items(&db, "SO-PO-ORIGIN").await;

    let linked = match insert_purchase_order(
        &db,
        supplier_id,
        warehouse_id,
        Some(source_id),
        "LINK",
    )
    .await
    {
        Ok(model) => model,
        Err(err) => panic!("带转采来源的采购单建单必须成功: {err}"),
    };
    assert_eq!(
        linked.source_sales_order_id,
        Some(source_id),
        "写入的来源销售订单 ID 必须回读为同一值"
    );

    // 经真实读取链（全列 SELECT + into_model::<Dto>()）回显
    let service = PurchaseOrderService::new(Arc::new(db.clone()));
    let dto = match service.get_order(linked.id, None).await {
        Ok(dto) => dto,
        Err(err) => panic!("采购单详情读取必须成功: {}", err.error_code()),
    };
    assert_eq!(
        dto.source_sales_order_id,
        Some(source_id),
        "出参必须把转采来源销售订单 ID 读回，否则血缘仍是「写了但查不到」"
    );
    let rendered = serde_json::to_value(&dto).expect("采购单出参必须可序列化");
    assert_eq!(
        rendered.get("source_sales_order_id"),
        Some(&json!(source_id)),
        "出参键恒存在且等于落库值（无 skip_serializing_if）"
    );

    let plain = match insert_purchase_order(&db, supplier_id, warehouse_id, None, "PLAIN").await {
        Ok(model) => model,
        Err(err) => panic!("普通采购单建单必须成功: {err}"),
    };
    let plain_dto = service
        .get_order(plain.id, None)
        .await
        .expect("普通采购单详情读取必须成功");
    assert_eq!(
        plain_dto.source_sales_order_id, None,
        "普通采购单的来源列必须是 NULL（服务端不得替它编一个来源）"
    );
    let plain_rendered = serde_json::to_value(&plain_dto).expect("普通采购单出参必须可序列化");
    assert_eq!(
        plain_rendered.get("source_sales_order_id"),
        Some(&Json::Null),
        "无来源时出参键必须存在且为 null，缺键会让前端把「字段缺失」误读成「有来源但为空」"
    );
}

/// 行为锁六：本列 FK 真实生效——孤儿引用被拒；来源被删时采购单存活且该列置回 NULL。
#[tokio::test]
async fn po_source_fk_refuses_orphan_and_survives_source_deletion_as_null() {
    let db = setup_test_db().await;
    seed_operator(&db).await;
    let supplier_id = seed_supplier(&db, "FK").await;
    let warehouse_id = seed_warehouse(&db).await;

    let orphan = insert_purchase_order(&db, supplier_id, warehouse_id, Some(9_873_999), "ORPHAN")
        .await
        .map(|model| Json::from(model.id));
    let err = expect_db_err(orphan, "孤儿来源销售订单");
    assert_eq!(
        sqlstate_of(&err),
        "23503",
        "必须按 foreign_key_violation 归因，实得: {}",
        sqlstate_of(&err)
    );
    assert_eq!(
        constraint_of(&err),
        FK_NAME,
        "23503 必须归因到本列外键 {FK_NAME}，实得约束名: {}",
        constraint_of(&err)
    );

    let source_id = seed_sales_order_without_items(&db, "SO-ONDELETE-PROBE").await;
    let po =
        match insert_purchase_order(&db, supplier_id, warehouse_id, Some(source_id), "DEL").await {
            Ok(model) => model,
            Err(err) => panic!("带真实来源的采购单建单必须成功: {err}"),
        };
    let deleted = SalesOrderEntity::delete_many()
        .filter(sales_order::Column::Id.eq(source_id))
        .exec(&db)
        .await;
    if let Err(err) = deleted {
        panic!("删除来源销售订单必须成功（不得因本列 FK 被 RESTRICT 阻断）: {err}");
    }
    let after = purchase_order::Entity::find_by_id(po.id)
        .one(&db)
        .await
        .expect("读取采购单失败")
        .expect("采购单必须仍然存在（FK 不得 CASCADE 删掉已开出的采购单）");
    assert_eq!(
        after.source_sales_order_id, None,
        "来源销售订单被删后本列必须置回 NULL（退化为普通采购单），实得: {:?}",
        after.source_sales_order_id
    );
}

/// 接线锁：门控挂在两条写链共用的明细漏斗上、明细构造器把请求值原样写进列、
/// 采购表头构造器把请求字段写进列。三处任一被摘，上面的行为断言必红，本锁再点名位置。
#[test]
fn write_funnel_is_wired_to_gate_and_columns() {
    let funnel = code_only(&read_src("src/services/so/order_crud.rs"));
    assert!(
        funnel.contains("item_req.validate_fulfillment_source()?;"),
        "明细漏斗必须逐行调用链来源门控（建单与改单共用该漏斗，缺调用即绕过校验）"
    );
    assert!(
        funnel.contains(
            "fulfillment_source: sea_orm::ActiveValue::Set(item_req.fulfillment_source.clone())"
        ),
        "明细构造器必须把请求的链来源原样写进列（写死 None 或做归一即吞值）"
    );
    let po_header = code_only(&read_src("src/services/po/order_ops/crud.rs"));
    assert!(
        po_header.contains("source_sales_order_id: Set(req.source_sales_order_id)"),
        "采购表头构造器必须把请求的来源销售订单 ID 写进列（只用于翻译门控而不落库即血缘断裂）"
    );
}

fn read_src(rel: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {rel} 失败: {e}"))
}

/// 去掉以 `//` 起始的注释行：接线锁只认真实代码，注释里出现同样文字不算接线。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}
