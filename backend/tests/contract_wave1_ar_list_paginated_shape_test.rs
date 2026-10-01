//! AR 应收单列表 PaginatedResponse 形状锁（本波契约：裸 Vec → 标准分页信封）
//!
//! 锁定的 file:line 契约：
//! - `backend/src/handlers/ar_invoice_handler.rs:61-88`（list_ar_invoices 返回
//!   `Json<ApiResponse<PaginatedResponse<ar_invoice::Model>>>`，page clamp(1,1000)、
//!   page_size clamp(1,100)，默认 page=1/page_size=20）
//! - `backend/src/utils/response.rs:37-43`（PaginatedResponse 字段恰为
//!   items/total/page/page_size）与 `response.rs:95-110`（success_paginated 组装）
//! - `backend/src/services/ar_invoice_service.rs:243-272`（total=count 真实总数、
//!   offset/limit 截断）
//!
//! 覆盖策略：
//! - 无需活库的纯形状断言（serde 层，锁 data 键集合恰为四键、防止回退裸 Vec / 加杂键）
//! - `sqlite::memory:` 自建 ar_invoices 表 + ActiveModel 播种后走真实 handler 的
//!   HTTP 形状/total/截断/clamp 断言（**不需要活 PostgreSQL**，走测试专用内存库；
//!   读路径 get_list 仅 SELECT，无 advisory lock，sqlite 方言足够保真）

use std::str::FromStr;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::ar_invoice_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::ar_invoice;
use bingxi_backend::utils::response::{ApiResponse, PaginatedResponse};
use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DbBackend, Statement,
};
use serde_json::Value;
use tower::ServiceExt;

/// 响应 JSON 键集合（排序后），用于"恰为"断言
fn sorted_keys(obj: &Value) -> Vec<String> {
    let mut ks: Vec<String> = obj
        .as_object()
        .unwrap_or_else(|| panic!("期望 JSON 对象，实际: {}", obj))
        .keys()
        .cloned()
        .collect();
    ks.sort();
    ks
}

/// 构造测试 AuthContext（与 test_permission_rbac / handlers_system_update_authz_test 同字段）
fn make_auth(user_id: i32, username: &str, data_scope: Option<&str>) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: data_scope.map(|s| s.to_string()),
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

/// 组装 GET /ar/invoices → 真实 handler（路径与 routes 挂载一致，前缀 /api/v1/erp 之后的部分）
fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/ar/invoices", get(ar_invoice_handler::list_ar_invoices))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn sqlite_db() -> DatabaseConnection {
    sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

/// 与 `models/ar_invoice.rs::Model` 列逐一对应的 sqlite 建表（全列，非空按 Option 与否标注；
/// Decimal/DateTime 以 TEXT 存储，由 SeaORM/sqlx 自身编码保证 round-trip）
async fn create_ar_invoices_table(db: &DatabaseConnection) {
    exec(
        db,
        r#"CREATE TABLE ar_invoices (
            id INTEGER PRIMARY KEY,
            invoice_no TEXT, invoice_date TEXT, due_date TEXT,
            customer_id INTEGER, customer_name TEXT, customer_code TEXT,
            source_type TEXT, source_module TEXT, source_bill_id INTEGER, source_bill_no TEXT,
            batch_no TEXT, color_no TEXT, dye_lot_no TEXT, sales_order_no TEXT,
            invoice_amount TEXT, received_amount TEXT, unpaid_amount TEXT, tax_amount TEXT,
            quantity_meters TEXT, quantity_kg TEXT, unit_price TEXT,
            status TEXT, approval_status TEXT,
            salesperson_id INTEGER, created_by INTEGER, reviewed_by INTEGER, reviewed_at TEXT,
            created_at TEXT, updated_at TEXT
        )"#,
    )
    .await;
}

/// 播种 25 行：customer_id 交替 1/2（13 行客户 1、12 行客户 2），invoice_date 逐日递增
async fn seed_25_invoices(db: &DatabaseConnection) {
    let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    for i in 0..25i64 {
        let cid = if i % 2 == 0 { 1 } else { 2 };
        ar_invoice::ActiveModel {
            invoice_no: Set(format!("AR-{i:04}")),
            invoice_date: Set(base + Duration::days(i)),
            due_date: Set(base + Duration::days(i + 30)),
            customer_id: Set(cid),
            invoice_amount: Set(Decimal::from_str("100.00").unwrap()),
            received_amount: Set(Decimal::from_str("0.00").unwrap()),
            unpaid_amount: Set(Decimal::from_str("100.00").unwrap()),
            status: Set("OPEN".to_string()),
            approval_status: Set("APPROVED".to_string()),
            created_by: Set(100),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
}

// =========================================================
// 纯 serde 形状断言（无需任何 DB）
// =========================================================

/// data 键恰为 {items,total,page,page_size}；顶层键恰为 {code,data}；值为 u64 精确回显。
/// 回归锁：本波前 handler 返回裸 Vec（data 为数组、无 total/page/page_size 键）。
#[test]
fn paginated_response_data_keys_exactly_four_and_values() {
    let resp: ApiResponse<PaginatedResponse<serde_json::Value>> =
        ApiResponse::success_paginated(vec![json_row(1), json_row(2)], 25, 2, 10);
    let v = serde_json::to_value(&resp).unwrap();

    assert_eq!(
        sorted_keys(&v),
        vec!["code", "data"],
        "顶层键恰为 code/data"
    );
    assert_eq!(v["code"], serde_json::json!(200));
    assert!(
        v["data"].is_object(),
        "data 必须是对象（旧裸 Vec 形状判红）"
    );
    assert_eq!(
        sorted_keys(&v["data"]),
        vec!["items", "page", "page_size", "total"],
        "分页 data 键集合必须恰好为 {{items,total,page,page_size}}，多一少一皆判红"
    );
    assert_eq!(v["data"]["total"], 25u64);
    assert_eq!(v["data"]["page"], 2u64);
    assert_eq!(v["data"]["page_size"], 10u64);
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 2);
}

fn json_row(id: u64) -> serde_json::Value {
    serde_json::json!({ "id": id })
}

/// PaginatedResponse::new 与 Default 的回显语义锁（page_size 默认 10 仅属 Default，
/// success_paginated 路径必须以入参回显，不得被默认值吞掉）
#[test]
fn paginated_new_and_default_field_values() {
    let p: PaginatedResponse<i32> = PaginatedResponse::new(vec![7, 8], 9, 3, 4);
    assert_eq!(p.items, vec![7, 8]);
    assert_eq!((p.total, p.page, p.page_size), (9, 3, 4));

    let d: PaginatedResponse<i32> = PaginatedResponse::default();
    assert_eq!((d.total, d.page, d.page_size), (0, 1, 10));
}

// =========================================================
// HTTP 层真实行为（sqlite::memory 自建表，无需活 PG）
// =========================================================

async fn seeded_app(auth: AuthContext) -> Router {
    let db = sqlite_db().await;
    create_ar_invoices_table(&db).await;
    seed_25_invoices(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    build_app(state, auth)
}

/// total=真实 count（25）、items 按 page_size 截断、page=2 偏移生效（invoice_date 倒序）
#[tokio::test]
async fn ar_invoices_list_shape_total_and_truncation() {
    let app = seeded_app(make_auth(100, "e2e_purchaser_100", Some("all"))).await;
    let (status, v) = get_json(&app, "/ar/invoices?page=2&page_size=10").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&v["data"]),
        vec!["items", "page", "page_size", "total"]
    );
    assert_eq!(
        v["data"]["total"], 25u64,
        "total 必须是全表真实 count，而非当页条数"
    );
    assert_eq!(v["data"]["page"], 2u64);
    assert_eq!(v["data"]["page_size"], 10u64);
    let items = v["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 10, "page_size=10 必须恰好截断到 10 条");

    // 全量再查第 3 页：尾窗只剩 5 条（25 = 10+10+5），锁 offset 计算
    let (_, v3) = get_json(&app, "/ar/invoices?page=3&page_size=10").await;
    assert_eq!(v3["data"]["items"].as_array().unwrap().len(), 5);
    assert_eq!(v3["data"]["total"], 25u64);

    // items 行即 ar_invoice::Model 序列化：抽一行验证键存在与值类型（snake_case，金额字符串）
    let first = &items[0];
    assert_eq!(
        first["invoice_no"].as_str().unwrap().chars().count(),
        7,
        "invoice_no 形如 AR-0001"
    );
    assert!(
        first["invoice_amount"].is_string(),
        "rust_decimal 默认序列化为字符串（前端 .toFixed 崩溃根因的契约面）"
    );
    assert_eq!(first["invoice_amount"], "100.00");
}

/// 缺省分页 = page 1 / page_size 20；越界参数被 clamp（防 DoS 契约），且回显的是 clamp 后值
#[tokio::test]
async fn ar_invoices_list_default_and_clamp_echo() {
    let app = seeded_app(make_auth(100, "e2e_purchaser_100", Some("all"))).await;

    let (_, v) = get_json(&app, "/ar/invoices").await;
    assert_eq!(
        (v["data"]["page"].as_u64(), v["data"]["page_size"].as_u64()),
        (Some(1), Some(20))
    );
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 20);
    assert_eq!(v["data"]["total"], 25u64);

    let (_, v) = get_json(&app, "/ar/invoices?page=0&page_size=500").await;
    assert_eq!(v["data"]["page"], 1u64, "page=0 应 clamp 到 1");
    assert_eq!(
        v["data"]["page_size"], 100u64,
        "page_size=500 应 clamp 到 100"
    );
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 25);

    let (_, v) = get_json(&app, "/ar/invoices?page=5000&page_size=10").await;
    assert_eq!(v["data"]["page"], 1000u64, "page=5000 应 clamp 到 1000");
    assert_eq!(
        v["data"]["items"].as_array().unwrap().len(),
        0,
        "远超总量的页必须空窗而非报错"
    );

    let (_, v) = get_json(&app, "/ar/invoices?page=1&page_size=0").await;
    assert_eq!(v["data"]["page_size"], 1u64, "page_size=0 应 clamp 到 1");
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 1);
}

/// customer_id 筛选与 total 一致性（total 是筛选后 count，非全表）
#[tokio::test]
async fn ar_invoices_list_total_respects_filter() {
    let app = seeded_app(make_auth(100, "e2e_purchaser_100", Some("all"))).await;
    let (_, v) = get_json(&app, "/ar/invoices?customer_id=1&page_size=100").await;
    assert_eq!(
        v["data"]["total"], 13u64,
        "25 行中客户 1 占 13 行，total 必须与筛选一致"
    );
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 13);
    for item in v["data"]["items"].as_array().unwrap() {
        assert_eq!(item["customer_id"], 1);
    }
}

/// 空表时形状不变：data 仍是四键对象（items 空、total 0），不得回退 null/裸数组
#[tokio::test]
async fn ar_invoices_list_empty_table_keeps_shape() {
    let db = sqlite_db().await;
    create_ar_invoices_table(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100, "e2e_purchaser_100", Some("all")));

    let (status, v) = get_json(&app, "/ar/invoices").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&v["data"]),
        vec!["items", "page", "page_size", "total"]
    );
    assert_eq!(v["data"]["total"], 0u64);
    assert!(v["data"]["items"].as_array().unwrap().is_empty());
}
