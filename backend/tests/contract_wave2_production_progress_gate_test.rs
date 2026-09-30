//! 生产订单进度上报状态门 + 审批归属预检契约锁(任务 #153 缺陷1/2)
//!
//! 锁定的 file:line 契约(修复后形态):
//! - `backend/src/handlers/production_order_handler.rs::update_production_progress`
//!   (find_by_id 读出实体后、回写前,以写入方状态机词表
//!   `models/status/production::PRODUCTION_IN_PROGRESS` 做状态门:仅 IN_PROGRESS 可上报;
//!   拒绝经 `AppError::business_displayable` 外显真实文案——`AppError::business` 出参会
//!   被脱敏为"业务处理失败",用户不可见)
//! - `backend/src/handlers/production_order_handler.rs::approve_production_order`
//!   (对齐同域 update/delete/submit_for_approval 既有范式:get_by_id(id,
//!   Some(&data_scope_ctx)) 归属预检,越权 403 由 data_scope::check_resource_owner 判定,
//!   不手写第二套 owner 比对规则)
//!
//! 修复前缺陷(三路调查核实):
//! 1. POST /{id}/progress find_by_id 后直接覆写 actual_quantity/remarks,无任何状态门、
//!    不参与状态机;"仅 IN_PROGRESS 能报进度"只存在于前端 ProductionTable.vue 按钮门控,
//!    任意状态(含 DRAFT/CANCELLED/COMPLETED)直连接口即可改产量,绕过审计与成本归集口径。
//! 2. POST /{id}/approve 是同域唯一没有 data_scope 归属预检的写端点,
//!    任意登录用户可对他人 PENDING_APPROVAL 订单执行审批。
//!
//! 覆盖策略(全部真实行为,无 mock):
//! - sqlite::memory: 自建 production_orders/products 两表 + 真实 handler 端到端
//!   (tower oneshot):非 IN_PROGRESS 七态逐个 400 拒绝且 DB 零改动、IN_PROGRESS 正向
//!   200 落库、progress 非 owner 403(状态门之前)、approve 非 owner 403(修复前该请求
//!   直落服务侧;预检发生在触库锁之前,sqlite 可跑)。
//!   注:approve owner 正向路径进服务侧 lock_exclusive,sqlite 方言不支持(先例:
//!   contract_wave1_data_scope_idor_test.rs),正向断言归入 #[ignore] 活库用例。
//! - 源码扫描防回潮锁:两个 handler 块的预检/状态门/外显映射锚点。

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
use bingxi_backend::handlers::production_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::production_order;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, EntityTrait, Statement,
};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

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

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
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
    (status, serde_json::from_slice(&bytes).unwrap())
}

// =========================================================
// 1) sqlite 自建表(与 models/production_order.rs::Model 逐列对应;
//    products 仅 LEFT JOIN 富化取 id/name 两列)
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
            id INTEGER PRIMARY KEY, name TEXT
        )"#,
    )
    .await;
}

/// 种一条 created_by=100 的生产订单,指定状态,返回落库 Model
async fn seed_order(db: &sea_orm::DatabaseConnection, status: &str, order_no: &str) -> production_order::Model {
    production_order::ActiveModel {
        order_no: Set(order_no.to_string()),
        product_id: Set(1),
        planned_quantity: Set(dec("100.00")),
        status: Set(status.to_string()),
        priority: Set(5),
        color_no: Set(String::new()),
        dye_lot_no: Set(String::new()),
        batch_no: Set(String::new()),
        order_type: Set("normal".to_string()),
        created_by: Set(100),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

/// progress + approve 两端点都挂上,便于同一 app 分别断言
fn build_app(db: sea_orm::DatabaseConnection, viewer: AuthContext) -> Router {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    Router::new()
        .route(
            "/production-orders/orders/{id}/progress",
            post(production_order_handler::update_production_progress),
        )
        .route(
            "/production-orders/orders/{id}/approve",
            post(production_order_handler::approve_production_order),
        )
        .with_state(state)
        .layer(from_fn_with_state(viewer, inject_auth))
}

/// 返回 (Router, 独立连接句柄, 订单 id):同一 sqlite 内存库经克隆连接共享
async fn seeded_app(status: &str, viewer: AuthContext) -> (Router, sea_orm::DatabaseConnection, i32) {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    create_tables(&db).await;
    let order = seed_order(&db, status, &format!("PO-WAVE2-{status}")).await;
    let read_db = db.clone();
    (build_app(db, viewer), read_db, order.id)
}

fn progress_body(qty: &str, remarks: Option<&str>) -> Value {
    json!({ "actual_quantity": qty, "remarks": remarks })
}

// =========================================================
// 2) 缺陷1:进度上报状态门(仅 IN_PROGRESS)
// =========================================================

/// 核心回归锁:非 IN_PROGRESS 七个可达状态(含客诉点名的 DRAFT/CANCELLED/COMPLETED)
/// 直连 POST /{id}/progress 必须被业务错误拒绝,出参 code=BUSINESS_ERROR 且 message
/// 外显真实拒绝文案(不得脱敏为"业务处理失败"),修复前任意状态可静默覆写产量
#[tokio::test]
async fn progress_rejected_for_all_non_in_progress_statuses_with_displayable_message() {
    for status in [
        "DRAFT",
        "SCHEDULED",
        "PENDING_APPROVAL",
        "APPROVED",
        "REJECTED",
        "COMPLETED",
        "CANCELLED",
    ] {
        let (app, db, id) = seeded_app(status, make_auth(100, Some("self"))).await;
        let (http_status, v) = call(
            &app,
            Method::POST,
            &format!("/production-orders/orders/{id}/progress"),
            Some(progress_body("999.50", Some("越界改量"))),
        )
        .await;
        assert_eq!(
            http_status,
            StatusCode::BAD_REQUEST,
            "{status} 状态上报进度必须被拒绝,实际体: {v}"
        );
        assert_eq!(v["code"], "BUSINESS_ERROR", "{status} 拒绝须走业务错误信封");
        assert_eq!(
            v["message"],
            format!("订单当前状态为 {status}，仅生产中（IN_PROGRESS）的订单可上报生产进度"),
            "business_displayable 出参必须外显真实拒绝文案,不得脱敏"
        );

        let row = production_order::Entity::find_by_id(id)
            .one(&db)
            .await
            .unwrap()
            .expect("被拒绝的写操作不得影响原记录存在性");
        assert!(
            row.actual_quantity.is_none(),
            "{status} 状态被拒绝后 actual_quantity 必须保持 NULL(修复前被覆写为 999.50)"
        );
        assert!(
            row.remarks.is_none(),
            "{status} 状态被拒绝后 remarks 必须保持 NULL"
        );
    }
}

/// 正向对照:IN_PROGRESS 上报成功 200 且真实落库(状态门不得误伤合法路径)
#[tokio::test]
async fn progress_allowed_for_in_progress_and_persists() {
    let (app, db, id) = seeded_app("IN_PROGRESS", make_auth(100, Some("self"))).await;
    let (http_status, v) = call(
        &app,
        Method::POST,
        &format!("/production-orders/orders/{id}/progress"),
        Some(progress_body("42.50", Some("第一批下机"))),
    )
    .await;
    assert_eq!(http_status, StatusCode::OK, "IN_PROGRESS 上报须 200,实际体: {v}");
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"]["actual_quantity"], "42.50");

    let row = production_order::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("IN_PROGRESS 上报应可回读");
    assert_eq!(row.actual_quantity, Some(dec("42.50")));
    assert_eq!(row.remarks.as_deref(), Some("第一批下机"));
    assert_eq!(
        row.status, "IN_PROGRESS",
        "上报进度不改状态(不参与状态机跃迁,仅做写入口门)"
    );
}

/// progress 归属预检防回潮锁(既有 V15 P0-S02 范式):非 owner self 范围 → 403,
/// 且拒绝发生在状态判定之前(PENDING_APPROVAL 单被他人访问是 403 而非 400)
#[tokio::test]
async fn progress_non_owner_403_before_status_gate() {
    let (app, _db, id) = seeded_app("PENDING_APPROVAL", make_auth(200, Some("self"))).await;
    let (http_status, v) = call(
        &app,
        Method::POST,
        &format!("/production-orders/orders/{id}/progress"),
        Some(progress_body("10.00", None)),
    )
    .await;
    assert_eq!(http_status, StatusCode::FORBIDDEN, "越权上报应 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
}

// =========================================================
// 3) 缺陷2:approve 归属预检(对齐同域既有 get_by_id(Some(&ctx)) 范式)
// =========================================================

/// 核心回归锁:非 owner(self 范围)审批他人 PENDING_APPROVAL 订单 → 403 FORBIDDEN。
/// 修复前 handler 无任何 data_scope 预检,请求直落 approve_order 服务
/// (仅 find_by_id+lock_exclusive+状态门,无归属校验)
#[tokio::test]
async fn approve_non_owner_403_and_status_unchanged() {
    let (app, db, id) = seeded_app("PENDING_APPROVAL", make_auth(200, Some("self"))).await;
    let (http_status, v) = call(
        &app,
        Method::POST,
        &format!("/production-orders/orders/{id}/approve"),
        Some(json!({ "approved": true, "opinion": "越权审批" })),
    )
    .await;
    assert_eq!(http_status, StatusCode::FORBIDDEN, "越权审批应 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    let row = production_order::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("被拒绝的审批不得影响订单存在性");
    assert_eq!(
        row.status, "PENDING_APPROVAL",
        "越权审批被拒后状态必须保持 PENDING_APPROVAL(修复前会被改写为 APPROVED)"
    );
}

/// all 范围用户不受归属预检拦截(对照组,防止把 data-scope 修成一律 403);
/// 服务侧 lock_exclusive 在 sqlite 报错/透传均可能 → 只锁"不是 403"
#[tokio::test]
async fn approve_all_scope_not_blocked_by_ownership_precheck() {
    let (app, _db, id) = seeded_app("PENDING_APPROVAL", make_auth(200, Some("all"))).await;
    let (http_status, v) = call(
        &app,
        Method::POST,
        &format!("/production-orders/orders/{id}/approve"),
        Some(json!({ "approved": true, "opinion": null })),
    )
    .await;
    assert_ne!(
        http_status,
        StatusCode::FORBIDDEN,
        "all 范围不应被归属层拦截(后续服务侧方言错误另说): {v}"
    );
}

/// 活库:approve owner 正向(PENDING_APPROVAL → APPROVED);sqlite lock_exclusive 不支持
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL(approve_order 服务侧 lock_exclusive)"]
async fn live_approve_owner_happy_path() {
    // 接库后补:owner POST approve {approved:true} → 2xx 且回读状态 APPROVED;
    // 与上方 sqlite 的 403 负向成对,保持 ignore 直至接库
    // (先例:contract_wave1_data_scope_idor_test.rs D 节活库矩阵)。
}

// =========================================================
// 4) 源码扫描防回潮锁(无 DB)
// =========================================================

/// 从源码截取一个顶层函数块:anchor 起,到首个 "\n}"。
/// 先剔除 `\r`:Windows 工作树 CRLF 会使跨行 contains 断言漏检(先例 wave2 测试)
fn extract_block(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src.find(anchor).unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n}")
        .unwrap_or_else(|| panic!("块结束定位失败: {anchor}"));
    src[i..i + j + 2].to_string()
}

/// update_production_progress:状态门必须以 models/status 词表常量比较(不引入第二套
/// 手写字符串)、拒绝必须 business_displayable(不得回潮脱敏 business)、归属预检不得移除
#[test]
fn source_scan_progress_status_gate() {
    let src = include_str!("../src/handlers/production_order_handler.rs");
    let block = extract_block(src, "pub async fn update_production_progress");
    assert!(
        block.contains("service.get_by_id(id, Some(&data_scope_ctx))"),
        "progress 归属预检(get_by_id + data_scope_ctx)不得回潮删除,实际块:\n{block}"
    );
    assert!(
        block.contains(
            "model.status != crate::models::status::production::PRODUCTION_IN_PROGRESS"
        ),
        "状态门必须以词表常量 PRODUCTION_IN_PROGRESS 比较,不得手写第二套字符串,实际块:\n{block}"
    );
    assert!(
        block.contains("AppError::business_displayable("),
        "状态门拒绝必须外显(business 出参会被脱敏为『业务处理失败』),实际块:\n{block}"
    );
    assert!(
        !block.contains("AppError::business("),
        "禁止回潮脱敏 AppError::business,实际块:\n{block}"
    );
}

/// approve_production_order:必须有 get_by_id(Some(&data_scope_ctx)) 归属预检
/// (同域 update/delete/submit_for_approval 既有范式),禁止手写 owner 比对第二套规则
#[test]
fn source_scan_approve_ownership_precheck() {
    let src = include_str!("../src/handlers/production_order_handler.rs");
    let block = extract_block(src, "pub async fn approve_production_order");
    assert!(
        block.contains("let data_scope_ctx = auth.to_data_scope_context();"),
        "approve 必须提取 data_scope 上下文,实际块:\n{block}"
    );
    assert!(
        block.contains("service.get_by_id(id, Some(&data_scope_ctx))"),
        "approve 归属预检必须复用 get_by_id(Some(&ctx)) 范式,禁止手写 created_by 比对,实际块:\n{block}"
    );
    assert!(
        !block.contains("check_resource_owner"),
        "handler 层不得绕过 get_by_id 直接手拼归属判定,实际块:\n{block}"
    );
}
