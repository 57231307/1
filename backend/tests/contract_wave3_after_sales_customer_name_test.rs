//! 售后工单读侧客户名富化契约锁（customer_name 只允许来自 LEFT JOIN 真值）
//!
//! 锁定契约（修复后形态）：
//! - `backend/src/models/custom_order_response_dto.rs::AfterSalesInfo`
//!   （`customer_name: Option<String>`，同时 derive `FromQueryResult` 作为读侧
//!   into_model 视图对象；`customer_id` 仍为 NOT NULL i32 如实透传）
//! - `backend/src/services/custom_order_aftersales_service.rs::list_by_order` /
//!   `find_dto_by_id`（唯一正解范式：`column_as(customer::Column::CustomerName,
//!   "customer_name")` + `JoinType::LeftJoin` + `into_model::<AfterSalesInfo>`，
//!   范式来源 `services/po/order_ops/crud.rs::list_orders`）
//! - `backend/src/handlers/custom_order_handler.rs`（4 处 `AfterSalesInfo {` 字面量
//!   构造点全部消失：列表/详情由富化查询直接产出；create/update 写后回读同链路）
//! - 前端 `frontend/src/api/custom-order.ts::AfterSales` 声明 `customer_name: string | null`，
//!   `AfterSalesPanel.vue` 客户列显示真实客户名、null 显 '-'，禁止用 id 冒充名称
//!
//! Option 语义说明：`after_sales.customer_id` 实体为 NOT NULL i32 外键，但
//! LEFT JOIN 在客户行缺失时产生 NULL，故 `customer_name` 必须为 Option——
//! "客户行缺失" 是本测试用 sqlite 自建表实证的第二态。
//!
//! 覆盖策略（全部真实行为，无 mock）：
//! - sqlite::memory: 自建 after_sales + customers（真实表名/列名：
//!   `models/customer.rs:9` table_name="customers"、`:19` pub customer_name）
//! - "客户存在"态：创建响应 / 列表 / 更新响应三端 customer_name 忠实回显真实名
//! - "客户行缺失"态（LEFT JOIN 出 NULL）：customer_name 键必存在且为 null，
//!   不得整键缺失、不得回退成 id 或拼装名
//! - 源码扫描防回潮锁（extract_block 形态先例：contract_wave2_after_sales_create_test.rs）

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::put,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::custom_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use tower::ServiceExt;

const REAL_CUSTOMER_ID: i64 = 7;
const REAL_CUSTOMER_NAME: &str = "滨海针织有限公司";
const MISSING_CUSTOMER_ID: i64 = 999;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("self".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

/// 与 `models/after_sales.rs::Model` 逐列对应的 sqlite 最小 DDL（先例：wave2）
async fn create_after_sales_table(db: &sea_orm::DatabaseConnection) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"CREATE TABLE after_sales (
            id INTEGER PRIMARY KEY,
            custom_order_id INTEGER, issue_type TEXT, customer_id INTEGER,
            description TEXT, status TEXT,
            opened_at TEXT, closed_at TEXT, resolution TEXT, refund_amount TEXT,
            quality_issue_id INTEGER, accepted_at TEXT,
            evaluation_score INTEGER, evaluation_comment TEXT, evaluated_at TEXT,
            reason_category TEXT, reason_detail TEXT,
            created_at TEXT, updated_at TEXT
        )"#,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("after_sales DDL 执行失败: {e}"));
}

/// 真实表名 customers、真实名称列 customer_name（`models/customer.rs:9,19`）；
/// 本测试只需 JOIN 富化涉及的列
async fn create_customers_table(db: &sea_orm::DatabaseConnection) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"CREATE TABLE customers (
            id INTEGER PRIMARY KEY,
            customer_name TEXT
        )"#,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("customers DDL 执行失败: {e}"));
}

async fn seed_customer(db: &sea_orm::DatabaseConnection, id: i64, name: &str) {
    // 预构造 Statement 执行走 execute_raw（sea-orm 2.0.2 ConnectionTrait，同仓内
    // 已编译写法 src/utils/number_generator.rs:263）；execute(&S: StatementBuilder) 不适用
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO customers (id, customer_name) VALUES (?, ?)",
        vec![id.into(), name.to_string().into()],
    ))
    .await
    .unwrap_or_else(|e| panic!("seed 客户失败: {e}"));
}

async fn seeded_app() -> (Router, sea_orm::DatabaseConnection) {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    create_after_sales_table(&db).await;
    create_customers_table(&db).await;
    let mut state = AppState::default();
    state.db = std::sync::Arc::new(db.clone());
    let app = Router::new()
        .route(
            "/custom-orders/{id}/after-sales",
            axum::routing::post(custom_order_handler::create_after_sales)
                .get(custom_order_handler::list_after_sales),
        )
        .route(
            "/custom-orders/after-sales/{id}",
            put(custom_order_handler::update_after_sales),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth));
    (app, db)
}

fn create_payload(customer_id: i64) -> Value {
    json!({
        "customer_id": customer_id,
        "issue_type": "complaint",
        "description": "面料色差超出允许范围"
    })
}

async fn post_create(app: &Router, path_id: i64, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/custom-orders/{path_id}/after-sales"))
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn get_list(app: &Router, path_id: i64) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/custom-orders/{path_id}/after-sales?page=1&page_size=50"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn put_update(app: &Router, after_sales_id: i64, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/custom-orders/after-sales/{after_sales_id}"))
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 在列表 items 中按 id 定位出参对象
fn find_item(items: &[Value], id: i64) -> Value {
    items
        .iter()
        .find(|it| it["id"].as_i64() == Some(id))
        .unwrap_or_else(|| panic!("列表应含 id={id} 的工单，实际: {items:?}"))
        .clone()
}

/// customer_name 键存在性 + 真值断言的公共口径
fn assert_customer_name(obj: &Value, expected: Option<&str>) {
    assert!(
        obj.get("customer_name").is_some(),
        "后端出参必须恒含 customer_name 键（null 也必须是显式键），实际: {obj}"
    );
    match expected {
        Some(name) => assert_eq!(
            obj["customer_name"],
            json!(name),
            "customer_name 必须忠实回显 JOIN 到的真实客户名，实际: {obj}"
        ),
        None => assert!(
            obj["customer_name"].is_null(),
            "客户行缺失（LEFT JOIN 出 NULL）时 customer_name 必须为 null，禁止 id/拼装名回退，实际: {obj}"
        ),
    }
}

// =========================================================
// 1) 真实 handler 端到端：客户存在 / 客户行缺失两态
// =========================================================

/// 创建端点出参锁：种子真实客户后 POST → 响应 customer_name == 真实客户名
/// （写后回读带 JOIN 链路的直接验证，修复前该键根本不存在）
#[tokio::test]
async fn create_response_carries_real_customer_name_from_join() {
    let (app, db) = seeded_app().await;
    seed_customer(&db, REAL_CUSTOMER_ID, REAL_CUSTOMER_NAME).await;
    let (status, v) = post_create(&app, 42, create_payload(REAL_CUSTOMER_ID)).await;
    assert_eq!(status, StatusCode::OK, "实际体: {v}");
    assert_eq!(v["code"], 200);
    assert_customer_name(&v["data"], Some(REAL_CUSTOMER_NAME));
    // customer_id 原键如实透传不受影响（wave2 契约不回退）
    assert_eq!(v["data"]["customer_id"], json!(REAL_CUSTOMER_ID));
}

/// 列表端点两态锁：同一订单下"客户存在"工单回显真实名；
/// "客户行缺失"工单 customer_name 为显式 null 键（LEFT JOIN 语义忠实透传）
#[tokio::test]
async fn list_endpoint_echoes_real_name_and_null_for_missing_customer() {
    let (app, db) = seeded_app().await;
    seed_customer(&db, REAL_CUSTOMER_ID, REAL_CUSTOMER_NAME).await;
    let (s1, c1) = post_create(&app, 42, create_payload(REAL_CUSTOMER_ID)).await;
    assert_eq!(s1, StatusCode::OK, "实际体: {c1}");
    let (s2, c2) = post_create(&app, 42, create_payload(MISSING_CUSTOMER_ID)).await;
    assert_eq!(s2, StatusCode::OK, "客户行缺失不得阻断建单，实际体: {c2}");
    assert_customer_name(&c2["data"], None);

    let (status, lv) = get_list(&app, 42).await;
    assert_eq!(status, StatusCode::OK, "列表应 200，实际体: {lv}");
    let items = lv["data"]["items"]
        .as_array()
        .expect("列表应为 PagedResponse{items,...}");
    assert_eq!(items.len(), 2, "应恰有 2 条工单，实际: {lv}");

    let named = find_item(items, c1["data"]["id"].as_i64().unwrap());
    assert_customer_name(&named, Some(REAL_CUSTOMER_NAME));
    assert_eq!(named["customer_id"], json!(REAL_CUSTOMER_ID));

    let orphan = find_item(items, c2["data"]["id"].as_i64().unwrap());
    assert_customer_name(&orphan, None);
    assert_eq!(
        orphan["customer_id"],
        json!(MISSING_CUSTOMER_ID),
        "外键值本身仍须如实透传，仅名称为 null"
    );
}

/// 更新端点出参锁：PUT 状态流转后响应仍带真实客户名（同一条回读链路）
#[tokio::test]
async fn update_response_carries_real_customer_name() {
    let (app, db) = seeded_app().await;
    seed_customer(&db, REAL_CUSTOMER_ID, REAL_CUSTOMER_NAME).await;
    let (s1, c1) = post_create(&app, 42, create_payload(REAL_CUSTOMER_ID)).await;
    assert_eq!(s1, StatusCode::OK, "实际体: {c1}");
    let id = c1["data"]["id"].as_i64().unwrap();

    let (status, v) = put_update(&app, id, json!({ "status": "accepted" })).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "opened→accepted 应更新成功，实际体: {v}"
    );
    assert_eq!(v["data"]["status"], "accepted");
    assert_customer_name(&v["data"], Some(REAL_CUSTOMER_NAME));
}

// =========================================================
// 2) 源码扫描防回潮锁（无 DB）
// =========================================================

/// 从源码截取一个顶层块：anchor 起，到首个 "\n}" 结束；先剔除 `\r` 兼容 CRLF
/// （先例：contract_wave2_after_sales_create_test.rs::extract_block）
fn extract_block(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n}")
        .unwrap_or_else(|| panic!("块结束定位失败: {anchor}"));
    src[i..i + j + 2].to_string()
}

/// DTO 锁：customer_name 必须为 Option<String> 且 derive FromQueryResult
/// （into_model 视图对象的唯一合法形态）
#[test]
fn source_scan_dto_customer_name_field() {
    let src = include_str!("../src/models/custom_order_response_dto.rs").replace('\r', "");
    let struct_pos = src
        .find("pub struct AfterSalesInfo")
        .expect("AfterSalesInfo 锚点丢失");
    let block = extract_block(&src, "pub struct AfterSalesInfo");
    assert!(
        block.contains("pub customer_name: Option<String>,"),
        "AfterSalesInfo 必须声明 customer_name: Option<String>，实际块:\n{block}"
    );
    // struct 之前最近的 #[derive(...)] 即本结构的 derive 区
    let derive_pos = src[..struct_pos]
        .rfind("#[derive")
        .expect("derive 锚点丢失");
    let derive = &src[derive_pos..struct_pos];
    assert!(
        derive.contains("FromQueryResult"),
        "AfterSalesInfo 必须 derive FromQueryResult 才能 into_model，实际 derive 区:\n{derive}"
    );
}

/// service 锁：读侧两条链路都必须 column_as + LeftJoin + into_model 单次富化查询，
/// 且 customer_name 不得出现任何构造期拼装（format! / 常量假名 / into_tuple 旁路）
#[test]
fn source_scan_service_join_chain_and_no_fake_name() {
    let src = include_str!("../src/services/custom_order_aftersales_service.rs");
    let clean = src.replace('\r', "");
    for anchor in ["pub async fn list_by_order", "pub async fn find_dto_by_id"] {
        let block = extract_block(&clean, anchor);
        for marker in [
            "column_as(customer::Column::CustomerName, \"customer_name\")",
            "JoinType::LeftJoin",
            "after_sales::Relation::Customer.def()",
            "into_model::<AfterSalesInfo>",
        ] {
            assert!(
                block.contains(marker),
                "{anchor} 缺富化链路锚点 {marker}，实际块:\n{block}"
            );
        }
    }
    assert!(
        !clean.contains("customer_name: Some("),
        "service 禁止手工构造 customer_name（Some(format!(..) 拼装/假名）"
    );
    assert!(
        !clean.contains("into_tuple"),
        "本场景出 DTO 全列，into_tuple 旁路不得出现"
    );
}

/// handler 锁：4 处 AfterSalesInfo 字面量构造点全部消失；create/update 出参
/// 必须经 find_dto_by_id 富化回读，禁止 None 蒙混或本地拼装
#[test]
fn source_scan_handler_no_literal_construction_and_reads_back() {
    let src = include_str!("../src/handlers/custom_order_handler.rs");
    let clean = src.replace('\r', "");
    assert!(
        !clean.contains("AfterSalesInfo {"),
        "handler 不得再残留 AfterSalesInfo 字面量构造点（customer_name 无真实来源）"
    );
    for anchor in [
        "pub async fn create_after_sales",
        "pub async fn update_after_sales",
    ] {
        let block = extract_block(&clean, anchor);
        assert!(
            block.contains("find_dto_by_id"),
            "{anchor} 必须写后回读带 JOIN 的富化查询出参，实际块:\n{block}"
        );
    }
    assert!(
        !clean.contains("customer_name:"),
        "handler 不得再出现 customer_name 字段赋值（值只允许来自 service 富化查询结果）"
    );
}

/// 前端锁：接口声明可空但必出的 customer_name；客户列必须显示客户名，
/// 禁止把 row.customer_id 当名称渲染（本列旧形态回潮）
#[test]
fn source_scan_frontend_displays_customer_name_not_id() {
    let api_src = include_str!("../../frontend/src/api/custom-order.ts");
    let iface = extract_block(&api_src.replace('\r', ""), "export interface AfterSales {");
    assert!(
        iface.contains("customer_name: string | null;"),
        "AfterSales 必须声明 customer_name: string | null（后端必出键，null=客户行缺失），实际块:\n{iface}"
    );

    let panel = include_str!("../../frontend/src/components/AfterSalesPanel.vue").replace('\r', "");
    assert!(
        panel.contains("row.customer_name"),
        "客户列必须渲染 customer_name"
    );
    assert!(
        !panel.contains("{{ row.customer_id }}"),
        "客户列禁止回潮为直接显示 customer_id 数字冒充名称"
    );
}
