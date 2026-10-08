//! 功能：大货批色审批（`/erp/bulk-color-approvals` 族）与应收对账单（`/ar-reconciliations`
//! 族）行级归属门禁的契约锁，钉死"读侧与写侧都先过归属门"，防水平越权（IDOR）回潮。
//! 调用方：集成测试层（真已迁移 PostgreSQL，`TEST_DATABASE_URL`）。
//! 入参：经 `test_common::setup_test_db()` 连接、清空业务表后播种分属两个 self 范围
//! 用户的销售订单 / 客户 / 批色记录 / 对账单，再以不同 `AuthContext`（data_scope=self，
//! user_id=归属人 / 非归属人）走真 HTTP 装配。
//! 传给谁：批色 handler 的创建/列表/详情/发送客户/降级/报废/历史七端点，
//! 与对账单 handler 的创建/列表/详情三端点。
//! 存什么：写路径成功后目标行落库并状态如实流转；越权拒绝必须零落库、状态零漂移。
//! 存哪里：`bulk_color_approval` / `bulk_color_approval_history` / `ar_reconciliations`
//! 表（回读真库比对）。
//!
//! 判据（逐端点）：
//! - 本人 2xx 如实回显 / 他人 403+FORBIDDEN（绝不降级成空列表或看着正常的响应）；
//! - 写族回读断言目标行状态未动（越权不触达 service 落库点）；
//! - 创建批色记录钉"对他人销售订单建单被拒且不留行"（按父销售订单 sales_order_id 回读计数）；
//! - 创建对账单钉"created_by 如实落会话用户"（回读该列等于登录人 user_id）。
//!
//! 归属继承：批色记录无自身归属列，归属经父销售订单继承（sales_orders 的 created_by）；
//! 对账单无自身部门列，归属经父客户继承（customers 的 owner_id）。两族读写前都必须先取
//! 父资源过 `check_resource_owner`，列表在查询构造处按父 RLS 归属列下推 data_scope。
//!
//! 反空操作自证：修复前各 handler 把会话提取器写成 `_auth`（提取后丢弃），函数体内零归属
//! 校验；故"他人 ⇒ 403""写族零漂移""建单不留行""created_by 落库"各条在修复前会拿到 2xx
//! 或落库，断言必红。判红分叉即修复后新增的父资源归属校验调用——校验失败立即
//! `return Err(403)`，不触达后续 service 写事务/查询，故越权路径既不流转状态也不留行。
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
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{ar_reconciliation_handler, bulk_color_approval_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    ar_reconciliation, bulk_color_approval, customer, dye_batch, sales_order, user,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const OWNER_A: i32 = 9001;
const OWNER_B: i32 = 9002;
const DEPT_ID: i32 = 9500;

const CUS_OWN: i32 = 9101;
const CUS_CROSS: i32 = 9102;
const SO_OWN: i32 = 9201;
const SO_CROSS: i32 = 9202;
const DYE_ID: i32 = 9301;

// 批色记录：均按父销售订单继承归属，SO_OWN→A、SO_CROSS→B。
const BCA_READ: i64 = 9401; // pending，读/历史目标，父 SO_OWN
const BCA_SEND: i64 = 9402; // sampled，发送客户目标，父 SO_OWN
const BCA_APPR: i64 = 9403; // approved，降级目标，父 SO_OWN
const BCA_SCRAP: i64 = 9404; // sampled，报废目标，父 SO_OWN
const BCA_CROSS: i64 = 9405; // pending，跨主读/写目标，父 SO_CROSS

// 对账单：CUS_OWN→A、CUS_CROSS→B。
const AR_OWN: i32 = 9601;
const AR_CROSS: i32 = 9602;

fn make_self_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("scope_fixture_{user_id}"),
        role_id: Some(2),
        department_id: Some(DEPT_ID),
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
    let now = Utc::now();

    for uid in [OWNER_A, OWNER_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("scope_fixture_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(DEPT_ID)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (cid, owner) in [(CUS_OWN, OWNER_A), (CUS_CROSS, OWNER_B)] {
        customer::ActiveModel {
            id: Set(cid),
            customer_code: Set(format!("CUS-SCOPE-{cid}")),
            customer_name: Set(format!("归属门测试客户{cid}")),
            credit_limit: Set(Decimal::ZERO),
            payment_terms: Set(30),
            status: Set("active".to_string()),
            customer_type: Set("retail".to_string()),
            owner_id: Set(owner),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 父销售订单：归属列 created_by（RLS 表），SO_OWN→A、SO_CROSS→B。
    for (so_id, owner, cust) in [(SO_OWN, OWNER_A, CUS_OWN), (SO_CROSS, OWNER_B, CUS_CROSS)] {
        sales_order::ActiveModel {
            id: Set(so_id),
            order_no: Set(format!("SO-SCOPE-{so_id}")),
            customer_id: Set(cust),
            order_date: Set(now),
            required_date: Set(Some(now)),
            status: Set("approved".to_string()),
            subtotal: Set(Decimal::ONE),
            tax_amount: Set(Decimal::ZERO),
            discount_amount: Set(Decimal::ZERO),
            shipping_cost: Set(Decimal::ZERO),
            total_amount: Set(Decimal::ONE),
            paid_amount: Set(Decimal::ZERO),
            balance_amount: Set(Decimal::ONE),
            created_by: Set(Some(owner)),
            department_id: Set(Some(DEPT_ID)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    dye_batch::ActiveModel {
        id: Set(DYE_ID),
        batch_no: Set("DYE-SCOPE-0001".to_string()),
        color_code: Set("C001".to_string()),
        color_name: Set("归属门测试色".to_string()),
        dye_lot_no: Set("DL-SCOPE".to_string()),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 批色记录：父单归属决定可见性，customer_id 仅业务列不参与归属门。
    let rows = [
        (BCA_READ, SO_OWN, "pending"),
        (BCA_SEND, SO_OWN, "sampled"),
        (BCA_APPR, SO_OWN, "approved"),
        (BCA_SCRAP, SO_OWN, "sampled"),
        (BCA_CROSS, SO_CROSS, "pending"),
    ];
    for (id, so, status) in rows {
        bulk_color_approval::ActiveModel {
            id: Set(id),
            sales_order_id: Set(so),
            dye_batch_id: Set(DYE_ID),
            customer_id: Set(CUS_OWN as i64),
            production_order_id: Set(None),
            product_id: Set(None),
            color_no: Set(Some("C001".to_string())),
            dye_lot_no: Set(Some("DL-SCOPE".to_string())),
            batch_no: Set(Some("B-SCOPE".to_string())),
            sample_type: Set("cut_sample".to_string()),
            sample_piece_id: Set(None),
            sample_length_m: Set(None),
            approval_status: Set(status.to_string()),
            approver_id: Set(None),
            approval_date: Set(None),
            sent_to_customer_at: Set(None),
            customer_feedback: Set(None),
            delta_e_value: Set(None),
            reject_reason: Set(None),
            delivery_blocking: Set(true),
            attachment_url: Set(None),
            remark: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 对账单：父客户归属决定可见性。
    for (id, cid) in [(AR_OWN, CUS_OWN), (AR_CROSS, CUS_CROSS)] {
        ar_reconciliation::ActiveModel {
            id: Set(id),
            reconciliation_no: Set(format!("RC-SCOPE-{id}")),
            reconciliation_date: Set(now.date_naive()),
            period_start: Set(now.date_naive()),
            period_end: Set(now.date_naive()),
            customer_id: Set(cid),
            customer_name: Set(Some(format!("归属门测试客户{cid}"))),
            opening_balance: Set(Decimal::ONE),
            total_invoices: Set(Decimal::ONE),
            total_collections: Set(Decimal::ZERO),
            closing_balance: Set(Decimal::ONE),
            reconciliation_status: Set(Some("draft".to_string())),
            confirmed_by_customer: Set(None),
            dispute_reason: Set(None),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            created_by: Set(Some(OWNER_A)),
            created_at: Set(now),
            updated_at: Set(now),
            notes: Set(None),
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
            "/erp/bulk-color-approvals",
            get(bulk_color_approval_handler::list_bulk_color_approvals)
                .post(bulk_color_approval_handler::create_bulk_color_approval),
        )
        .route(
            "/erp/bulk-color-approvals/{id}",
            get(bulk_color_approval_handler::get_bulk_color_approval),
        )
        .route(
            "/erp/bulk-color-approvals/{id}/send-to-customer",
            post(bulk_color_approval_handler::send_to_customer),
        )
        .route(
            "/erp/bulk-color-approvals/{id}/downgrade",
            post(bulk_color_approval_handler::downgrade),
        )
        .route(
            "/erp/bulk-color-approvals/{id}/scrap",
            post(bulk_color_approval_handler::scrap),
        )
        .route(
            "/erp/bulk-color-approvals/{id}/history",
            get(bulk_color_approval_handler::list_history),
        )
        .route(
            "/ar-reconciliations",
            get(ar_reconciliation_handler::list_reconciliations)
                .post(ar_reconciliation_handler::create_reconciliation),
        )
        .route(
            "/ar-reconciliations/{id}",
            get(ar_reconciliation_handler::get_reconciliation),
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

// ============ 批色：创建（写门 + 不留行） ============

#[tokio::test]
async fn bca_create_own_sales_order_persists_and_echoes() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before = bulk_color_approval::Entity::find()
        .filter(bulk_color_approval::Column::SalesOrderId.eq(SO_OWN))
        .count(&*db)
        .await
        .unwrap();

    let (status, v) = call(
        &app,
        Method::POST,
        "/erp/bulk-color-approvals",
        Some(json!({
            "sales_order_id": SO_OWN,
            "dye_batch_id": DYE_ID,
            "customer_id": CUS_OWN,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人父销售订单建批色单应 2xx: {v}");
    assert_eq!(v["data"]["sales_order_id"].as_i64(), Some(SO_OWN as i64));

    let after = bulk_color_approval::Entity::find()
        .filter(bulk_color_approval::Column::SalesOrderId.eq(SO_OWN))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(after, before + 1, "本人建单应在本人父单下如实落一行");
}

#[tokio::test]
async fn bca_create_on_other_sales_order_is_forbidden_and_leaves_no_row() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_cross = bulk_color_approval::Entity::find()
        .filter(bulk_color_approval::Column::SalesOrderId.eq(SO_CROSS))
        .count(&*db)
        .await
        .unwrap();

    // A(self) 试图对他人(B)的销售订单 SO_CROSS 发起批色单：父单归属门早退 403，
    // 判红分叉即 handler 新增的 ensure_parent_sales_order_access 返回 Err 提前退出，
    // 不触达 service.create，SO_CROSS 下行数零增长。
    let (status, v) = call(
        &app,
        Method::POST,
        "/erp/bulk-color-approvals",
        Some(json!({
            "sales_order_id": SO_CROSS,
            "dye_batch_id": DYE_ID,
            "customer_id": CUS_OWN,
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "对他人销售订单建批色单必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");

    let after_cross = bulk_color_approval::Entity::find()
        .filter(bulk_color_approval::Column::SalesOrderId.eq(SO_CROSS))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(after_cross, before_cross, "越权建单严禁在他人父单下落行");
}

// ============ 批色：列表（查询构造处下推，非空非全量） ============

#[tokio::test]
async fn bca_list_self_scope_returns_only_own_rows() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        "/erp/bulk-color-approvals?page_size=100",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人列表应 2xx: {v}");

    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();
    // 本人（SO_OWN 下 4 行）如实可见，绝不因归属门下推被误清空。
    assert!(!items.is_empty(), "越权门严禁把本人可见行降级成空列表");
    for it in &items {
        assert_eq!(
            it["sales_order_id"].as_i64(),
            Some(SO_OWN as i64),
            "self 用户列表只应含本人父销售订单的批色行: {it}"
        );
    }
    // 他人父单 SO_CROSS 的行绝不出现在结果里。
    assert!(
        items.iter().all(|it| it["id"].as_i64() != Some(BCA_CROSS)),
        "他人批色行不得越权可见"
    );
}

#[tokio::test]
async fn bca_list_for_other_user_sees_only_their_own_rows() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_B)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        "/erp/bulk-color-approvals?page_size=100",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "B 本人列表应 2xx: {v}");
    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();
    // B 只应看到 SO_CROSS 下那一行 BCA_CROSS。
    assert_eq!(
        items.len(),
        1,
        "B 可见集应恰为他人父单下的一行（total 与可见集一致）: {items:?}"
    );
    assert_eq!(items[0]["id"].as_i64(), Some(BCA_CROSS));
    assert_eq!(
        v["data"]["total"].as_u64(),
        Some(1),
        "分页 total 必须等于可见集"
    );
}

// ============ 批色：详情（读单行 IDOR） ============

#[tokio::test]
async fn bca_get_own_returns_2xx_faithfully() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/erp/bulk-color-approvals/{BCA_READ}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人详情应 2xx: {v}");
    assert_eq!(v["data"]["id"].as_i64(), Some(BCA_READ));
    assert_eq!(v["data"]["approval_status"].as_str(), Some("pending"));
}

#[tokio::test]
async fn bca_get_cross_owner_is_forbidden() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_B)).await;
    // B 读 A 的批色行：父单归属门早退 403，判红分叉即 handler 详情新增的
    // ensure_parent_sales_order_access 校验失败即 return，绝不返回他人记录字段。
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/erp/bulk-color-approvals/{BCA_READ}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主详情必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
}

// ============ 批色：发送客户（写门 + 状态零漂移） ============

#[tokio::test]
async fn bca_send_to_customer_own_transitions() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_SEND}/send-to-customer"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人发送客户应 2xx: {v}");

    let row = bulk_color_approval::Entity::find_by_id(BCA_SEND)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.approval_status, "sent_to_customer",
        "本人发送客户应如实流转到 sent_to_customer"
    );
}

#[tokio::test]
async fn bca_send_to_customer_cross_owner_is_forbidden_and_status_unmoved() {
    let (app, db) = seeded_app(make_self_auth(OWNER_B)).await;
    // B 改 A 的 sampled 行：归属门早退先于 service 写事务，判红分叉即
    // ensure_parent_sales_order_access 返回 Err，approval_status 保持 sampled。
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_SEND}/send-to-customer"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主发送客户必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    let row = bulk_color_approval::Entity::find_by_id(BCA_SEND)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.approval_status, "sampled", "越权写严禁流转他人记录状态");
}

// ============ 批色：降级（写门 + 状态零漂移） ============

#[tokio::test]
async fn bca_downgrade_own_transitions() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_APPR}/downgrade"),
        Some(json!({ "reject_reason": "色差偏大降级" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人降级应 2xx: {v}");

    let row = bulk_color_approval::Entity::find_by_id(BCA_APPR)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.approval_status, "downgraded", "本人降级应如实流转");
}

#[tokio::test]
async fn bca_downgrade_cross_owner_is_forbidden_and_status_unmoved() {
    let (app, db) = seeded_app(make_self_auth(OWNER_B)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_APPR}/downgrade"),
        Some(json!({ "reject_reason": "越权尝试" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主降级必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    let row = bulk_color_approval::Entity::find_by_id(BCA_APPR)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.approval_status, "approved",
        "越权降级严禁改他人记录状态"
    );
}

// ============ 批色：报废（写门 + 状态零漂移） ============

#[tokio::test]
async fn bca_scrap_own_transitions() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_SCRAP}/scrap"),
        Some(json!({ "reject_reason": "报废" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人报废应 2xx: {v}");

    let row = bulk_color_approval::Entity::find_by_id(BCA_SCRAP)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.approval_status, "scrapped", "本人报废应如实流转");
}

#[tokio::test]
async fn bca_scrap_cross_owner_is_forbidden_and_status_unmoved() {
    let (app, db) = seeded_app(make_self_auth(OWNER_B)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_SCRAP}/scrap"),
        Some(json!({ "reject_reason": "越权尝试" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主报废必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    let row = bulk_color_approval::Entity::find_by_id(BCA_SCRAP)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.approval_status, "sampled", "越权报废严禁改他人记录状态");
}

// ============ 批色：状态变更历史（读门） ============

#[tokio::test]
async fn bca_history_own_returns_2xx() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/erp/bulk-color-approvals/{BCA_READ}/history"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人历史应 2xx: {v}");
    assert!(v["data"].is_array(), "历史出参应为数组");
}

#[tokio::test]
async fn bca_history_cross_owner_is_forbidden() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_B)).await;
    // B 读 A 的变更轨迹：归属门早退 403，绝不降级成"该单无历史"的 2xx 空数组（假绿）。
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/erp/bulk-color-approvals/{BCA_READ}/history"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主历史必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_ne!(
        status,
        StatusCode::OK,
        "越权读历史严禁伪装成正常 2xx 空列表"
    );
}

// ============ 对账单：创建（父客户归属门 + created_by 落会话） ============

#[tokio::test]
async fn ar_create_own_persists_created_by_actor() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        "/ar-reconciliations",
        Some(json!({
            "reconciliation_no": "RC-NEW-OWN",
            "customer_id": CUS_OWN,
            "period_start": "2026-01-01",
            "period_end": "2026-01-31",
            "opening_balance": "100.0",
            "total_invoices": "50.0",
            "total_collections": "20.0",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人父客户建对账单应 2xx: {v}");
    let new_id = v["data"]["id"].as_i64().expect("新建对账单应回自增主键");

    let row = ar_reconciliation::Entity::find_by_id(new_id as i32)
        .one(&*db)
        .await
        .unwrap()
        .expect("新建对账单必须落库");
    // 钉"created_by 如实落会话用户"：修复前因丢弃会话写死 NULL，此处必须等于登录人。
    assert_eq!(
        row.created_by,
        Some(OWNER_A),
        "created_by 必须如实等于会话用户，不得再留 NULL"
    );
}

#[tokio::test]
async fn ar_create_other_customer_is_forbidden_and_leaves_no_row() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before = ar_reconciliation::Entity::find()
        .filter(ar_reconciliation::Column::CustomerId.eq(CUS_CROSS))
        .count(&*db)
        .await
        .unwrap();

    // A(self) 试图对他人(B)的客户 CUS_CROSS 发起对账单：父客户归属门早退 403，
    // 判红分叉即 handler 新增的 ensure_reconciliation_customer_access 返回 Err，
    // 不触达 service.create，CUS_CROSS 下对账单行数零增长。
    let (status, v) = call(
        &app,
        Method::POST,
        "/ar-reconciliations",
        Some(json!({
            "reconciliation_no": "RC-NEW-CROSS",
            "customer_id": CUS_CROSS,
            "period_start": "2026-01-01",
            "period_end": "2026-01-31",
            "opening_balance": "100.0",
            "total_invoices": "50.0",
            "total_collections": "20.0",
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "对他人客户建对账单必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");

    let after = ar_reconciliation::Entity::find()
        .filter(ar_reconciliation::Column::CustomerId.eq(CUS_CROSS))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(after, before, "越权建单严禁在他人客户下落行");
}

// ============ 对账单：列表（查询构造处按父客户下推） ============

#[tokio::test]
async fn ar_list_self_scope_returns_only_own_customer_rows() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, "/ar-reconciliations?page_size=100", None).await;
    assert_eq!(status, StatusCode::OK, "本人对账单列表应 2xx: {v}");

    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();
    assert!(!items.is_empty(), "越权门严禁把本人可见行降级成空列表");
    for it in &items {
        assert_eq!(
            it["customer_id"].as_i64(),
            Some(CUS_OWN as i64),
            "self 用户列表只应含本人客户的对账单: {it}"
        );
    }
    assert!(
        items
            .iter()
            .all(|it| it["id"].as_i64() != Some(AR_CROSS as i64)),
        "他人客户的对账单不得越权可见"
    );
}

#[tokio::test]
async fn ar_list_for_other_user_sees_only_their_own_rows() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_B)).await;
    let (status, v) = call(&app, Method::GET, "/ar-reconciliations?page_size=100", None).await;
    assert_eq!(status, StatusCode::OK, "B 本人列表应 2xx: {v}");
    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        items.len(),
        1,
        "B 可见集应恰为本人客户下的一条对账单: {items:?}"
    );
    assert_eq!(items[0]["customer_id"].as_i64(), Some(CUS_CROSS as i64));
    assert_eq!(
        v["data"]["total"].as_u64(),
        Some(1),
        "分页 total 必须等于可见集"
    );
}

// ============ 对账单：详情（读单行 IDOR） ============

#[tokio::test]
async fn ar_get_own_returns_2xx_faithfully() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/ar-reconciliations/{AR_OWN}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人对账单详情应 2xx: {v}");
    assert_eq!(v["data"]["id"].as_i64(), Some(AR_OWN as i64));
    assert_eq!(v["data"]["customer_id"].as_i64(), Some(CUS_OWN as i64));
}

#[tokio::test]
async fn ar_get_cross_owner_is_forbidden() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_B)).await;
    // B 读 A 客户的对账单：父客户归属门早退 403，判红分叉即 handler 详情新增的
    // ensure_reconciliation_customer_access 校验失败即 return，绝不返回他人财务字段。
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/ar-reconciliations/{AR_OWN}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主对账单详情必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
}
