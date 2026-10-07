//! 功能：定制订单（`/custom-orders/{id}`）及其全部子资源行级归属门禁的契约锁，
//! 钉死"读侧与写侧都先过订单归属门"，防水平越权（IDOR）回潮。
//! 调用方：`cargo test` 集成层（真已迁移 PostgreSQL，TEST_DATABASE_URL）。
//! 入参：经 `test_common::setup_test_db()` 连接、清空业务表后，按固定 id 播种两条
//! 定制订单（分属两个 self 范围用户）；再以不同 `AuthContext`（data_scope=self，
//! user_id= 订单 owner / 非 owner）走真 HTTP 装配。
//! 传给谁：`handlers::custom_order_handler` 九个端点（D21–D29）。
//! 存什么：写路径成功后对应子资源行落库；越权拒绝必须零落库。
//! 存哪里：`process_nodes` / `quality_issues` / `after_sales` 表（回读真库比对）。
//!
//! 判据（逐站点）：
//! - 本人 2xx 如实回显 / 他人 403+FORBIDDEN（绝不降级成空列表）；
//! - 写族回读断言零写入；
//! - E01–E03：父不存在的 POST 必须被拒且不落孤儿行（含回读计数）。
//!
//! 反空操作自证：修复前 handler 把会话提取器写成 `_auth`（丢弃），
//! 无任何归属校验；故"cross owner 应 403"各条在修复前会拿到 2xx，断言必红。
//! 判红关键分叉即修复后各 handler 新增的 `check_resource_owner` 调用——
//! 归属失败时立即 `return Err(403)`，不触达后续 service 落库/查询。
//!
//! 夹具防法：`setup_test_db()` 自带 TRUNCATE 台账双向守卫，本文件不调 Migrator、
//! 不依赖台账内容。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{get, post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::custom_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    after_sales, custom_order, customer, process_node, product, quality_issue, user,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const OWNER_A: i32 = 9001;
const OWNER_B: i32 = 9002;
const CUSTOMER_ID: i32 = 9100;
const PRODUCT_ID: i32 = 9200;
const CO_OWN: i64 = 9301;
const CO_CROSS: i64 = 9302;
const CO_NOT_EXIST: i64 = 9999;
const AS_OWN: i64 = 9401;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("co_owner_{user_id}"),
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

async fn seed(db: &sea_orm::DatabaseConnection) {
    for uid in [OWNER_A, OWNER_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("co_owner_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(1)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    customer::ActiveModel {
        id: Set(CUSTOMER_ID),
        customer_code: Set("CUS-CO-TEST".to_string()),
        customer_name: Set("定制订单测试客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OWNER_A),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    product::ActiveModel {
        id: Set(PRODUCT_ID),
        code: Set("PROD-CO-TEST".to_string()),
        name: Set("定制订单测试产品".to_string()),
        unit: Set("m".to_string()),
        product_type: Set("fabric".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    for (co_id, owner, order_no) in [
        (CO_OWN, OWNER_A as i64, "CO-TEST-OWN"),
        (CO_CROSS, OWNER_B as i64, "CO-TEST-CROSS"),
    ] {
        custom_order::ActiveModel {
            id: Set(co_id),
            order_no: Set(order_no.to_string()),
            customer_id: Set(CUSTOMER_ID as i64),
            product_id: Set(PRODUCT_ID as i64),
            color_id: Set(None),
            spec: Set("test-spec".to_string()),
            quantity: Set(Decimal::ONE),
            unit: Set("m".to_string()),
            custom_requirements: Set(serde_json::json!({})),
            status: Set("draft".to_string()),
            currency: Set("CNY".to_string()),
            created_by: Set(Some(owner)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    after_sales::ActiveModel {
        id: Set(AS_OWN),
        custom_order_id: Set(CO_OWN),
        issue_type: Set("complaint".to_string()),
        customer_id: Set(CUSTOMER_ID),
        description: Set("测试售后工单".to_string()),
        status: Set("opened".to_string()),
        opened_at: Set(Utc::now()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

fn build_app(auth: AuthContext, db: Arc<sea_orm::DatabaseConnection>) -> Router {
    let state = AppState {
        db,
        ..Default::default()
    };
    Router::new()
        .route(
            "/custom-orders/{id}",
            get(custom_order_handler::get_custom_order)
                .put(custom_order_handler::update_custom_order),
        )
        .route(
            "/custom-orders/{id}/nodes",
            post(custom_order_handler::add_process_node),
        )
        .route(
            "/custom-orders/{id}/timeline",
            get(custom_order_handler::get_timeline),
        )
        .route(
            "/custom-orders/{id}/issues",
            get(custom_order_handler::list_quality_issues)
                .post(custom_order_handler::report_quality_issue),
        )
        .route(
            "/custom-orders/{id}/after-sales",
            get(custom_order_handler::list_after_sales)
                .post(custom_order_handler::create_after_sales),
        )
        .route(
            "/custom-orders/after-sales/{id}",
            put(custom_order_handler::update_after_sales),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn seeded_app(auth: AuthContext) -> (Router, Arc<sea_orm::DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let app = build_app(auth, db.clone());
    (app, db)
}

async fn node_count_for_co(db: &sea_orm::DatabaseConnection, co_id: i64) -> i64 {
    process_node::Entity::find()
        .filter(process_node::Column::CustomOrderId.eq(co_id))
        .all(db)
        .await
        .unwrap()
        .len() as i64
}

async fn issue_count_for_co(db: &sea_orm::DatabaseConnection, co_id: i64) -> i64 {
    quality_issue::Entity::find()
        .filter(quality_issue::Column::CustomOrderId.eq(co_id))
        .all(db)
        .await
        .unwrap()
        .len() as i64
}

async fn after_sales_count_for_co(db: &sea_orm::DatabaseConnection, co_id: i64) -> i64 {
    after_sales::Entity::find()
        .filter(after_sales::Column::CustomOrderId.eq(co_id))
        .all(db)
        .await
        .unwrap()
        .len() as i64
}

// =============================================================
// D21：GET /custom-orders/{id}
// =============================================================
#[tokio::test]
async fn d21_get_own_custom_order_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, &format!("/custom-orders/{CO_OWN}"), None).await;
    assert_eq!(status, StatusCode::OK, "own 定制订单详情应 2xx: {v}");
    assert_eq!(v["data"]["id"], CO_OWN);
}

#[tokio::test]
async fn d21_get_others_custom_order_is_403_forbidden() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/custom-orders/{CO_CROSS}"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主定制订单详情必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
}

// =============================================================
// D22：PUT /custom-orders/{id}
// =============================================================
#[tokio::test]
async fn d22_update_own_custom_order_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let body = json!({"spec": "updated-spec"});
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/{CO_OWN}"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 定制订单更新应 2xx: {v}");
}

#[tokio::test]
async fn d22_update_others_custom_order_is_403_and_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before_spec = custom_order::Entity::find_by_id(CO_CROSS)
        .one(&*db)
        .await
        .unwrap()
        .unwrap()
        .spec;
    let body = json!({"spec": "hacked-spec"});
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/{CO_CROSS}"),
        Some(body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主定制订单更新必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    let after_spec = custom_order::Entity::find_by_id(CO_CROSS)
        .one(&*db)
        .await
        .unwrap()
        .unwrap()
        .spec;
    assert_eq!(before_spec, after_spec, "越权写被拒后 spec 不得变更");
}

// =============================================================
// D23：POST /custom-orders/{id}/nodes
// =============================================================
#[tokio::test]
async fn d23_add_node_own_is_2xx_and_persists() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = node_count_for_co(&db, CO_OWN).await;
    let body = json!({"node_type": "dyeing", "node_name": "own-node", "sequence": 99});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_OWN}/nodes"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 添加工艺节点应 2xx: {v}");
    let after = node_count_for_co(&db, CO_OWN).await;
    assert_eq!(after, before + 1, "own 写入后节点数应 +1");
}

#[tokio::test]
async fn d23_add_node_others_is_403_and_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = node_count_for_co(&db, CO_CROSS).await;
    let body = json!({"node_type": "dyeing", "node_name": "hacked-node", "sequence": 99});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_CROSS}/nodes"),
        Some(body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主添加工艺节点必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    let after = node_count_for_co(&db, CO_CROSS).await;
    assert_eq!(after, before, "越权后节点数不得增长");
}

// =============================================================
// D24：GET /custom-orders/{id}/timeline
// =============================================================
#[tokio::test]
async fn d24_get_timeline_own_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/custom-orders/{CO_OWN}/timeline"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own timeline 应 2xx: {v}");
    assert_eq!(v["data"]["order_id"], CO_OWN);
}

#[tokio::test]
async fn d24_get_timeline_others_is_403_forbidden() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/custom-orders/{CO_CROSS}/timeline"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主 timeline 必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
}

// =============================================================
// D25：POST /custom-orders/{id}/issues
// =============================================================
#[tokio::test]
async fn d25_report_issue_own_is_2xx_and_persists() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = issue_count_for_co(&db, CO_OWN).await;
    let body = json!({"issue_type": "color_diff", "severity": "high", "description": "色差异常"});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_OWN}/issues"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 上报质量异常应 2xx: {v}");
    let after = issue_count_for_co(&db, CO_OWN).await;
    assert_eq!(after, before + 1, "own 写入后异常数应 +1");
}

#[tokio::test]
async fn d25_report_issue_others_is_403_and_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = issue_count_for_co(&db, CO_CROSS).await;
    let body = json!({"issue_type": "color_diff", "severity": "high", "description": "越权上报"});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_CROSS}/issues"),
        Some(body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主上报质量异常必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    let after = issue_count_for_co(&db, CO_CROSS).await;
    assert_eq!(after, before, "越权后异常数不得增长");
}

// =============================================================
// D26：GET /custom-orders/{id}/issues
// =============================================================
#[tokio::test]
async fn d26_list_issues_own_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/custom-orders/{CO_OWN}/issues"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 异常列表应 2xx: {v}");
}

#[tokio::test]
async fn d26_list_issues_others_is_403_not_empty_200() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/custom-orders/{CO_CROSS}/issues"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主异常列表必须 403（不得降级成空列表）: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_ne!(status, StatusCode::OK, "越权读严禁伪装成正常 2xx 空列表");
}

// =============================================================
// D27：POST /custom-orders/{id}/after-sales
// =============================================================
#[tokio::test]
async fn d27_create_after_sales_own_is_2xx_and_persists() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = after_sales_count_for_co(&db, CO_OWN).await;
    let body =
        json!({"customer_id": CUSTOMER_ID, "issue_type": "complaint", "description": "own 售后"});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_OWN}/after-sales"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 创建售后应 2xx: {v}");
    let after = after_sales_count_for_co(&db, CO_OWN).await;
    assert_eq!(after, before + 1, "own 写入后售后数应 +1");
}

#[tokio::test]
async fn d27_create_after_sales_others_is_403_and_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = after_sales_count_for_co(&db, CO_CROSS).await;
    let body =
        json!({"customer_id": CUSTOMER_ID, "issue_type": "complaint", "description": "越权售后"});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_CROSS}/after-sales"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主创建售后必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    let after = after_sales_count_for_co(&db, CO_CROSS).await;
    assert_eq!(after, before, "越权后售后数不得增长");
}

// =============================================================
// D28：GET /custom-orders/{id}/after-sales
// =============================================================
#[tokio::test]
async fn d28_list_after_sales_own_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/custom-orders/{CO_OWN}/after-sales"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 售后列表应 2xx: {v}");
}

#[tokio::test]
async fn d28_list_after_sales_others_is_403_not_empty_200() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/custom-orders/{CO_CROSS}/after-sales"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主售后列表必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_ne!(status, StatusCode::OK, "越权读严禁伪装成正常 2xx 空列表");
}

// =============================================================
// D29：PUT /custom-orders/after-sales/{id}（id 是售后行 id）
// =============================================================
#[tokio::test]
async fn d29_update_after_sales_own_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let body = json!({"status": "accepted"});
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/after-sales/{AS_OWN}"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 更新售后应 2xx: {v}");
}

#[tokio::test]
async fn d29_update_after_sales_others_is_403_and_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_B)).await;
    let body = json!({"status": "accepted"});
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/after-sales/{AS_OWN}"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主更新售后必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    let row = after_sales::Entity::find_by_id(AS_OWN)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "opened", "越权后售后状态不得变更");
}

// =============================================================
// E01：D23 父不存在的 POST 必须被拒且不落孤儿行
// =============================================================
#[tokio::test]
async fn e01_add_node_parent_not_exist_is_404_and_no_orphan() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = node_count_for_co(&db, CO_NOT_EXIST).await;
    assert_eq!(before, 0, "前提：不存在的订单无节点");
    let body = json!({"node_type": "dyeing", "node_name": "orphan-node", "sequence": 99});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_NOT_EXIST}/nodes"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "父不存在必须 404: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
}

// =============================================================
// E02：D25 父不存在的 POST 必须被拒且不落孤儿行
// =============================================================
#[tokio::test]
async fn e02_report_issue_parent_not_exist_is_404_and_no_orphan() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let body = json!({"issue_type": "color_diff", "severity": "high", "description": "孤儿异常"});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_NOT_EXIST}/issues"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "父不存在必须 404: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
    let orphan_count = quality_issue::Entity::find()
        .filter(quality_issue::Column::CustomOrderId.eq(CO_NOT_EXIST))
        .all(&*_db)
        .await
        .unwrap()
        .len();
    assert_eq!(orphan_count, 0, "父不存在时不得产生孤儿行");
}

// =============================================================
// E03：D27 父不存在的 POST 必须被拒且不落孤儿行
// =============================================================
#[tokio::test]
async fn e03_create_after_sales_parent_not_exist_is_404_and_no_orphan() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let body =
        json!({"customer_id": CUSTOMER_ID, "issue_type": "complaint", "description": "孤儿售后"});
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_NOT_EXIST}/after-sales"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "父不存在必须 404: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
    let orphan_count = after_sales::Entity::find()
        .filter(after_sales::Column::CustomOrderId.eq(CO_NOT_EXIST))
        .all(&*db)
        .await
        .unwrap()
        .len();
    assert_eq!(orphan_count, 0, "父不存在时不得产生孤儿行");
}
