//! 客户信用读侧 customer_name 富化与 {id} 自然键语义契约锁
//!
//! ## 六要素
//! - **目的（功能）**：钉死两件事——① 客户信用三个 HTTP 出口（列表 / 详情 / 写后回读）
//!   的出参 customer_name 唯一来源是“按 customer_id LEFT JOIN customers”的真值，
//!   与本表冗余列 customer_credit_ratings.customer_name 的落库值无关；② /customer-credits/{id}
//!   系列的 {id} 是**客户自然键** customer_credit_ratings.customer_id（= customers.id），
//!   不是本表行主键 id。
//! - **判据**：新建信用行（写入路径从不给冗余列赋值 ⇒ 库内为 NULL）后，列表与详情出参
//!   的 customer_name 必须等于 customers 的真实客户名，同时 SQL 直查证明库内该列仍是
//!   NULL（真名来自读侧富化，不是把名字落库后直读）；库内存量为 NULL 或为过期旧名的行，
//!   出参同样回显真名（存量行无需回填迁移）。语义侧双向锁：按 customer_id 打详情/更新
//!   命中本客户行，按行主键打同一路径必须 404 且**零写入**（错行写库是本端点最坏形态）。
//!   出参键集另用序列化比对锁与 customer_credit::Model 逐键同形（漏键=前端列恒空假绿）。
//! - **依据**：backend/src/services/customer_credit_service.rs（CustomerCreditView +
//!   view_select 富化基底 + get_list / get_view_by_customer_id / get_view_by_row_id）、
//!   backend/src/handlers/customer_credit_handler.rs（list/get/create/update/rating 出参与
//!   {id} 口径文档）、backend/src/routes/crm.rs（/customers/{id}/credit 与
//!   /customer-credits/{id} 共用 get_credit）、backend/migration/src/domain/business/
//!   m0012（表与客户名冗余列）+ m0082（customer_id 全表唯一 + FK ⇒ 每客户恒一行，
//!   客户行必然存在，JOIN 命中真名）。
//! - **夹具**：真 PostgreSQL（test_common 的 setup_test_db()，缺 TEST_DATABASE_URL 或指向
//!   sqlite 直接 panic）⇒ 本文件真库用例只有 CI 活库有结果，本机仅证明编译与格式。
//!   customer_credit_ratings / customers 均**不在** SEALED_REFERENCE_TABLES
//!   （services/test_common.rs 清单里没有这两张表），属逐用例 TRUNCATE 的业务表且
//!   RESTART IDENTITY，故新插行自增 id 从 1 起、与显式种的 9971xx 行主键不冲突。
//!   ⚠️ 执行必须 cargo test -- --test-threads=1：夹具的全表 TRUNCATE 在多测试线程下互相清表。
//! - **覆盖**：GET /customer-credits、GET /customer-credits/{id}、GET
//!   /customers/{id}/credit、POST /customer-credits、PUT /customer-credits/{id}、
//!   POST /customer-credits/{id}/rating 六个出口 + 打印数据装配（源码锁）+ 出参键集同形锁。
//! - **边界**：GET /customer-credits/{id}/print 的 {id} 是**行主键**（print_service 按
//!   行主键取单行），与上面六个端点不同义，本锁按源码锁覆盖其取数链路；客户行缺失
//!   （LEFT JOIN 出 NULL）在生产 schema 下被 m0082 的 FK 挡在写入层，不可构造，
//!   故该态由“投影用 LeftJoin 而非 InnerJoin”的源码锁覆盖（InnerJoin 会整行吞掉记录）。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::customer_credit_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::customer_credit;
use bingxi_backend::services::customer_credit_service::CustomerCreditView;
use bingxi_backend::ymd;
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::{Value, json};
use tower::ServiceExt;

mod test_common;
use test_common::setup_test_db;

/// 本文件独占 997xxx id 带：客户 997011/997012/997013、显式行主键 997101/997102
const CUST_A: i32 = 997_011;
const CUST_B: i32 = 997_012;
/// 只用于“新建信用行”路径：初始无信用行
const CUST_C: i32 = 997_013;
/// 显式行主键，与客户 id 逐值错开——{id} 语义双向锁的判据前提
const ROW_PK_A: i32 = 997_101;
const ROW_PK_B: i32 = 997_102;
const NAME_A: &str = "信用富化锁客户A";
const NAME_B: &str = "信用富化锁客户B";
const NAME_C: &str = "信用富化锁客户C";
/// 行冗余列里的过期旧名：改名事件回填不到该列时的存量形态
const STALE_NAME_B: &str = "信用富化锁客户B（旧名）";
const ACTING_USER_ID: i32 = 997_001;

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

async fn query_bool(db: &DatabaseConnection, sql: &str, alias: &str) -> bool {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("回读查询失败: {e}\nSQL: {sql}"))
        .unwrap_or_else(|| panic!("回读查询应恒有行: {sql}"));
    row.try_get::<bool>("", alias)
        .unwrap_or_else(|e| panic!("{alias} 解码失败: {e}"))
}

async fn query_i64(db: &DatabaseConnection, sql: &str, alias: &str) -> i64 {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("回读查询失败: {e}\nSQL: {sql}"))
        .unwrap_or_else(|| panic!("回读查询应恒有行: {sql}"));
    row.try_get::<i64>("", alias)
        .unwrap_or_else(|e| panic!("{alias} 解码失败: {e}"))
}

/// 种真实客户行。customers.status 与 customers.payment_terms 在 DDL 里可空且无默认，
/// 实体却声明为非 Option，而新建信用行路径含整行解码，种子必须显式给足这两列真实值，
/// 否则整行解码撞 NULL 报数据库错误。credit_limit 与 owner_id 按 DDL 默认语义写实
/// （零额度、公海归属）。customer_type 受 CHECK 词表约束，取权威常量。
/// status 取主数据词表 active。
async fn seed_customer(db: &DatabaseConnection, id: i32, code: &str, name: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        format!(
            "INSERT INTO customers (id, customer_code, customer_name, customer_type, credit_limit, \
             owner_id, status, payment_terms) VALUES ($1, $2, $3, '{}', 0, 0, 'active', 30)",
            bingxi_backend::constants::customer_type::RETAIL
        ),
        vec![id.into(), code.to_string().into(), name.to_string().into()],
    ))
    .await
    .unwrap_or_else(|e| panic!("seed 客户失败（id={id}）: {e}"));
}

/// 种信用行：stored_name 为 None ⇒ 该列不写（NULL，新建信用行的真实形态）
async fn seed_credit_row(
    db: &DatabaseConnection,
    row_id: i32,
    customer_id: i32,
    stored_name: Option<&str>,
) {
    match stored_name {
        Some(name) => {
            db.execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO customer_credit_ratings (id, customer_id, customer_name, credit_level, \
                 credit_limit, status) VALUES ($1, $2, $3, 'A', 1000.00, 'active')",
                vec![
                    row_id.into(),
                    customer_id.into(),
                    name.to_string().into(),
                ],
            ))
            .await
            .unwrap_or_else(|e| panic!("seed 信用行失败（id={row_id}）: {e}"));
        }
        None => {
            db.execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO customer_credit_ratings (id, customer_id, credit_level, credit_limit, \
                 status) VALUES ($1, $2, 'A', 1000.00, 'active')",
                vec![row_id.into(), customer_id.into()],
            ))
            .await
            .unwrap_or_else(|e| panic!("seed 信用行失败（id={row_id}）: {e}"));
        }
    }
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
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

/// 挂载真实 handler（路径写法与 routes/crm.rs 一致），返回 (app, db)。
///
/// 初始状态：A 有行主键 997101 且冗余列 NULL；B 有行主键 997102 且冗余列是过期旧名；
/// C 无信用行（留给“新建”路径）。
async fn seeded_app() -> (Router, DatabaseConnection) {
    let db = setup_test_db().await;
    seed_customer(&db, CUST_A, "CW27-A", NAME_A).await;
    seed_customer(&db, CUST_B, "CW27-B", NAME_B).await;
    seed_customer(&db, CUST_C, "CW27-C", NAME_C).await;
    seed_credit_row(&db, ROW_PK_A, CUST_A, None).await;
    seed_credit_row(&db, ROW_PK_B, CUST_B, Some(STALE_NAME_B)).await;

    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/customer-credits",
            get(customer_credit_handler::list_credits).post(customer_credit_handler::create_credit),
        )
        .route(
            "/customer-credits/{id}",
            get(customer_credit_handler::get_credit).put(customer_credit_handler::update_credit),
        )
        .route(
            "/customer-credits/{id}/rating",
            post(customer_credit_handler::set_credit_rating),
        )
        .route(
            "/customers/{id}/credit",
            get(customer_credit_handler::get_credit),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(ACTING_USER_ID), inject_auth));
    (app, db)
}

async fn call(
    app: &Router,
    method: axum::http::Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(v) => builder
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    call(app, axum::http::Method::GET, uri, None).await
}

async fn post_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    call(app, axum::http::Method::POST, uri, Some(body)).await
}

async fn put_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    call(app, axum::http::Method::PUT, uri, Some(body)).await
}

/// 列表出参（分页信封的 items 行数组）中按行主键定位对象
fn find_item(items: &[Value], row_id: i32) -> Value {
    items
        .iter()
        .find(|it| it["id"].as_i64() == Some(row_id as i64))
        .unwrap_or_else(|| panic!("列表应含 id={row_id} 的信用行，实际: {items:?}"))
        .clone()
}

/// customer_name 键必出 + 真值断言的公共口径
fn assert_customer_name(obj: &Value, expected: &str) {
    assert!(
        obj.get("customer_name").is_some(),
        "出参必须恒含 customer_name 键（无名字也必须是显式键），实际: {obj}"
    );
    assert_eq!(
        obj["customer_name"],
        json!(expected),
        "customer_name 必须忠实回显 JOIN 到的真实客户名，禁止 NULL/旧名/id 拼装，实际: {obj}"
    );
}

// =========================================================
// 1) 读侧出口出参真名（含新建行与存量 NULL/旧名行）
// =========================================================

/// 列表出参锁：冗余列为 NULL 的行（A）与存着过期旧名的行（B）都必须回显 customers 真名；
/// 同时 SQL 直查证明库内该列形态未变——真名来自读侧富化，不是落库后直读。
/// 另锁列表走分页信封（行数组在 data.items、总数在 data.total），富化只改名列取值来源、不改信封形态。
#[tokio::test]
async fn list_echoes_real_customer_name_for_null_and_stale_rows() {
    let (app, db) = seeded_app().await;
    let (status, v) = get_json(&app, "/customer-credits?page=1&page_size=50").await;
    assert_eq!(status, StatusCode::OK, "列表应 200，实际体: {v}");

    let items = v["data"]["items"]
        .as_array()
        .expect("列表 data.items 必须是行数组");
    assert_eq!(items.len(), 2, "应恰有 2 条信用行，实际: {v}");

    assert_customer_name(&find_item(items, ROW_PK_A), NAME_A);
    assert_customer_name(&find_item(items, ROW_PK_B), NAME_B);
    // 外键值本身仍如实透传（富化不改键集，只改名列取值来源）
    assert_eq!(find_item(items, ROW_PK_A)["customer_id"], json!(CUST_A));
    assert_eq!(find_item(items, ROW_PK_B)["customer_id"], json!(CUST_B));

    assert!(
        query_bool(
            &db,
            &format!(
                "SELECT customer_name IS NULL AS ok FROM customer_credit_ratings WHERE id={ROW_PK_A}"
            ),
            "ok",
        )
        .await,
        "库内 A 行冗余列必须仍是 NULL——否则本锁退化成「读的是落库值」的假绿"
    );
    assert!(
        query_bool(
            &db,
            &format!(
                "SELECT customer_name = '{STALE_NAME_B}' AS ok \
                  FROM customer_credit_ratings WHERE id={ROW_PK_B}"
            ),
            "ok",
        )
        .await,
        "库内 B 行冗余列必须仍是过期旧名——出参 {NAME_B} 只可能来自 JOIN 真值"
    );
}

/// 新建路径出参锁：POST 建信用行（写入路径从不给冗余列赋值）→ 创建响应、列表、
/// 详情三处 customer_name 都是真实客户名，且库内该列仍为 NULL。
/// 这是“新建信用行的 customer_name 恒空”缺陷的正向锁。
#[tokio::test]
async fn newly_created_credit_row_carries_real_name_on_all_read_paths() {
    let (app, db) = seeded_app().await;
    let (status, created) = post_json(
        &app,
        "/customer-credits",
        json!({"customer_id": CUST_C, "credit_level": "A", "credit_limit": 5000, "credit_days": 30}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "创建应 200，实际体: {created}");
    assert_customer_name(&created["data"], NAME_C);
    let new_row_id = created["data"]["id"]
        .as_i64()
        .expect("创建响应应回行主键 id");

    // 行主键与客户 id 逐值不同（TRUNCATE RESTART IDENTITY ⇒ 新行 id=1），
    // 后续 {id} 语义双向锁的前提在此钉住
    assert_ne!(
        new_row_id, CUST_C as i64,
        "本用例要求行主键 != customer_id，否则语义双向锁失去判据"
    );

    let (lstatus, lv) = get_json(&app, "/customer-credits?page=1&page_size=50").await;
    assert_eq!(lstatus, StatusCode::OK, "列表应 200，实际体: {lv}");
    let items = lv["data"]["items"]
        .as_array()
        .expect("列表 data.items 必须是行数组");
    assert_customer_name(&find_item(items, new_row_id as i32), NAME_C);

    let (dstatus, dv) = get_json(&app, &format!("/customer-credits/{CUST_C}")).await;
    assert_eq!(dstatus, StatusCode::OK, "详情应 200，实际体: {dv}");
    assert_customer_name(&dv["data"], NAME_C);

    assert!(
        query_bool(
            &db,
            &format!(
                "SELECT customer_name IS NULL AS ok FROM customer_credit_ratings WHERE id={new_row_id}"
            ),
            "ok",
        )
        .await,
        "新建信用行的冗余列仍应为 NULL（读侧富化方案不落库名字），出参真名来自 JOIN"
    );
}

/// 详情与 /customers/{id}/credit 两个挂载点共用同一 handler：两处 {id} 同义
/// （客户自然键），同一请求必须返回同一行同一真名。
#[tokio::test]
async fn detail_and_customer_alias_mount_return_same_enriched_row() {
    let (app, _db) = seeded_app().await;
    let (s1, v1) = get_json(&app, &format!("/customer-credits/{CUST_A}")).await;
    assert_eq!(
        s1,
        StatusCode::OK,
        "按 customer_id 打详情应 200，实际体: {v1}"
    );
    assert_eq!(v1["data"]["id"], json!(ROW_PK_A));
    assert_customer_name(&v1["data"], NAME_A);

    let (s2, v2) = get_json(&app, &format!("/customers/{CUST_A}/credit")).await;
    assert_eq!(
        s2,
        StatusCode::OK,
        "客户前缀下的同一 handler 应 200，实际体: {v2}"
    );
    assert_eq!(
        v1["data"], v2["data"],
        "两个挂载点必须同语义（客户自然键），否则共享 handler 的其中一侧取错行"
    );
}

/// {id} 语义反向锁：把行主键当 {id} 打详情必须 404（客户不存在 ⇒ 无评级行），
/// 命中不了别的客户行。若有人把 {id} 改回行主键解释，本用例必红。
#[tokio::test]
async fn row_primary_key_is_not_accepted_as_path_id() {
    let (app, _db) = seeded_app().await;
    let (status, v) = get_json(&app, &format!("/customer-credits/{ROW_PK_A}")).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "行主键 {ROW_PK_A} 不是本端点的路径键（该值不是任何 customers.id），应 404，实际体: {v}"
    );
    assert_eq!(
        v["code"], "NOT_FOUND",
        "错误码须如实 NOT_FOUND，实际体: {v}"
    );
}

// =========================================================
// 2) 写侧出口出参同源 + 路径键反向零写入
// =========================================================

/// 更新出参锁：PUT 按 customer_id 命中本客户行，响应 customer_name 为真名
/// （写后回读同一条富化链路），额度落库真值如实回读。
#[tokio::test]
async fn update_response_carries_real_name_and_persists_limit() {
    let (app, db) = seeded_app().await;
    let (status, v) = put_json(
        &app,
        &format!("/customer-credits/{CUST_A}"),
        json!({"credit_level": "AA", "credit_limit": 6000}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "按 customer_id 更新应 200，实际体: {v}"
    );
    assert_customer_name(&v["data"], NAME_A);
    assert_eq!(v["data"]["id"], json!(ROW_PK_A), "必须更新本客户行");
    assert_eq!(v["data"]["credit_level"], json!("AA"));

    assert!(
        query_bool(
            &db,
            &format!(
                "SELECT credit_limit = 6000 AND used_credit = 0 AS ok \
                  FROM customer_credit_ratings WHERE id={ROW_PK_A}"
            ),
            "ok",
        )
        .await,
        "更新必须落在行主键 {ROW_PK_A} 上且额度真值为 6000"
    );
    assert_eq!(
        query_i64(
            &db,
            &format!(
                "SELECT COUNT(*) AS cnt FROM customer_credit_ratings WHERE customer_id={CUST_A}"
            ),
            "cnt",
        )
        .await,
        1,
        "同一客户第二次写必须更新原行而不是插新行（单行契约）"
    );
}

/// 写侧路径键反向锁：把行主键当 {id} 打 PUT 不得写错行——引用的客户不存在即 404，
/// 且原有行的额度零变化、表内零新增行。
#[tokio::test]
async fn update_with_row_primary_key_writes_nothing() {
    let (app, db) = seeded_app().await;
    let (status, v) = put_json(
        &app,
        &format!("/customer-credits/{ROW_PK_A}"),
        json!({"credit_limit": 1}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "客户 {ROW_PK_A} 不存在时更新必须 404，禁止静默命中别的客户行，实际体: {v}"
    );
    assert!(
        query_bool(
            &db,
            &format!(
                "SELECT credit_limit = 1000 AS ok FROM customer_credit_ratings WHERE id={ROW_PK_A}"
            ),
            "ok",
        )
        .await,
        "被拒的更新必须零写入：行主键 {ROW_PK_A} 的额度仍为种子值 1000"
    );
    assert_eq!(
        query_i64(
            &db,
            "SELECT COUNT(*) AS cnt FROM customer_credit_ratings",
            "cnt"
        )
        .await,
        2,
        "被拒的更新不得新插任何行（种子 2 行仍在）"
    );
}

/// 设级端点出参锁：POST /customer-credits/{id}/rating 的客户由请求体
/// customer_id 指定（路径 {id} 本 handler 不读取），出参走同一条富化链路。
#[tokio::test]
async fn set_rating_response_carries_real_name() {
    let (app, _db) = seeded_app().await;
    let (status, v) = post_json(
        &app,
        &format!("/customer-credits/{CUST_B}/rating"),
        json!({"customer_id": CUST_B, "credit_level": "B", "credit_limit": 700}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "设级应 200，实际体: {v}");
    assert_customer_name(&v["data"], NAME_B);
    assert_eq!(v["data"]["id"], json!(ROW_PK_B));
}

// =========================================================
// 3) 出参键集同形锁（无库，本机即可跑）
// =========================================================

/// 键集同形锁：同一行取值下，富化视图与实体序列化结果必须逐键逐值相同（键名、
/// Decimal/日期编码形态、可空性）。富化改的是名列取值来源，不是出参形状；
/// 本表新增列而视图漏列时，本锁直接点名（本仓“出参漏键 ⇒ 前端列恒空假绿”的形态）。
#[test]
fn view_serializes_identically_to_entity_model() {
    let model = customer_credit::Model {
        id: 1,
        customer_id: 2,
        customer_name: Some("落库值不参与出参".to_string()),
        credit_level: Some("A".to_string()),
        credit_score: Some(80),
        credit_limit: rust_decimal::Decimal::new(5_000_000, 2),
        used_credit: rust_decimal::Decimal::new(1_000_000, 2),
        available_credit: rust_decimal::Decimal::new(49_000_000, 2),
        credit_days: Some(30),
        last_assessment_date: Some(ymd!(2026, 1, 1)),
        next_assessment_date: Some(ymd!(2026, 12, 31)),
        status: "active".to_string(),
        created_by: Some(3),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let view = CustomerCreditView {
        id: model.id,
        customer_id: model.customer_id,
        customer_name: Some(NAME_A.to_string()),
        credit_level: model.credit_level.clone(),
        credit_score: model.credit_score,
        credit_limit: model.credit_limit,
        used_credit: model.used_credit,
        available_credit: model.available_credit,
        credit_days: model.credit_days,
        last_assessment_date: model.last_assessment_date,
        next_assessment_date: model.next_assessment_date,
        status: model.status.clone(),
        created_by: model.created_by,
        created_at: model.created_at,
        updated_at: model.updated_at,
    };

    let mv = serde_json::to_value(&model).expect("实体应可序列化");
    let vv = serde_json::to_value(&view).expect("视图应可序列化");
    let mut mk: Vec<&String> = mv.as_object().expect("实体出参是对象").keys().collect();
    let mut vk: Vec<&String> = vv.as_object().expect("视图出参是对象").keys().collect();
    mk.sort();
    vk.sort();
    assert_eq!(mk, vk, "视图键集必须与实体逐键同名（漏键=前端列恒空假绿）");

    // created_at/updated_at 由 Utc::now() 现取，比对时剔除这两键，其余逐值相等
    let strip_time = |mut v: Value| {
        let obj = v.as_object_mut().expect("对象");
        obj.remove("created_at");
        obj.remove("updated_at");
        obj.insert("customer_name".to_string(), json!("X"));
        v
    };
    assert_eq!(
        strip_time(mv.clone()),
        strip_time(vv.clone()),
        "除 customer_name 取值来源外，视图与实体的出参编码必须逐值一致"
    );
}

// =========================================================
// 4) 源码扫描防回潮锁（无库，本机即可跑）
// =========================================================

/// 整行注释剔除（//、///、//!）：禁项与锚点都必须只看执行体，
/// 否则注释里复述的旧形态会被假判回潮。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从源码截取一个顶层块：anchor 起，到首个 \n} 结束（先例
/// contract_wave3_after_sales_customer_name_test.rs 的 extract_block）
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

/// impl 成员函数窗（4 空格收口，同 contract_wave3 的 extract_impl_fn）：
/// 用 \n} 收口会让相邻函数共享同一窗，单侧掉链路仍绿。
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

/// 去空白规范形（rustfmt 折行/改对齐不参与判定）
fn canon(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// service 富化链路锁：投影显式剔除本表冗余名列、单次 LEFT JOIN 取真名、
/// 三个读出口都 into_model::<CustomerCreditView>；禁止 InnerJoin（缺客户会整行吞记录）。
#[test]
fn source_scan_service_enrichment_chain() {
    let clean = code_only(include_str!("../src/services/customer_credit_service.rs"));
    let block = extract_impl_fn(&clean, "fn view_select()");
    for marker in [
        "select_only()",
        "column_as(customer::Column::CustomerName, \"customer_name\")",
        "JoinType::LeftJoin",
    ] {
        assert!(
            canon(&block).contains(&canon(marker)),
            "view_select 缺富化链路锚点 {marker}，实际块:\n{block}"
        );
    }
    assert!(
        !canon(&block).contains(&canon("JoinType::InnerJoin")),
        "富化基底禁止 InnerJoin：客户行缺失时会把整条信用记录从列表/详情里吞掉"
    );
    // 冗余列不得进入投影（否则同名列出现两次，取值来源随驱动列序漂移）
    assert!(
        !canon(&block).contains(&canon("column(customer_credit::Column::CustomerName)")),
        "投影不得选择本表冗余 customer_name 列，真名唯一来源是 customers JOIN"
    );
    for anchor in [
        "pub async fn get_list(",
        "pub async fn get_view_by_customer_id(",
        "pub async fn get_view_by_row_id(",
    ] {
        let fn_block = extract_impl_fn(&clean, anchor);
        assert!(
            canon(&fn_block).contains(&canon("into_model::<CustomerCreditView>")),
            "{anchor} 必须走 into_model::<CustomerCreditView>，实际块:\n{fn_block}"
        );
        assert!(
            canon(&fn_block).contains(&canon("Self::view_select()")),
            "{anchor} 必须复用 view_select 同一基底，实际块:\n{fn_block}"
        );
    }
    assert!(
        !clean.contains("customer_name: Some("),
        "service 禁止手工构造 customer_name（拼装名/假名的落点）"
    );
}

/// handler 出口锁：四个 HTTP 出口不得再直出 customer_credit::Model（直出即把未赋值的
/// 冗余列当出参），写侧三端点必须写后回读富化视图。
#[test]
fn source_scan_handler_uses_view_on_all_exits() {
    let clean = code_only(include_str!("../src/handlers/customer_credit_handler.rs"));
    assert!(
        !clean.contains("customer_credit::Model"),
        "handler 不得再出现 customer_credit::Model 出参类型（列表/详情/写侧都要富化视图）"
    );
    for anchor in [
        "pub async fn list_credits(",
        "pub async fn get_credit(",
        "pub async fn set_credit_rating(",
    ] {
        let block = extract_block(&clean, anchor);
        assert!(
            canon(&block).contains(&canon("CustomerCreditView")),
            "{anchor} 出参必须是 CustomerCreditView，实际块:\n{block}"
        );
    }
    for anchor in [
        "pub async fn create_credit(",
        "pub async fn update_credit(",
        "pub async fn set_credit_rating(",
    ] {
        let block = extract_block(&clean, anchor);
        assert!(
            canon(&block).contains(&canon("read_back_view(")),
            "{anchor} 必须写后回读富化视图（出参与列表/详情同源），实际块:\n{block}"
        );
    }
    assert!(
        !clean.contains("customer_name: Some("),
        "handler 禁止手工构造 customer_name"
    );
}

/// 打印装配锁：打印数据的客户名必须走行主键富化回读，不得回到直读实体冗余列。
#[test]
fn source_scan_print_data_uses_enriched_row_read() {
    let clean = code_only(include_str!("../src/services/print_service.rs"));
    let block = extract_impl_fn(&clean, "async fn get_customer_credit_print_data(");
    assert!(
        canon(&block).contains(&canon("get_view_by_row_id(id)")),
        "打印装配必须用 get_view_by_row_id 富化取客户名，实际块:\n{block}"
    );
    assert!(
        !canon(&block).contains(&canon("customer_credit::Entity::find_by_id")),
        "打印装配禁止直读实体（冗余列未赋值 ⇒ 打印件客户名空白），实际块:\n{block}"
    );
}
