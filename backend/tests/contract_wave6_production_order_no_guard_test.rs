//! 生产订单单号禁手输契约锁(任务 #153 缺陷3)
//!
//! 锁定形态(修复后):
//! - `handlers/production_order_handler.rs::CreateProductionOrderPayload`:
//!   DTO 上**不存在** order_no 可写字段;旧前端仍携带非空 order_no 时经 serde(flatten)
//!   残差映射识别并 `tracing::warn`(不静默、不透传服务层)。
//! - `services/production_order_ops/types.rs::CreateProductionOrderRequest`:
//!   无 order_no 字段(类型层面无注入口,编译期杜绝任何调用方旁路写号);
//!   planned_quantity 为必填 Decimal(NOT NULL m0007:78,不设 Option 兜底)。
//! - `services/production_order_ops/crud.rs::create`:一律服务端取号
//!   (DocumentNumberGenerator,`PO{YYYYMMDD}{3位流水}`);resolve_order_no
//!   (用户号查重后原样入库)已整体移除。
//! - 默认值单一来源:status/priority 缺省走 DB DEFAULT(m0007:84/85),
//!   代码侧不再 unwrap_or_default 把"未指定"塌成 0。
//!
//! 修复前缺陷:创建端点接受前端传入的 order_no 并原样入库——可伪造单号、
//! 可撞 order_no UNIQUE 约束(m0007:75)导致裸 500、旁路取号器号段。
//!
//! 覆盖策略(全部分层,无 mock):
//! - sqlite 端到端(真实行为):携带伪造单号的创建请求走完 DTO 反序列化 +
//!   服务链后,取号环节(sqlite 方言不支持 pg_advisory_xact_lock)显式失败,
//!   订单行零写入——证明"取号先于写入、客户号永不可达 INSERT";
//!   planned_quantity 缺键在 DTO 反序列化层即 400(required 语义)。
//! - `#[ignore]` 活库(TEST_DATABASE_URL→PG,ci-test-rust-ignored 执行):
//!   伪造单号被忽略、响应单号为服务端 PO 序列、status/priority 由 DB 默认生效。
//! - 源码扫描防回潮锁:payload/service DTO 字段、create 时序、NotSet 默认值纪律。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::production_order_handler::{self, CreateProductionOrderPayload};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{product, production_order};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, Statement,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

fn make_auth(user_id: i32, scope: Option<&str>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: scope.map(|s| s.to_string()),
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

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            builder.body(Body::from(v.to_string())).unwrap()
        }
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        // 非 JSON 体(如 axum Json 提取器拒绝的纯文本)统一包成字符串供断言
        Err(_) => json!({ "raw_body": String::from_utf8_lossy(&bytes).to_string() }),
    };
    (status, value)
}

// =========================================================
// 1) sqlite 自建表(products/boms 列与 models::Entity 逐列对应,
//    production_orders 用于零写入回读)
// =========================================================

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

async fn create_tables(db: &sea_orm::DatabaseConnection) {
    exec(
        db,
        r#"CREATE TABLE production_orders (
            id INTEGER PRIMARY KEY,
            order_no TEXT UNIQUE, sales_order_id INTEGER, product_id INTEGER,
            planned_quantity TEXT, actual_quantity TEXT,
            planned_start_date TEXT, planned_end_date TEXT,
            actual_start_date TEXT, actual_end_date TEXT,
            status TEXT, priority INTEGER, work_center_id INTEGER, remarks TEXT,
            color_no TEXT, dye_lot_no TEXT, batch_no TEXT,
            order_type TEXT, original_batch_id INTEGER, schedule_batch_key TEXT,
            created_by INTEGER, created_at TEXT, updated_at TEXT
        )"#,
    )
    .await;
    exec(
        db,
        r#"CREATE TABLE products (
            id INTEGER PRIMARY KEY, name TEXT, code TEXT, barcode TEXT,
            category_id INTEGER, specification TEXT, unit TEXT,
            standard_price TEXT, cost_price TEXT, description TEXT,
            status TEXT, is_deleted INTEGER, created_at TEXT, updated_at TEXT,
            product_type TEXT, fabric_composition TEXT, yarn_count TEXT,
            density TEXT, width TEXT, gram_weight TEXT, structure TEXT,
            finish TEXT, min_order_quantity TEXT, lead_time INTEGER,
            meters_per_piece TEXT, meters_per_roll TEXT,
            supplier_product_code TEXT, supplier_id INTEGER,
            is_batch_managed INTEGER, batch_level TEXT,
            execution_standard TEXT, factory_name TEXT, factory_address TEXT,
            product_grade TEXT
        )"#,
    )
    .await;
    exec(
        db,
        r#"CREATE TABLE boms (
            id INTEGER PRIMARY KEY, product_id INTEGER, version INTEGER,
            is_default INTEGER, status TEXT, remarks TEXT,
            created_by INTEGER, is_deleted INTEGER,
            created_at TEXT, updated_at TEXT
        )"#,
    )
    .await;
}

/// 种一条可用产品(create 服务链 validate_product_exists 要求真实行)
async fn seed_product(db: &sea_orm::DatabaseConnection) -> i32 {
    product::ActiveModel {
        name: Set("波次六产品".to_string()),
        code: Set(format!(
            "PRD-W6-{}",
            Utc::now().timestamp_nanos_opt().unwrap()
        )),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子产品写入应成功")
    .id
}

fn build_app(db: sea_orm::DatabaseConnection) -> Router {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    Router::new()
        .route(
            "/production-orders/orders",
            post(production_order_handler::create_production_order),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100, Some("all")), inject_auth))
}

async fn seeded_app() -> (Router, sea_orm::DatabaseConnection, i32) {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    create_tables(&db).await;
    let product_id = seed_product(&db).await;
    let read_db = db.clone();
    (build_app(db), read_db, product_id)
}

// =========================================================
// 2) sqlite 端到端:伪造单号不可达 INSERT、缺键 400
// =========================================================

/// 携带伪造 order_no 的创建请求:DTO 无该字段(serde 残差吸收,请求体不报错),
/// 服务链走到服务端取号为止——sqlite 方言不支持 pg_advisory_xact_lock,取号显式
/// 失败并返回 BUSINESS_ERROR;关键断言是 production_orders **零行**:
/// 修复前该请求会以 "FORGED-9999" 直接落库(伪造单号成功)。
#[tokio::test]
async fn forged_order_no_never_reaches_insert_zero_rows_written() {
    let (app, db, product_id) = seeded_app().await;
    let (status, v) = call(
        &app,
        Method::POST,
        "/production-orders/orders",
        Some(json!({
            "order_no": "FORGED-9999",
            "product_id": product_id,
            "planned_quantity": "100.00",
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "取号失败须显式报错(不静默): {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR", "取号失败归业务族: {v}");
    assert_eq!(
        v["message"], "生产订单号生成失败，请稍后重试",
        "sqlite 不支持 PG 咨询锁 → 取号失败即为预期拒绝点;真实落库断言见活库用例"
    );

    let rows = production_order::Entity::find()
        .all(&db)
        .await
        .expect("回读不得失败");
    assert!(
        rows.is_empty(),
        "伪造单号请求被拒后 production_orders 必须零写入(修复前 FORGED-9999 已落库)"
    );
}

/// planned_quantity 为 NOT NULL 列(m0007:78):缺键在请求 DTO 反序列化层即拒绝,
/// 不进入服务链、不落库;修复前服务层 Option + unwrap_or_default 会把"未提供"
/// 静默塌成数量 0 的订单。
#[tokio::test]
async fn missing_planned_quantity_rejected_at_dto_layer() {
    let (app, db, product_id) = seeded_app().await;
    let (status, _v) = call(
        &app,
        Method::POST,
        "/production-orders/orders",
        Some(json!({ "product_id": product_id })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "NOT NULL 列缺键必须 400(required 语义),不得回落默认值"
    );
    let rows = production_order::Entity::find().all(&db).await.unwrap();
    assert!(rows.is_empty(), "缺键被拒后不得有任何写入");
}

/// DTO 形态的行为锁:order_no 在请求体中出现不报反序列化错(旧前端兼容),
/// 且结构体不存在该可写字段(若存在反编译形态变化会在此暴露)。
#[test]
fn payload_accepts_order_no_key_without_writable_field() {
    let payload: CreateProductionOrderPayload = serde_json::from_value(json!({
        "order_no": "FORGED-9999",
        "product_id": 1,
        "planned_quantity": "10.00",
    }))
    .expect("携带 order_no 的旧前端请求体必须仍可反序列化(残差吸收)");
    // 反序列化成功后仅能观察到声明字段——order_no 不在其中(编译期即无注入口)。
    assert_eq!(payload.product_id, 1);
    let missing_no: Result<CreateProductionOrderPayload, _> =
        serde_json::from_value(json!({ "order_no": "X", "product_id": 1 }));
    assert!(
        missing_no.is_err(),
        "planned_quantity(NOT NULL)缺键必须由 DTO required 语义显式反序列化失败"
    );
}

// =========================================================
// 3) 活库(#[ignore],TEST_DATABASE_URL→PG):全链路正向
// =========================================================

/// 活库用例缺 TEST_DATABASE_URL 时显式 panic(禁止条件跳过假绿)
fn live_pg_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .expect("本用例必须跑在已迁移 PostgreSQL(TEST_DATABASE_URL)上,由 ci-test-rust-ignored 执行;禁止 sqlite 回退假绿")
}

/// 活库端到端:伪造单号被服务端忽略,真实落库单号为 PO 序列;
/// status/priority 不进 INSERT 列、由 DB DEFAULT DRAFT/5 生效(默认值单一来源)。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL(create 取号走 pg_advisory_xact_lock,sqlite 方言不支持)"]
async fn live_create_ignores_forged_no_and_defaults_come_from_db() {
    let db = sea_orm::Database::connect(live_pg_url())
        .await
        .expect("活库连接失败");
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let p = product::ActiveModel {
        name: Set(format!("波次六活库产品{suffix}")),
        code: Set(format!("PRD-W6L-{suffix}")),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let forged = format!("FORGED-W6L-{suffix}");
    let app = build_app(db.clone());
    let (status, v) = call(
        &app,
        Method::POST,
        "/production-orders/orders",
        Some(json!({
            "order_no": forged,
            "product_id": p.id,
            "planned_quantity": "100.00",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "活库创建须 200: {v}");
    let no = v["data"]["order_no"].as_str().unwrap().to_string();
    assert_ne!(no, forged, "伪造单号绝不允许成为落库单号");
    assert!(no.starts_with("PO"), "单号一律服务端取 PO 前缀: {no}");
    assert_eq!(
        v["data"]["status"], "DRAFT",
        "status 默认值唯一来源 = DB DEFAULT 'DRAFT'(m0007:84)"
    );
    assert_eq!(
        v["data"]["priority"], 5,
        "priority 缺键时默认值唯一来源 = DB DEFAULT 5(m0007:85),不再是代码塌 0"
    );

    // 连建两单:服务端序列不重复(取号器锁),且 DB 内不存在任何伪造号
    let (status2, v2) = call(
        &app,
        Method::POST,
        "/production-orders/orders",
        Some(json!({ "product_id": p.id, "planned_quantity": "50.00" })),
    )
    .await;
    assert_eq!(status2, StatusCode::OK, "活库第二单创建须 200: {v2}");
    let no2 = v2["data"]["order_no"].as_str().unwrap();
    assert_ne!(no, no2, "服务端取号不得重号");

    let forged_count = production_order::Entity::find()
        .filter(production_order::Column::OrderNo.eq(&forged))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(forged_count, 0, "伪造单号不得存在于库中");
}

// =========================================================
// 4) 源码扫描防回潮锁(无 DB)
// =========================================================

/// 从源码截取一个顶层条目块:anchor 起,到首个 "\n}"。
/// 先剔除 `\r`:Windows 工作树 CRLF 会使跨行 contains 断言漏检(先例 wave2 测试)
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

/// handler:CreateProductionOrderPayload 不存在 pub order_no 字段;
/// create_production_order 必须残差识别 + warn(不静默),且不得把请求值传服务层
#[test]
fn source_scan_handler_dto_has_no_writable_order_no() {
    let src = include_str!("../src/handlers/production_order_handler.rs");
    let payload = extract_block(src, "pub struct CreateProductionOrderPayload");
    assert!(
        !payload.contains("pub order_no"),
        "创建 DTO 不得存在 order_no 可写字段,实际块:\n{payload}"
    );
    assert!(
        payload.contains("#[serde(flatten)]"),
        "旧前端携带的 order_no 须由 flatten 残差映射识别(否则 warn 不可达),实际块:\n{payload}"
    );
    let block = extract_block(src, "pub async fn create_production_order");
    assert!(
        block.contains("get(\"order_no\")"),
        "create handler 必须显式检查残差中的 order_no 并 warn,不得静默丢弃,实际块:\n{block}"
    );
    assert!(
        block.contains("tracing::warn!"),
        "识别到用户手输单号必须留 warn 日志,实际块:\n{block}"
    );
    assert!(
        !block.contains("order_no: payload.order_no") && !block.contains(".order_no"),
        "请求体单号绝不允许流入服务层请求构造,实际块:\n{block}"
    );
}

/// service DTO:CreateProductionOrderRequest 无 order_no 字段;
/// planned_quantity NOT NULL 列不得声明 Option
#[test]
fn source_scan_service_request_has_no_order_no_field() {
    let src = include_str!("../src/services/production_order_ops/types.rs");
    let block = extract_block(src, "pub struct CreateProductionOrderRequest");
    assert!(
        !block.contains("pub order_no"),
        "服务层创建请求类型上不得存在 order_no 注入口(编译期杜绝旁路写号),实际块:\n{block}"
    );
    assert!(
        block.contains("pub planned_quantity: Decimal"),
        "NOT NULL 列 planned_quantity(m0007:78)不得声明 Option,实际块:\n{block}"
    );
}

/// crud:create 必须一律服务端取号;resolve_order_no(接受用户号)不得回潮;
/// build_create_active_model 不得 unwrap_or_default 兜底、默认值走 DB DEFAULT
#[test]
fn source_scan_create_always_server_side_numbering_and_no_default_swallow() {
    let src = include_str!("../src/services/production_order_ops/crud.rs");
    assert!(
        !src.contains("resolve_order_no"),
        "resolve_order_no(用户单号查重后原样入库)已因单号禁手输移除,不得回潮"
    );
    assert!(
        !src.contains("req.order_no"),
        "任何调用点不得再读取请求中的 order_no(字段已不存在)"
    );
    let block = extract_block(src, "pub async fn create(");
    assert!(
        block.contains("self.generate_unique_order_no()"),
        "create 必须一律服务端取号,实际块:\n{block}"
    );
    let builder = extract_block(src, "fn build_create_active_model(");
    assert!(
        !builder.contains("unwrap_or_default"),
        "不得以 unwrap_or_default 兜底掩盖 NOT NULL 缺键,实际块:\n{builder}"
    );
    assert!(
        builder.contains("status: NotSet"),
        "status 默认值唯一来源 = DB DEFAULT 'DRAFT',代码侧不得硬编码第二处,实际块:\n{builder}"
    );
    assert!(
        builder.contains("priority: NotSet"),
        "priority 缺键时不进 INSERT 列由 DB DEFAULT 5 生效,不得塌 0,实际块:\n{builder}"
    );
}
