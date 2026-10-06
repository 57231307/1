//! AR 核销候选列表 customer_id 过滤真正生效契约锁（任务 #162-②）
//!
//! 锁定的根因（日志取证 + 契约断言两路一致）：
//! - `handlers/ar_verification_handler.rs` 的两个候选列表端点曾以
//!   `Query<serde_json::Value>` 原样透传，Axum urlencoded 反序列化下所有值恒为
//!   `Value::String` ⇒ `services/ar_ops/verification_ops/query.rs` 的
//!   `query.get("customer_id").and_then(|v| v.as_i64())` 恒 `None` ⇒ 客户过滤
//!   **完全失效且静默**（勾选客户仍返回全部单据，核销错单风险）。
//! - 修复形态（对齐 sales_order_handler::OrderStatisticsQuery / budget_management_handler::BudgetListQuery 先例）：
//!   handler 改 typed DTO `UnverifiedDocsQuery { customer_id: Option<i64> }`，serde 在反序列化
//!   边界完成字符串→整数（非法值 400，不 unwrap_or 静默回落），再按 service **既有契约键**
//!   `customer_id` 以 `Value::Number` 重建透传，service 签名不变。
//!
//! 覆盖策略（全部真实 handler + 真实 SQL 行为，无 mock）：
//! 1. invoices 端点：不带过滤 = 两客户全部未核销发票（非 CANCELLED 且 unpaid>0）；
//!    带 customer_id=1 → 结果集为不带过滤结果集的真子集且**逐行 customer_id 相符**；
//!    边界锁：unpaid=0 行与 CANCELLED 行两个口径都不出现。
//! 2. payments 端点：confirmed 收款 + 已完全核销行排除的既有口径不变，customer_id 过滤生效。
//! 3. 非法值 `customer_id=abc` → 400（typed DTO 反序列化拒绝，锁"不再静默吞掉"）。
//! 4. 防回潮源码扫描：handler 不再出现 `Query<serde_json::Value>`；service 契约键
//!    `customer_id` + `as_i64()` 形态未动（本次只修 handler 侧取数，不改 service 签名）。

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
use bingxi_backend::handlers::ar_verification_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::{ar::COLLECTION_CONFIRMED, common::STATUS_CANCELLED};
use bingxi_backend::models::{ar_collection, ar_invoice, ar_reconciliation_item};
use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DbBackend, Statement,
};
use serde_json::Value;
use tower::ServiceExt;

mod test_common;
use test_common::setup_test_db;

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

/// 挂载两个候选列表真实 handler（相对路径与 routes 挂载形态一致）
fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route(
            "/ar/verifications/unverified/invoices",
            get(ar_verification_handler::get_unverified_invoices),
        )
        .route(
            "/ar/verifications/unverified/payments",
            get(ar_verification_handler::get_unverified_payments),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn get_status_and_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let body = if status.is_success() {
        serde_json::from_slice(&bytes).unwrap()
    } else {
        Value::Null
    };
    (status, body)
}

async fn exec_pg(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await
        .unwrap_or_else(|e| panic!("真库种子执行失败: {e}\nSQL: {sql}"));
}

/// FK 前置种子（真库路线一，表由迁移产出、不自建 DDL）：
/// - customers 1/2：ar_invoices.customer_id 带 REFERENCES customers(id) FK（m0012）
/// - ar_reconciliations id=1：ar_reconciliation_items.reconciliation_id 带 FK（m0012）
async fn seed_fk_prerequisites(db: &DatabaseConnection) {
    exec_pg(
        db,
        // 波0 夹具补 NULL 地雷：customers.customer_type 模型非 Option 而 DDL 可空，
        // 省略该列的种下行按模型读出即 SeaORM 类型错；补真实白名单值（唯一词表模块）。
        &format!(
            r#"INSERT INTO customers (id, customer_code, customer_name, customer_type) VALUES
           (1, 'W4AR-C1', 'AR过滤锁客户一', '{0}'),
           (2, 'W4AR-C2', 'AR过滤锁客户二', '{0}')"#,
            bingxi_backend::constants::customer_type::RETAIL
        ),
    )
    .await;
    exec_pg(
        db,
        &format!(
            r#"INSERT INTO ar_reconciliations
               (id, reconciliation_no, reconciliation_date, period_start, period_end,
                customer_id, opening_balance, total_invoices, total_collections,
                closing_balance, reconciliation_status)
           VALUES
               (1, 'W4AR-RECON-1', '2026-01-01', '2026-01-01', '2026-01-31',
                1, 0, 0, 0, 0, '{}')"#,
            // 主单状态取核销状态词表写入值（models/status/finance.rs ar 模块）：
            // 可核销收款列表的已核销汇总只计 closed 核销单挂的账本明细
            bingxi_backend::models::status::ar::RECONCILIATION_CLOSED
        ),
    )
    .await;
}

async fn seed_invoice(
    db: &DatabaseConnection,
    no: &str,
    customer_id: i32,
    unpaid: &str,
    status: &str,
    day_offset: i64,
) {
    let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    ar_invoice::ActiveModel {
        invoice_no: Set(no.to_string()),
        invoice_date: Set(base + Duration::days(day_offset)),
        due_date: Set(base + Duration::days(day_offset + 30)),
        customer_id: Set(customer_id),
        invoice_amount: Set(Decimal::from_str("100.00").unwrap()),
        received_amount: Set(
            Decimal::from_str("100.00").unwrap() - Decimal::from_str(unpaid).unwrap()
        ),
        unpaid_amount: Set(Decimal::from_str(unpaid).unwrap()),
        status: Set(status.to_string()),
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

async fn seed_collection(
    db: &DatabaseConnection,
    no: &str,
    customer_id: i32,
    amount: &str,
    status: &str,
    day_offset: i64,
) -> i32 {
    let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let m = ar_collection::ActiveModel {
        collection_no: Set(no.to_string()),
        collection_date: Set(base + Duration::days(day_offset)),
        customer_id: Set(customer_id),
        collection_amount: Set(Decimal::from_str(amount).unwrap()),
        status: Set(status.to_string()),
        created_by: Set(100),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    m.id
}

/// 种 6 张发票：客户 1 三张未清、客户 2 两张未清、客户 1 一张已清零、客户 2 一张已作废未清。
async fn seed_invoices(db: &DatabaseConnection) {
    seed_invoice(db, "AR-0001", 1, "100.00", "OPEN", 0).await;
    seed_invoice(db, "AR-0002", 2, "100.00", "OPEN", 1).await;
    seed_invoice(db, "AR-0003", 1, "100.00", "OPEN", 2).await;
    seed_invoice(db, "AR-0004", 2, "100.00", "OPEN", 3).await;
    seed_invoice(db, "AR-0005", 1, "100.00", "OPEN", 4).await;
    seed_invoice(db, "AR-0006", 1, "0.00", "OPEN", 5).await; // 已清：unpaid>0 门排除
    seed_invoice(db, "AR-0007", 2, "100.00", STATUS_CANCELLED, 6).await; // 作废：状态门排除
}

/// 种收款：客户 1 两笔 confirmed（其一将整笔核销）、客户 2 一笔 confirmed、客户 1 一笔非 confirmed。
/// 返回 (客户1首笔id, 客户1被整笔核销的id, 客户2id)
async fn seed_collections(db: &DatabaseConnection) -> (i32, i32, i32) {
    let p1 = seed_collection(db, "COL-0001", 1, "100.00", COLLECTION_CONFIRMED, 0).await;
    let p2 = seed_collection(db, "COL-0002", 1, "200.00", COLLECTION_CONFIRMED, 1).await;
    let p3 = seed_collection(db, "COL-0003", 2, "300.00", COLLECTION_CONFIRMED, 2).await;
    seed_collection(db, "COL-0004", 1, "400.00", "pending", 3).await; // 状态门排除

    // p2 整笔核销（|amount| = 200 == collection_amount → verified < amount 不成立，应被剔除）。
    // 明细形态与核销账本真实写入口一致：document_type = AR_COLLECTION，
    // 主单 closed（见 seed_fk_prerequisites）——已核销汇总读数只计该口径。
    ar_reconciliation_item::ActiveModel {
        reconciliation_id: Set(1),
        item_type: Set("RECEIPT".to_string()),
        document_type: Set(Some("AR_COLLECTION".to_string())),
        document_id: Set(Some(p2)),
        amount: Set(Decimal::from_str("-200.00").unwrap()),
        match_status: Set("MATCHED".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    (p1, p2, p3)
}

async fn seeded_app() -> Router {
    // 路线一：真库 PostgreSQL（迁移产出真实表），不再自建 sqlite 同构 DDL
    let db = setup_test_db().await;
    seed_fk_prerequisites(&db).await;
    seed_invoices(&db).await;
    seed_collections(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    build_app(state, make_auth(100, "e2e_ar_verifier", Some("all")))
}

/// 从 data 数组提取 (id, customer_id) 集合，并断言逐行 customer_id 等于期望值（None=不校验）
fn collect_rows(data: &Value, expect_customer: Option<i64>) -> Vec<(i64, i64)> {
    let arr = data
        .as_array()
        .unwrap_or_else(|| panic!("候选列表 data 应为数组，实际 {data}"));
    let mut out = Vec::new();
    for row in arr {
        let id = row["id"].as_i64().expect("行应有数字 id");
        let cid = row["customer_id"].as_i64().expect("行应有数字 customer_id");
        if let Some(e) = expect_customer {
            assert_eq!(cid, e, "过滤结果中混入非目标客户行: {row}");
        }
        out.push((id, cid));
    }
    out
}

/// invoices：typed DTO 让 customer_id 过滤真正生效 —— 带过滤集是不带过滤集的真子集，
/// 且逐行 customer_id 相符；已清/作废行两个口径都不出现（service 既有门不变）。
#[tokio::test]
async fn unverified_invoices_customer_filter_actually_applies() {
    let app = seeded_app().await;

    let (status, all) = get_status_and_json(&app, "/ar/verifications/unverified/invoices").await;
    assert_eq!(status, StatusCode::OK);
    let all_rows = collect_rows(&all["data"], None);
    assert_eq!(
        all_rows.len(),
        5,
        "不带过滤应含客户1×3 + 客户2×2（unpaid=0 与 CANCELLED 被既有门排除），实际 {all_rows:?}"
    );
    assert!(
        all_rows.iter().any(|(_, c)| *c == 2),
        "全量集应包含客户 2 行（否则子集断言无意义）"
    );

    let (status, one) =
        get_status_and_json(&app, "/ar/verifications/unverified/invoices?customer_id=1").await;
    assert_eq!(status, StatusCode::OK);
    let one_rows = collect_rows(&one["data"], Some(1));
    assert_eq!(
        one_rows.len(),
        3,
        "customer_id=1 应只剩客户 1 的 3 张未清发票（修复前恒返回 5 张=过滤静默失效）"
    );

    let all_ids: Vec<i64> = all_rows.iter().map(|(id, _)| *id).collect();
    for (id, _) in &one_rows {
        assert!(
            all_ids.contains(id),
            "带过滤行必须是不带过滤行的子集，id {id} 不在 {all_ids:?}"
        );
    }
    assert!(
        one_rows.len() < all_rows.len(),
        "存在两个客户的数据时，带过滤结果必须严格小于全量"
    );
}

/// payments：customer_id 过滤生效，且「已完全核销剔除 / 非 confirmed 剔除」既有口径不动。
#[tokio::test]
async fn unverified_payments_customer_filter_actually_applies() {
    let app = seeded_app().await;

    let (status, all) = get_status_and_json(&app, "/ar/verifications/unverified/payments").await;
    assert_eq!(status, StatusCode::OK);
    let all_rows = collect_rows(&all["data"], None);
    assert_eq!(
        all_rows.len(),
        2,
        "全量应为 p1(客户1)+p3(客户2)；p2 整笔核销、p4 非 confirmed 被既有门排除，实际 {all_rows:?}"
    );

    let (status, one) =
        get_status_and_json(&app, "/ar/verifications/unverified/payments?customer_id=1").await;
    assert_eq!(status, StatusCode::OK);
    let one_rows = collect_rows(&one["data"], Some(1));
    assert_eq!(
        one_rows.len(),
        1,
        "customer_id=1 应只剩 p1（修复前恒返回全量=过滤静默失效）"
    );

    let all_ids: Vec<i64> = all_rows.iter().map(|(id, _)| *id).collect();
    for (id, _) in &one_rows {
        assert!(all_ids.contains(id), "子集断言：id {id} 不在 {all_ids:?}");
    }
}

/// 非法 customer_id（urlencoded 恒为字符串 "abc"，无法转 i64）必须在反序列化边界拒绝为 400，
/// 锁"不再静默当作未过滤"——修复前该请求会静默返回全量 200。
#[tokio::test]
async fn non_integer_customer_id_is_rejected_400() {
    let app = seeded_app().await;
    for uri in [
        "/ar/verifications/unverified/invoices?customer_id=abc",
        "/ar/verifications/unverified/payments?customer_id=abc",
    ] {
        let (status, _) = get_status_and_json(&app, uri).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{uri}: 非法 customer_id 必须 400，不得静默回落为全量列表"
        );
    }
}

/// 防回潮源码扫描：handler 改 typed DTO 且不残留 Value 透传；service 契约键保持 customer_id 不变。
#[test]
fn source_scan_handler_typed_dto_and_service_contract_unchanged() {
    let handler = include_str!("../src/handlers/ar_verification_handler.rs").replace('\r', "");
    let service =
        include_str!("../src/services/ar_ops/verification_ops/query.rs").replace('\r', "");

    assert!(
        handler.contains("Query<UnverifiedDocsQuery>"),
        "handler 必须使用 typed DTO 取数"
    );
    assert!(
        handler.contains("customer_id") && handler.contains("serde_json::Value::from(v)"),
        "handler 必须以既有契约键 customer_id 重建 Value::Number 透传"
    );
    assert!(
        !handler.contains("Query<serde_json::Value>"),
        "handler 不得再出现 Query<serde_json::Value> 原样透传（同族缺陷形态）"
    );
    // 注：不限扫 handler 全文的 unwrap_or —— list_verifications 的分页 unwrap_or+clamp
    // 是既有合法默认语义；本族修复的"不静默回落"由 typed DTO 400 行为锁
    // （non_integer_customer_id_is_rejected_400）保证。
    // service 契约：本次修复不改 service 签名与键名，防两边各改一半
    assert!(
        service.contains("query.get(\"customer_id\")")
            && service.contains("and_then(|v| v.as_i64())"),
        "service 应仍以 customer_id + as_i64 消费参数（契约未漂移）"
    );
}
