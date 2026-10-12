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
//! Option 语义说明：`after_sales.customer_id` 实体为 NOT NULL i32 外键，真库
//! （路线一：`test_common::setup_test_db()` → 已迁移 PostgreSQL）上该列带
//! `REFERENCES customers(id)` FK，"客户行缺失"的新行在写入层即被数据库拒绝——
//! 建单拒绝形态由 `create_with_missing_customer_is_rejected_visibly` 锁定；
//! DTO 的 Option 键必出形态由"客户存在"两态出参 + 源码扫描锁共同覆盖。
//!
//! 覆盖策略（全部真实行为，无 mock）：
//! - 真库 PostgreSQL（迁移产出的 after_sales / customers / custom_orders / products
//!   真实表，真实表名/列名：`models/customer.rs:9` table_name="customers"、
//!   `:19` pub customer_name）；FK 前置链：售后→定制订单→客户/产品，用例自种子
//! - "客户存在"态：创建响应 / 列表 / 更新响应三端 customer_name 忠实回显真实名
//! - "客户行缺失"态（真库 FK 实证）：建单必须被拒且出可见错误信封，
//!   禁止静默成功/裸 panic；已存行的出参 customer_name 键必存在，
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

mod test_common;
use test_common::setup_test_db;

const REAL_CUSTOMER_ID: i64 = 7;
const REAL_CUSTOMER_NAME: &str = "滨海针织有限公司";
const SECOND_CUSTOMER_ID: i64 = 8;
const SECOND_CUSTOMER_NAME: &str = "远东纺织厂";
const MISSING_CUSTOMER_ID: i64 = 999;
/// 夹具代表操作人身份（self 数据范围）。after_sales 三端点在读侧对父定制单执行
/// `check_resource_owner_by_member_scope` 行级归属门：self 作用域仅能触达
/// `created_by == 本人` 的行（NULL 归属按最小权限拒绝，非过度收紧）。真实建单流程
/// （`custom_order_handler::create_custom_order` → service.create_draft）必然写入
/// created_by，故夹具 raw seed 的父单也必须写入该列并等于操作人，才能以有权身份
/// 复现"本应可达"的读侧回显。
const ACTING_USER_ID: i32 = 100;

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

/// 真库 customers（m0001 基础表：customer_code/customer_name NOT NULL）种子
async fn seed_customer(db: &sea_orm::DatabaseConnection, id: i32, code: &str, name: &str) {
    // 预构造 Statement 执行走 execute_raw（sea-orm 2.0.2 ConnectionTrait，同仓内
    // 已编译写法 src/utils/number_generator.rs:263）；execute(&S: StatementBuilder) 不适用
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        // 波0 夹具补 NULL 地雷：customers.customer_type 模型非 Option（models/customer.rs:65）
        // 而 DDL 可空，省略该列的种下行按模型读出即 SeaORM 类型错；补真实白名单值（唯一词表模块）。
        format!(
            "INSERT INTO customers (id, customer_code, customer_name, customer_type) VALUES ($1, $2, $3, '{}')",
            bingxi_backend::constants::customer_type::RETAIL
        ),
        vec![id.into(), code.to_string().into(), name.to_string().into()],
    ))
    .await
    .unwrap_or_else(|e| panic!("seed 客户失败: {e}"));
}

/// FK 前置链：after_sales.custom_order_id → custom_orders → (customer_id, product_id)，
/// 用例自种子最小合法行（列名/取值与迁移 m0044 建表逐一对应）
async fn seed_custom_order(db: &sea_orm::DatabaseConnection, order_id: i64, customer_id: i32) {
    const PRODUCT_ID: i32 = 5;
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO products (id, code, name) VALUES ($1, $2, $3)",
        vec![
            PRODUCT_ID.into(),
            format!("W3A-PROD-{order_id}").into(),
            "测试面料产品".to_string().into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("seed 产品失败（custom_orders FK 前置）: {e}"));
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO custom_orders
               (id, order_no, customer_id, product_id, spec, quantity, created_by)
           VALUES ($1, $2, $3, $4, '180cm*120g', 100.00, $5)"#,
        vec![
            order_id.into(),
            format!("W3A-CO-{order_id}").into(),
            customer_id.into(),
            PRODUCT_ID.into(),
            // created_by 与夹具操作人同源：见 ACTING_USER_ID 说明（self 归属门要求本人行）
            (ACTING_USER_ID as i64).into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("seed 定制订单失败（after_sales FK 前置）: {e}"));
}

/// 返回 (app, db)：path id 42 对应的 custom_orders 行与真实客户 7 均已自种子
async fn seeded_app() -> (Router, sea_orm::DatabaseConnection) {
    let db = setup_test_db().await;
    seed_customer(&db, REAL_CUSTOMER_ID as i32, "W3A-C07", REAL_CUSTOMER_NAME).await;
    seed_custom_order(&db, 42, REAL_CUSTOMER_ID as i32).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
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
        .layer(from_fn_with_state(make_auth(ACTING_USER_ID), inject_auth));
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
    let (app, _db) = seeded_app().await;
    let (status, v) = post_create(&app, 42, create_payload(REAL_CUSTOMER_ID)).await;
    assert_eq!(status, StatusCode::OK, "实际体: {v}");
    assert_eq!(v["code"], 200);
    assert_customer_name(&v["data"], Some(REAL_CUSTOMER_NAME));
    // customer_id 原键如实透传不受影响（wave2 契约不回退）
    assert_eq!(v["data"]["customer_id"], json!(REAL_CUSTOMER_ID));
}

/// 列表端点逐行锁：同一订单下两张工单各挂不同真实客户，
/// 每行 customer_name 必须回显"本行"JOIN 到的真名（证明逐行富化、非缓存/拼装）
#[tokio::test]
async fn list_endpoint_echoes_real_name_and_null_for_missing_customer() {
    let (app, db) = seeded_app().await;
    seed_customer(
        &db,
        SECOND_CUSTOMER_ID as i32,
        "W3A-C08",
        SECOND_CUSTOMER_NAME,
    )
    .await;
    let (s1, c1) = post_create(&app, 42, create_payload(REAL_CUSTOMER_ID)).await;
    assert_eq!(s1, StatusCode::OK, "实际体: {c1}");
    let (s2, c2) = post_create(&app, 42, create_payload(SECOND_CUSTOMER_ID)).await;
    assert_eq!(s2, StatusCode::OK, "第二个真实客户建单应成功，实际体: {c2}");
    assert_customer_name(&c2["data"], Some(SECOND_CUSTOMER_NAME));

    let (status, lv) = get_list(&app, 42).await;
    assert_eq!(status, StatusCode::OK, "列表应 200，实际体: {lv}");
    let items = lv["data"]["items"]
        .as_array()
        .expect("列表应为 PagedResponse{items,...}");
    assert_eq!(items.len(), 2, "应恰有 2 条工单，实际: {lv}");

    let named = find_item(items, c1["data"]["id"].as_i64().unwrap());
    assert_customer_name(&named, Some(REAL_CUSTOMER_NAME));
    assert_eq!(named["customer_id"], json!(REAL_CUSTOMER_ID));

    let second = find_item(items, c2["data"]["id"].as_i64().unwrap());
    assert_customer_name(&second, Some(SECOND_CUSTOMER_NAME));
    assert_eq!(
        second["customer_id"],
        json!(SECOND_CUSTOMER_ID),
        "外键值本身仍须如实透传"
    );
}

/// 真库 FK 实证（路线一）：after_sales.customer_id 带
/// `REFERENCES customers(id)`（迁移 m0044），"客户行缺失"的建单在写入层
/// 必被数据库拒绝——不得静默成功、不得裸 panic，出参必须是可见错误信封。
/// （旧 sqlite 自建表无 FK，才有"建单成功 + customer_name=null"态；
/// 该态在生产 schema 下不可达，Option 键形态由 DTO 源码锁与前端锁继续锁定）
#[tokio::test]
async fn create_with_missing_customer_is_rejected_by_real_fk() {
    let (app, _db) = seeded_app().await;
    let (status, v) = post_create(&app, 42, create_payload(MISSING_CUSTOMER_ID)).await;
    assert_ne!(
        status,
        StatusCode::OK,
        "客户行缺失时 FK 必须拒绝建单，禁止静默成功造假工单，实际体: {v}"
    );
    assert!(
        v.get("code").is_some() && v.get("message").is_some(),
        "拒绝必须出标准错误信封（code+message 可见），禁止空体/裸 panic，实际: {v}"
    );
}

/// 更新端点出参锁：PUT 状态流转后响应仍带真实客户名（同一条回读链路）
///
/// 注意（真库首跑判责点）：服务状态机权威词表含 accepted/evaluated
/// （`services/custom_order_aftersales_service.rs::is_valid_transition`），而迁移
/// m0044 建表的 `chk_aftersales_status` CHECK 只含
/// opened/processing/resolved/closed/rejected——若真库首跑此用例报 23514，
/// 属「疑迁移/表结构缺口」（约束与写入方词表漂移），不是本断言错误，
/// 禁止反向改断言或改服务迁就约束。
#[tokio::test]
async fn update_response_carries_real_customer_name() {
    let (app, _db) = seeded_app().await;
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

/// impl 成员函数版窗（4 空格收口，同 `contract_wave5_purchase_actual_delivery_writeback_test.rs::extract_impl_method` 先例）。
/// service 两条链路函数是 impl 成员、顶层闭合为 `    }`：若沿用 extract_block 的 `"\n}"`
/// 收口，第一条函数的窗会吞进第二条函数——两条断言实际共享同一窗，单侧掉链路仍绿
/// （假绿形态；现按各自函数体判定，收紧判据）。
fn extract_impl_fn(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n    }")
        .unwrap_or_else(|| panic!("函数结束定位失败: {anchor}"));
    src[i..i + j + 6].to_string()
}

/// 只保留"代码 + 字符串字面量"：整行注释（`//`、`///`、`//!`）逐行剔除。
/// B1① 判责：本组扫描锁既有"源码不该出现 X"禁项（`AfterSalesInfo {`、
/// `customer_name:`、`row.customer_id`），极易被"旧实现曾在此构造 …"类说明注释
/// 假判违例；正向必备项若只在注释出现则是假绿。故**正反判定统一在剥注释文本上**，
/// 锚点定位也在剥注释文本上进行（防窗起点被注释里的同名符号骗走）。
/// 按行剥（不做字符级引号配平）的理由同 `contract_wave5_outsource_issue_guard_test.rs`
/// 先例：宁少剥不误吃代码。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 剔除 HTML 注释 `<!-- … -->`（.vue 模板/样式区的注释形态，`//` 行剥不适用），
/// 之后再过 code_only 处理 `<script>` 区。简单非嵌套切分——vue 模板注释不嵌套。
fn strip_html_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(i) = rest.find("<!--") {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find("-->").unwrap_or(rest.len() - 3);
        rest = &rest[end + 3..];
    }
    out.push_str(rest);
    out
}

/// DTO 锁：customer_name 必须为 Option<String> 且 derive FromQueryResult
/// （into_model 视图对象的唯一合法形态）
#[test]
fn source_scan_dto_customer_name_field() {
    // 锚点与 derive 区定位都在剥注释文本上进行（防注释里的同名符号骗走窗起点）
    let src = code_only(include_str!("../src/models/custom_order_response_dto.rs"));
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
    // 判据在剥注释文本上执行：service 里"修复前这里是拼装假名"类自述注释
    // 不是执行体，命中禁词属假判违例（B1① all_lead_pii 同型）。
    let clean = code_only(include_str!(
        "../src/services/custom_order_aftersales_service.rs"
    ));
    for anchor in ["pub async fn list_by_order", "pub async fn find_dto_by_id"] {
        let block = extract_impl_fn(&clean, anchor);
        // 正向必备项在去空白规范形上比对：`column_as(...)` 被 rustfmt 折行/改对齐
        // 不参与判定（B1② 纪律；同 `contract_wave5_outsource_issue_guard_test.rs::canon`）。
        let block_flat: String = block.chars().filter(|c| !c.is_whitespace()).collect();
        for marker in [
            "column_as(customer::Column::CustomerName, \"customer_name\")",
            "JoinType::LeftJoin",
            "after_sales::Relation::Customer.def()",
            "into_model::<AfterSalesInfo>",
        ] {
            let flat_marker: String = marker.chars().filter(|c| !c.is_whitespace()).collect();
            assert!(
                block_flat.contains(&flat_marker),
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
    // 同 service 锁：禁项与窗定位都在剥注释文本上；handler 的文档注释若描述
    // "旧形态 AfterSalesInfo { ... } 已移除"，按原文扫描即假判回潮（B1①）。
    let clean = code_only(include_str!("../src/handlers/custom_order_handler.rs"));
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
    // TS 侧先剥整行注释再定位接口块；锚点不带 `{`，排版把左花括号挪行不参与判定。
    let api_src = code_only(include_str!("../../frontend/src/api/custom-order.ts"));
    let iface = extract_block(&api_src, "export interface AfterSales");
    assert!(
        iface.contains("customer_name: string | null;"),
        "AfterSales 必须声明 customer_name: string | null（后端必出键，null=客户行缺失），实际块:\n{iface}"
    );

    let panel_raw = include_str!("../../frontend/src/components/AfterSalesPanel.vue");
    // .vue：先剥 HTML 注释再剥 `//` 行——"旧版本这里显示 row.customer_id"类
    // 说明注释命中禁词属假判违例（B1① 同型），执行体才是要锁的层。
    let panel = code_only(&strip_html_comments(panel_raw));
    assert!(
        panel.contains("row.customer_name"),
        "客户列必须渲染 customer_name"
    );
    assert!(
        !panel.contains("{{ row.customer_id }}"),
        "客户列禁止回潮为直接显示 customer_id 数字冒充名称"
    );
}
