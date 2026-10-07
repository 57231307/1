//! 报价域（`/quotations` 的 list/get/terms/expiring/expired）行级归属
//! 门禁契约锁，钉死 data_scope=self 用户仅可见/可操作本人名下报价，
//! 防水平越权（IDOR）回潮。
//! 调用方：`cargo test` 集成层（真已迁移 PostgreSQL，TEST_DATABASE_URL）。
//! 入参：经 `test_common::setup_test_db()` 连接、清空业务表后，按固定 id 播种两条
//! 报价（分属两 owner），再以不同 `AuthContext`（data_scope=self，user_id=
//! 报价 owner / 非 owner）走真 HTTP 装配。
//! 传给谁：`handlers::quotation_handler` 的 list/get/terms/expiring/expired 端点。
//! 存什么：写路径（PUT terms）成功后 `sales_quotation_term` 行落库；
//! 越权拒绝必须零漂移。
//! 存哪里：`sales_quotation_term` 表（回读真库比对证明"拒绝即零漂移、放行即真写"）。
//!
//! 判据（逐站点两判据）：
//! - self 范围 owner 访问自己名下报价：2xx 且如实返回真实数据；
//! - self 范围非 owner 访问他人报价：403 + code=FORBIDDEN（不得降级成 2xx 空列表）；
//! - 列表族 self 范围角色：仅返回本人报价、total 与可见集一致（不返回全量）；
//! - 写族（PUT terms）越权拒绝后回读 `sales_quotation_term` 零漂移。
//!
//! 修复前判红分叉：
//! - list_quotations/get_quotation/get_quotation_terms/set_quotation_terms/
//!   list_expiring/list_expired 六个 handler 均把 `auth: AuthContext` 写成
//!   `_auth: AuthContext` 丢弃，service 调用处无 data_scope 过滤，故任何持
//!   权限键的 self 范围用户能看到/操作全量报价；测试中 self 用户访问他人
//!   报价在修复前拿 2xx（读）或直接落库成功（写），断言必红。
//! - 列表族 total 分叉：修复前 list 无 scope 过滤，self 用户传 page_size=999
//!   能拿到所有报价，total 远超可见集；修复后 `apply_data_scope` 下推过滤，
//!   total = 本人名下数（本测试 owner_a=2 条、owner_b=0 条可见）。
//!
//! 夹具说明：`setup_test_db()` 自带 TRUNCATE 台账双向守卫（见 services/test_common.rs
//! 清表前守卫 + 清表后 seaql_migrations 行数自检），本文件不调 Migrator、不依赖台账
//! 内容，故不受"TRUNCATE 抹 seaql_migrations 致真库断言空转"影响。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::quotation_handler::{
    get_quotation, get_quotation_terms, list_expired, list_expiring, list_quotations,
    set_quotation_terms,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{sales_quotation, sales_quotation_term};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// 固定播种 id：OWNER_A 名下两条报价可访问，OWNER_B 名下一条为越权目标。
const OWNER_A: i32 = 9001;
const OWNER_B: i32 = 9002;
const CUSTOMER_ID: i32 = 9100;
const QT_OWN: i64 = 9201;
const QT_OWN_APPROVED: i64 = 9202;
const QT_CROSS: i64 = 9203;

fn make_auth_self(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("qt_owner_{user_id}"),
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "响应非 JSON（{uri}）: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

fn make_quotation(
    id: i64,
    owner: i64,
    status: &str,
    valid_until: NaiveDate,
) -> sales_quotation::Model {
    sales_quotation::Model {
        id,
        quotation_no: format!("QT-{id}"),
        customer_id: CUSTOMER_ID,
        sales_user_id: owner,
        quotation_date: valid_until - chrono::Duration::days(30),
        valid_until,
        currency: "USD".to_string(),
        exchange_rate: Decimal::ONE,
        base_currency: "CNY".to_string(),
        price_terms: "FOB".to_string(),
        incoterms_version: None,
        incoterm_location: None,
        tax_inclusive: false,
        tax_rate: Decimal::ZERO,
        moq: None,
        lead_time_days: None,
        customer_level: None,
        subtotal: Decimal::ZERO,
        tax_amount: Decimal::ZERO,
        total_amount: Decimal::ZERO,
        freight_cost: None,
        insurance_cost: None,
        duty_cost: None,
        status: status.to_string(),
        approval_instance_id: None,
        approved_by: None,
        approved_at: None,
        approval_reason: None,
        rejection_reason: None,
        converted_sales_order_id: None,
        converted_at: None,
        notes: None,
        created_by: owner,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

async fn seed(db: &sea_orm::DatabaseConnection) {
    use sea_orm::ActiveModelTrait;

    // 播种三条报价：QT_OWN(self, owner_a)、QT_OWN_APPROVED(APPROVED, owner_a)、QT_CROSS(APPROVED, owner_b)
    let today = Utc::now().date_naive();
    let future = today + chrono::Duration::days(3);
    let past = today - chrono::Duration::days(10);

    let rows = [
        make_quotation(QT_OWN, OWNER_A as i64, "draft", future),
        make_quotation(QT_OWN_APPROVED, OWNER_A as i64, "approved", future),
        make_quotation(QT_CROSS, OWNER_B as i64, "approved", future),
    ];
    for m in rows {
        let active: sales_quotation::ActiveModel = m.into();
        active.insert(db).await.unwrap();
    }

    // 播种一条过期报价（owner_b，状态 approved，valid_until 已过）
    let expired = make_quotation(9204, OWNER_B as i64, "approved", past);
    let active: sales_quotation::ActiveModel = expired.into();
    active.insert(db).await.unwrap();

    // 播种条款：QT_OWN 下一条条款
    sales_quotation_term::ActiveModel {
        id: Default::default(),
        quotation_id: Set(QT_OWN),
        term_type: Set("payment".to_string()),
        term_key: Set("T/T".to_string()),
        term_value: Set("30% deposit".to_string()),
        sequence: Set(0),
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seeded_db_app(auth: AuthContext) -> (Router, Arc<sea_orm::DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = Router::new()
        .route("/quotations", get(list_quotations))
        .route("/quotations/{id}", get(get_quotation))
        .route(
            "/quotations/{id}/terms",
            get(get_quotation_terms).put(set_quotation_terms),
        )
        .route("/quotations/expiring", get(list_expiring))
        .route("/quotations/expired", get(list_expired))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth));
    (app, db)
}

async fn term_count_for_qt(db: &sea_orm::DatabaseConnection, qt_id: i64) -> i64 {
    sales_quotation_term::Entity::find()
        .filter(sales_quotation_term::Column::QuotationId.eq(qt_id))
        .all(db)
        .await
        .unwrap()
        .len() as i64
}

// =============================================================
// D15 列表：self 用户仅见自己名下报价，total 与可见集一致
// =============================================================
#[tokio::test]
async fn self_user_list_only_own_quotations_total_matches() {
    let (app, _db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, "/quotations?page=1&page_size=99", None).await;
    assert_eq!(status, StatusCode::OK, "list 应 2xx: {v}");
    let data = &v["data"];
    let items = data["list"].as_array().expect("list 应为数组");
    let total = data["total"].as_u64().expect("total 应为数字");
    // OWNER_A 名下有 QT_OWN 和 QT_OWN_APPROVED 两条（不含 QT_CROSS）
    assert_eq!(
        total, 2,
        "self 范围 total 应为可见集大小(2)，不得返回全量(4)：实得 total={total}"
    );
    assert_eq!(items.len(), 2, "items 数量应与 total 一致");
    // 每条的 sales_user_id 必须是 OWNER_A
    for item in items {
        assert_eq!(
            item["sales_user_id"].as_i64().unwrap() as i32,
            OWNER_A,
            "列表不得包含他人报价：{item}"
        );
    }
}

// =============================================================
// D15 列表判红：self 用户不传 sales_user_id 也绝不返回全量
// =============================================================
#[tokio::test]
async fn self_user_list_without_filter_never_returns_all() {
    let (app, _db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, "/quotations", None).await;
    assert_eq!(status, StatusCode::OK);
    let total = v["data"]["total"].as_u64().unwrap();
    // 全库共 4 条，self 用户可见 2 条——修复前为 4（无 scope 过滤），必红
    assert!(
        total < 4,
        "self 范围不可拿到全量(4)，判红锚点：total={total}"
    );
}

// =============================================================
// D16 详情：本人报价 2xx 且如实回显
// =============================================================
#[tokio::test]
async fn self_user_get_own_quotation_is_2xx_and_truthful() {
    let (app, _db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, &format!("/quotations/{QT_OWN}"), None).await;
    assert_eq!(status, StatusCode::OK, "own 报价详情应 2xx: {v}");
    assert_eq!(v["data"]["id"], QT_OWN);
    assert_eq!(v["data"]["sales_user_id"], OWNER_A as i64);
}

// =============================================================
// D16 详情：他人报价 403 + FORBIDDEN
// =============================================================
#[tokio::test]
async fn self_user_get_others_quotation_is_403_forbidden() {
    let (app, _db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, &format!("/quotations/{QT_CROSS}"), None).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主报价详情必须 403（修复前为 2xx 返回他人数据）: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    let msg = v["message"].as_str().unwrap_or_default();
    assert!(
        !msg.contains(&QT_CROSS.to_string()),
        "对外文案不得拼记录 ID: {v}"
    );
}

// =============================================================
// D17 terms 读：本人 2xx 且如实返回
// =============================================================
#[tokio::test]
async fn self_user_get_own_terms_is_2xx() {
    let (app, _db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/quotations/{QT_OWN}/terms"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own terms 应 2xx: {v}");
    let terms = v["data"].as_array().unwrap();
    assert!(!terms.is_empty(), "own terms 应如实返回预置条款");
    assert_eq!(terms[0]["term_key"], "T/T");
}

// =============================================================
// D17 terms 读：他人 403
// =============================================================
#[tokio::test]
async fn self_user_get_others_terms_is_403() {
    let (app, _db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/quotations/{QT_CROSS}/terms"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主 terms 读必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
}

// =============================================================
// D18 terms 写：本人 2xx 且落库（回读零漂移）
// =============================================================
#[tokio::test]
async fn self_user_set_own_terms_is_2xx_and_persists() {
    let (app, db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let before = term_count_for_qt(&db, QT_OWN).await;
    assert_eq!(before, 1, "前提：own 报价有 1 条条款");

    let body = json!([
        {"term_type": "shipping", "term_key": "CIF", "term_value": "USD 500", "sequence": 0}
    ]);
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/quotations/{QT_OWN}/terms"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own terms 写应 2xx: {v}");

    let after = term_count_for_qt(&db, QT_OWN).await;
    assert_eq!(after, 1, "写后条款数应替换为新的 1 条（全量替换语义）");
}

// =============================================================
// D18 terms 写：他人 403 + 零漂移
// =============================================================
#[tokio::test]
async fn self_user_set_others_terms_is_403_and_zero_write() {
    let (app, db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    // QT_CROSS 属于 OWNER_B，没有预置条款
    let before = term_count_for_qt(&db, QT_CROSS).await;
    assert_eq!(before, 0, "前提：跨主报价无条款");

    let body = json!([
        {"term_type": "payment", "term_key": "越权", "term_value": "hack", "sequence": 0}
    ]);
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/quotations/{QT_CROSS}/terms"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主 terms 写必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    let after = term_count_for_qt(&db, QT_CROSS).await;
    assert_eq!(after, 0, "越权写被拒后条款必须零新增（不得先写再拒）");
}

// =============================================================
// D19 expiring：self 用户仅见自己即将到期 APPROVED 报价
// =============================================================
#[tokio::test]
async fn self_user_expiring_only_own() {
    let (app, _db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, "/quotations/expiring?days=7", None).await;
    assert_eq!(status, StatusCode::OK, "expiring 应 2xx: {v}");
    let items = v["data"].as_array().unwrap();
    // OWNER_A 有 QT_OWN_APPROVED (APPROVED, 未来 3 天)
    // OWNER_B 有 QT_CROSS (APPROVED, 未来 3 天) 和过期报价
    // self scope 用户只能看到自己的 QT_OWN_APPROVED
    for item in items {
        assert_eq!(
            item["sales_user_id"].as_i64().unwrap() as i32,
            OWNER_A,
            "expiring 列表不得包含他人报价: {item}"
        );
    }
    // 修复前此断言必红（QT_CROSS 也会出现在列表中）
}

// =============================================================
// D20 expired：self 用户仅见自己已过期的报价
// =============================================================
#[tokio::test]
async fn self_user_expired_only_own() {
    let (app, _db) = seeded_db_app(make_auth_self(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, "/quotations/expired", None).await;
    assert_eq!(status, StatusCode::OK, "expired 应 2xx: {v}");
    let items = v["data"].as_array().unwrap();
    // 过期报价 id=9204 属 OWNER_B，self 用户 OWNER_A 应看不到
    for item in items {
        assert_eq!(
            item["sales_user_id"].as_i64().unwrap() as i32,
            OWNER_A,
            "expired 列表不得包含他人报价: {item}"
        );
    }
    assert!(
        items.is_empty(),
        "OWNER_A 无过期报价，self 用户 expired 应为空（修复前返回全量非空）"
    );
}
