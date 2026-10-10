//! 定制订单流程写族（取消 / 推进 / 工艺节点改与推进 / 节点日志 / 质量异常处置）
//! 六个残余站点的行级归属门契约锁，钉死"写侧先过（父）订单归属门 + 子资源实际归属==路径父 id"，
//! 防水平越权（IDOR）回潮。与 `contract_wave12_custom_order_owner_scope_test.rs`（定制订单主表
//! 读写族）互补：本文件只覆盖工艺节点与质量异常这些带子资源 id 的写端点，不改既有文件。
//!
//! 调用方：`cargo test` 集成层（真已迁移 PostgreSQL，TEST_DATABASE_URL）。
//! 入参：经 `test_common::setup_test_db()` 连接、清空业务表后，按固定 id 播种两条定制订单
//! （分属两个 self 范围用户 OWNER_A / OWNER_B）、各挂一条工艺节点与质量异常；再以不同
//! `AuthContext`（data_scope=self）走真 HTTP 装配。
//!
//! 判据（逐站点两条）：
//! - 本人 ⇒ 2xx 且行为如实（回读主表状态/节点 operator_id/节点日志计数/异常状态确实按请求变更）；
//! - 他人 ⇒ 403 + FORBIDDEN，写族回读断言零写入 / 状态零漂移（绝不降级成看着正常的空/200）。
//! 子资源三站另加「配对 oid/nid 绕过归属」⇒ 404（节点实际归属 != 路径父 id）与父不存在 ⇒ 404。
//!
//! 反空操作自证：修复前 6 handler 丢弃/不判归属——
//! - `cancel/advance` 仅取 `auth.user_id` 传 service，从不判订单归属；
//! - `update/advance_process_node`、`add_node_log` 忽略路径父 id（`_oid`），service 只按 nid 取行
//!   （`add_log` 甚至不查节点直接落日志）；
//! - `resolve_quality_issue` 直取异常行 id 上抛 service，不溯父单归属。
//! 故"他人应 403 / 配对应 404"各条在修复前会拿到 2xx，断言必红；判红分叉即各 handler 新增的
//! `check_resource_owner` 归属早退点与子资源 `node.custom_order_id != oid` 早退点。
//!
//! 夹具防法：`setup_test_db()` 自带 TRUNCATE 台账双向守卫，本文件不调 Migrator。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{delete, post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::custom_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    custom_order, customer, process_log, process_node, product, quality_issue, user,
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
const CO_OWN: i64 = 9301; // 归 OWNER_A，draft
const CO_CROSS: i64 = 9302; // 归 OWNER_B，draft
const CO_NOT_EXIST: i64 = 9999;
const NODE_OWN: i64 = 9501; // 属 CO_OWN
const NODE_CROSS: i64 = 9502; // 属 CO_CROSS
const QI_OWN: i64 = 9601; // 属 CO_OWN，open
const QI_CROSS: i64 = 9602; // 属 CO_CROSS，open
const QI_NOT_EXIST: i64 = 9699;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("copg_owner_{user_id}"),
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
            username: Set(format!("copg_owner_{uid}")),
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
        customer_code: Set("CUS-COPG-TEST".to_string()),
        customer_name: Set("定制订单流程门测试客户".to_string()),
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
        code: Set("PROD-COPG-TEST".to_string()),
        name: Set("定制订单流程门测试产品".to_string()),
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
        (CO_OWN, OWNER_A as i64, "COPG-TEST-OWN"),
        (CO_CROSS, OWNER_B as i64, "COPG-TEST-CROSS"),
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

    for (nid, co_id, node_name) in [
        (NODE_OWN, CO_OWN, "own-node"),
        (NODE_CROSS, CO_CROSS, "cross-node"),
    ] {
        process_node::ActiveModel {
            id: Set(nid),
            custom_order_id: Set(co_id),
            node_type: Set("dyeing".to_string()),
            node_name: Set(node_name.to_string()),
            sequence: Set(2),
            status: Set("pending".to_string()),
            planned_start_date: Set(None),
            planned_end_date: Set(None),
            actual_start_date: Set(None),
            actual_end_date: Set(None),
            operator_id: Set(None),
            notes: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (qi_id, co_id, desc) in [
        (QI_OWN, CO_OWN, "own 质量异常"),
        (QI_CROSS, CO_CROSS, "cross 质量异常"),
    ] {
        quality_issue::ActiveModel {
            id: Set(qi_id),
            custom_order_id: Set(co_id),
            process_node_id: Set(None),
            issue_type: Set("color_diff".to_string()),
            severity: Set("high".to_string()),
            description: Set(desc.to_string()),
            discovered_at: Set(Utc::now()),
            resolved_at: Set(None),
            resolution: Set(None),
            status: Set("open".to_string()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            root_cause_method: Set(None),
            root_cause_detail: Set(None),
            permanent_action_owner: Set(None),
            permanent_action_due_date: Set(None),
            permanent_action_completed_at: Set(None),
        }
        .insert(db)
        .await
        .unwrap();
    }
}

fn build_app(auth: AuthContext, db: Arc<sea_orm::DatabaseConnection>) -> Router {
    let state = AppState {
        db,
        ..Default::default()
    };
    Router::new()
        .route(
            "/custom-orders/{id}",
            delete(custom_order_handler::cancel_custom_order),
        )
        .route(
            "/custom-orders/{id}/advance",
            post(custom_order_handler::advance_custom_order),
        )
        .route(
            "/custom-orders/{id}/nodes/{nid}",
            put(custom_order_handler::update_process_node),
        )
        .route(
            "/custom-orders/{id}/nodes/{nid}/advance",
            post(custom_order_handler::advance_process_node),
        )
        .route(
            "/custom-orders/{id}/nodes/{nid}/logs",
            post(custom_order_handler::add_node_log),
        )
        .route(
            "/custom-orders/issues/{id}/resolve",
            put(custom_order_handler::resolve_quality_issue),
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

// ---- 回读辅助（真库比对，写族零漂移判据依赖） ----

async fn order_status(db: &sea_orm::DatabaseConnection, co_id: i64) -> String {
    custom_order::Entity::find_by_id(co_id)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .status
}

async fn node_operator(db: &sea_orm::DatabaseConnection, nid: i64) -> Option<i32> {
    process_node::Entity::find_by_id(nid)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .operator_id
}

async fn node_status(db: &sea_orm::DatabaseConnection, nid: i64) -> String {
    process_node::Entity::find_by_id(nid)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .status
}

async fn log_count_for_node(db: &sea_orm::DatabaseConnection, nid: i64) -> i64 {
    process_log::Entity::find()
        .filter(process_log::Column::ProcessNodeId.eq(nid))
        .all(db)
        .await
        .unwrap()
        .len() as i64
}

async fn issue_status(db: &sea_orm::DatabaseConnection, qi_id: i64) -> String {
    quality_issue::Entity::find_by_id(qi_id)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .status
}

// =============================================================
// 端点：DELETE /custom-orders/{id} - cancel_custom_order（写，先于 service 状态门）
// =============================================================
#[tokio::test]
async fn p01_cancel_own_is_2xx_and_status_drifts_to_cancelled() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/custom-orders/{CO_OWN}"),
        Some(json!({"reason": "客户取消"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 取消应 2xx: {v}");
    assert_eq!(
        order_status(&db, CO_OWN).await,
        "cancelled",
        "own 取消后状态应如实落 cancelled"
    );
}

#[tokio::test]
async fn p01_cancel_others_is_403_and_status_zero_drift() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/custom-orders/{CO_CROSS}"),
        Some(json!({"reason": "越权取消"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主取消必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        order_status(&db, CO_CROSS).await,
        "draft",
        "越权取消被拒后他人单状态零漂移"
    );
}

#[tokio::test]
async fn p01_cancel_not_exist_is_404() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/custom-orders/{CO_NOT_EXIST}"),
        Some(json!({"reason": "不存在"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "不存在订单必须 404: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
}

// =============================================================
// 端点：POST /custom-orders/{id}/advance - advance_custom_order（写，先于 service 状态机门）
// =============================================================
#[tokio::test]
async fn p02_advance_own_is_2xx_and_status_drifts_to_lab_dip() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_OWN}/advance"),
        Some(json!({"notes": "推进打样"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 推进应 2xx: {v}");
    // draft 顺序推进目标为 lab_dip（无状态门），行为如实
    assert_eq!(
        order_status(&db, CO_OWN).await,
        "lab_dip",
        "own 推进后状态应如实落 lab_dip"
    );
}

#[tokio::test]
async fn p02_advance_others_is_403_and_status_zero_drift() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_CROSS}/advance"),
        Some(json!({"notes": "越权推进"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主推进必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        order_status(&db, CO_CROSS).await,
        "draft",
        "越权推进被拒后他人单状态零漂移"
    );
}

// =============================================================
// 端点：PUT /custom-orders/{id}/nodes/{nid} - update_process_node（写子资源）
// =============================================================
#[tokio::test]
async fn p03_update_node_own_is_2xx_and_operator_from_session() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    assert_eq!(
        node_operator(&db, NODE_OWN).await,
        None,
        "前提：种子节点无操作人"
    );
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/{CO_OWN}/nodes/{NODE_OWN}"),
        Some(json!({"notes": "own 更新"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 改节点应 2xx: {v}");
    assert_eq!(
        node_operator(&db, NODE_OWN).await,
        Some(OWNER_A),
        "own 改节点后 operator_id 应如实落会话身份"
    );
}

#[tokio::test]
async fn p03_update_node_others_is_403_and_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/{CO_CROSS}/nodes/{NODE_CROSS}"),
        Some(json!({"notes": "越权改节点"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主改节点必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        node_operator(&db, NODE_CROSS).await,
        None,
        "越权后他人节点 operator_id 零写入"
    );
}

#[tokio::test]
async fn p03_update_node_mismatched_pair_is_404() {
    // 路径父 id 是本人单 CO_OWN（过归属门），但 nid 实际属他人单 CO_CROSS：
    // 必须"节点实际归属==路径父 id"命中失败 ⇒ 404，堵配对绕过通道。
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/{CO_OWN}/nodes/{NODE_CROSS}"),
        Some(json!({"notes": "配对绕过"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "节点实际归属!=路径父 id 必须 404: {v}"
    );
    assert_eq!(v["code"], "NOT_FOUND");
    assert_eq!(
        node_operator(&db, NODE_CROSS).await,
        None,
        "配对绕过被拒后他人节点零写入"
    );
}

#[tokio::test]
async fn p03_update_node_parent_not_exist_is_404() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/{CO_NOT_EXIST}/nodes/{NODE_OWN}"),
        Some(json!({"notes": "父不存在"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "父订单不存在必须 404: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
}

// =============================================================
// 端点：POST /custom-orders/{id}/nodes/{nid}/advance - advance_process_node（写子资源 + 落日志）
// =============================================================
#[tokio::test]
async fn p04_advance_node_own_is_2xx_and_persists() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before_logs = log_count_for_node(&db, NODE_OWN).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_OWN}/nodes/{NODE_OWN}/advance"),
        Some(json!({"action": "start"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 推进节点应 2xx: {v}");
    assert_eq!(
        node_status(&db, NODE_OWN).await,
        "in_progress",
        "行为如实：start→in_progress"
    );
    assert_eq!(
        node_operator(&db, NODE_OWN).await,
        Some(OWNER_A),
        "own 推进节点 operator_id 落会话身份"
    );
    assert_eq!(
        log_count_for_node(&db, NODE_OWN).await,
        before_logs + 1,
        "own 推进节点应新增一条工艺日志"
    );
}

#[tokio::test]
async fn p04_advance_node_others_is_403_and_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before_logs = log_count_for_node(&db, NODE_CROSS).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_CROSS}/nodes/{NODE_CROSS}/advance"),
        Some(json!({"action": "start"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主推进节点必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        node_status(&db, NODE_CROSS).await,
        "pending",
        "越权后他人节点状态零漂移"
    );
    assert_eq!(
        log_count_for_node(&db, NODE_CROSS).await,
        before_logs,
        "越权后他人节点日志零新增"
    );
}

#[tokio::test]
async fn p04_advance_node_mismatched_pair_is_404() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_OWN}/nodes/{NODE_CROSS}/advance"),
        Some(json!({"action": "start"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "节点实际归属!=路径父 id 必须 404: {v}"
    );
    assert_eq!(v["code"], "NOT_FOUND");
    assert_eq!(
        node_status(&db, NODE_CROSS).await,
        "pending",
        "配对绕过被拒后零漂移"
    );
}

// =============================================================
// 端点：POST /custom-orders/{id}/nodes/{nid}/logs - add_node_log（写子资源日志）
// =============================================================
#[tokio::test]
async fn p05_add_log_own_is_2xx_and_persists() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = log_count_for_node(&db, NODE_OWN).await;
    assert_eq!(before, 0, "前提：own 节点无日志");
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_OWN}/nodes/{NODE_OWN}/logs"),
        Some(json!({"action": "note", "log_content": "own 记录"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 加日志应 2xx: {v}");
    assert_eq!(
        log_count_for_node(&db, NODE_OWN).await,
        before + 1,
        "own 写入日志数 +1"
    );
}

#[tokio::test]
async fn p05_add_log_others_is_403_and_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = log_count_for_node(&db, NODE_CROSS).await;
    assert_eq!(before, 0, "前提：cross 节点无日志");
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_CROSS}/nodes/{NODE_CROSS}/logs"),
        Some(json!({"action": "note", "log_content": "越权记录"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主加日志必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        log_count_for_node(&db, NODE_CROSS).await,
        before,
        "越权后他人节点日志零新增"
    );
}

#[tokio::test]
async fn p05_add_log_mismatched_pair_is_404() {
    // service.add_log 不查节点直接按 nid 落日志：handler 必须自证节点归属，
    // 否则本人单 id + 他人节点 nid 的配对可把日志塞进他人节点。
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = log_count_for_node(&db, NODE_CROSS).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/custom-orders/{CO_OWN}/nodes/{NODE_CROSS}/logs"),
        Some(json!({"action": "note", "log_content": "配对塞日志"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "节点实际归属!=路径父 id 必须 404: {v}"
    );
    assert_eq!(v["code"], "NOT_FOUND");
    assert_eq!(
        log_count_for_node(&db, NODE_CROSS).await,
        before,
        "配对绕过被拒后零日志写入"
    );
}

// =============================================================
// 端点：PUT /custom-orders/issues/{id}/resolve - resolve_quality_issue（写子资源，上溯父单归属）
// =============================================================
#[tokio::test]
async fn p06_resolve_own_is_2xx_and_status_drifts_to_resolved() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    assert_eq!(
        issue_status(&db, QI_OWN).await,
        "open",
        "前提：own 异常 open"
    );
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/issues/{QI_OWN}/resolve"),
        Some(json!({"resolution": "own 已整改"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 处置异常应 2xx: {v}");
    assert_eq!(
        issue_status(&db, QI_OWN).await,
        "resolved",
        "own 处置后状态如实 resolved"
    );
}

#[tokio::test]
async fn p06_resolve_others_is_403_and_status_zero_drift() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/issues/{QI_CROSS}/resolve"),
        Some(json!({"resolution": "越权处置"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主处置异常必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        issue_status(&db, QI_CROSS).await,
        "open",
        "越权后他人异常状态零漂移"
    );
}

#[tokio::test]
async fn p06_resolve_not_exist_is_404() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/custom-orders/issues/{QI_NOT_EXIST}/resolve"),
        Some(json!({"resolution": "不存在"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "异常不存在必须 404: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
}
