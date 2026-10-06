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
//! - 真库端到端(路线一,#4669 判责;表结构唯一来源 = `backend/migration`,本文件不自建
//!   同构 DDL):携带伪造单号的创建请求走完 DTO 反序列化 + 服务链后,在**服务端取号**环节
//!   显式失败,订单行零写入——证明"取号先于写入、客户号永不可达 INSERT"。
//!   该失败分支在真库上的前置由夹具自建:一枚把当日 PO 号段基数顶到 u64::MAX 且自身
//!   仍占用候选位的旁路病态号,命中 `utils/number_generator.rs:326-352` 的显式报错路径
//!   (源码注释自称"病态号段…最终走显式报错路径失败可见")。原 sqlite 通道是靠方言缺失
//!   函数(pg_advisory_xact_lock 在 sqlite 不存在)偶然失败,属被废弃的 sqlite 可验性假设。
//!   planned_quantity 缺键在 DTO 反序列化层即 400(required 语义)。
//! - `#[ignore]` 活库(`test_common::setup_test_db()`→PG,ci-test-rust-ignored 执行):
//!   伪造单号被忽略、响应单号为服务端 PO 序列、status/priority 由 DB 默认生效。
//! - 源码扫描防回潮锁:payload/service DTO 字段、create 时序、NotSet 默认值纪律。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::production_order_handler::{self, CreateProductionOrderPayload};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::trace_context::catch_panic_middleware;
use bingxi_backend::models::{product, production_order};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter,
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
// 1) 真库种子(products 列与 models::product + backend/migration 逐列对应;
//    production_orders 用于零写入回读)
// =========================================================

/// 种一条可用产品(create 服务链 validate_product_exists 要求真实行)
async fn seed_product(db: &DatabaseConnection) -> i32 {
    product::ActiveModel {
        name: Set("波次六产品".to_string()),
        code: Set(format!(
            "PRD-W6-{}",
            Utc::now().timestamp_nanos_opt().unwrap()
        )),
        unit: Set("米".to_string()),
        // 状态 token 与写入方词表同源(models::status::master_data::ACTIVE)
        status: Set(bingxi_backend::models::status::master_data::ACTIVE.to_string()),
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

/// 取号失败分支的真库前置:种一枚"当日 PO 号段基数顶到 u64::MAX 且候选位仍被占用"的
/// 旁路病态号,让 `DocumentNumberGenerator::allocate_no` 在 100 次探测后走**显式报错**
/// 路径(number_generator.rs:326-352),从而在生产方言上稳定复现
/// "取号失败→400 业务族→订单零写入"这一被锁的分支。
///
/// 为什么不用"号段内连排 100 个已占用号"造前置:`allocate_no` 的基数取
/// `max(后缀流水)+1`,候选位恒高于既有最大值,连排占用只会把基数一起推高,
/// 无法在真库上穷尽——这正是源码注释所说的"病态号段"形态。
///
/// 除 order_no(探针本体)外的列一律照写入方 `build_create_active_model` 的口径:
/// order_type='normal'(NOT NULL+CHECK 词表)、status/priority 不 Set(由 DB DEFAULT 生效)、
/// planned_quantity 给合法数量 1(该列 NOT NULL 且无 DB 默认值,不塞 0 假值)。
async fn seed_exhausted_number_segment(db: &DatabaseConnection, product_id: i32) -> String {
    let probe_no = format!("PO{}{}", Utc::now().format("%Y%m%d"), u64::MAX);
    production_order::ActiveModel {
        order_no: Set(probe_no.clone()),
        product_id: Set(product_id),
        planned_quantity: Set(Decimal::ONE),
        order_type: Set("normal".to_string()),
        created_by: Set(100),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("取号前置探针({probe_no})写入失败: {e}"));
    probe_no
}

fn build_app(db: DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/production-orders/orders",
            post(production_order_handler::create_production_order),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100, Some("all")), inject_auth))
}

async fn seeded_app() -> (Router, DatabaseConnection, i32) {
    let db = test_common::setup_test_db().await;
    let product_id = seed_product(&db).await;
    let read_db = db.clone();
    (build_app(db), read_db, product_id)
}

// =========================================================
// 2) 真库端到端:伪造单号不可达 INSERT、缺键 400
// =========================================================

/// 携带伪造 order_no 的创建请求:DTO 无该字段(serde 残差吸收,请求体不报错),
/// 服务链走到服务端取号为止——当日号段基数被前置探针顶到饱和、100 个候选位全部占用,
/// 取号显式失败并返回 BUSINESS_ERROR;关键断言是除该前置探针行外 production_orders
/// **零行**:修复前该请求会以 "FORGED-9999" 直接落库(伪造单号成功)。
#[tokio::test]
async fn forged_order_no_never_reaches_insert_zero_rows_written() {
    let (app, db, product_id) = seeded_app().await;
    let probe_no = seed_exhausted_number_segment(&db, product_id).await;
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
        "取号分支失败(号段候选位穷尽)即为预期拒绝点,真实落库断言见活库用例"
    );

    // 回读排除夹具自建的前置探针行本身:被锁语义是"这笔被拒的请求零写入"
    let rows = production_order::Entity::find()
        .filter(production_order::Column::OrderNo.ne(&probe_no))
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
    // 生产链路把 axum 提取器拒绝（原生 415/422 纯文本）收口成 400 + VALIDATION_ERROR 的
    // 地点是 `middleware/trace_context.rs:259` `normalize_extractor_rejection`，它挂在
    // `catch_panic_middleware` 的响应回廊里（trace_context.rs:339-342）。测试 Router 不挂
    // 这一层就会锁到与生产不同的形态（裸 422）——按仓内既有范式
    // （`contract_wave8_extractor_rejection_envelope_test.rs:114`）把该层真实挂上，
    // 走同源链路，断"状态码 + 机器码 + 零写入"三件，不放宽成"任意 4xx"。
    let app = app.layer(from_fn(catch_panic_middleware));
    let (status, v) = call(
        &app,
        Method::POST,
        "/production-orders/orders",
        Some(json!({ "product_id": product_id })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "NOT NULL 列缺键必须 400(required 语义),不得回落默认值，实得: {v}"
    );
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "字段级缺失属解码/校验族机器码，不得是 BUSINESS/DATABASE 族: {v}"
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
// 3) 活库(#[ignore],test_common::setup_test_db()→已迁移 PG):全链路正向
// =========================================================

/// 活库端到端:伪造单号被服务端忽略,真实落库单号为 PO 序列;
/// status/priority 不进 INSERT 列、由 DB DEFAULT DRAFT/5 生效(默认值单一来源)。
/// 缺 `TEST_DATABASE_URL` 或指向 sqlite 时由夹具直接 panic(禁止条件跳过假绿)。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL(create 取号走 pg_advisory_xact_lock + 唯一约束兜底)"]
async fn live_create_ignores_forged_no_and_defaults_come_from_db() {
    let db = test_common::setup_test_db().await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let p = product::ActiveModel {
        name: Set(format!("波次六活库产品{suffix}")),
        code: Set(format!("PRD-W6L-{suffix}")),
        unit: Set("米".to_string()),
        // 状态 token 与写入方词表同源(models::status::master_data::ACTIVE)
        status: Set(bingxi_backend::models::status::master_data::ACTIVE.to_string()),
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
