//! 销售/采购单据「子面」（变更历史、发货记录、明细列表、拒绝写口）行级归属门禁的契约锁，
//! 钉死"读侧与写侧都先过父单归属门"，防水平越权（IDOR）回潮。
//! 调用方：`cargo test` 集成层（真已迁移 PostgreSQL，TEST_DATABASE_URL）。
//! 入参：经 `test_common::setup_test_db()` 连接、清空业务表后，按固定 id 播种两条父单
//! （分属两个 self 范围用户 OWNER_A / OWNER_B）及其子面行；再以不同 `AuthContext`
//! （data_scope=self，user_id = 父单 owner / 非 owner）走真 HTTP 装配。
//! 传给谁：
//! - `handlers::sales_order_handler::get_order_history` / `reject_order` / `get_order_deliveries`
//! - `handlers::sales_return_handler::list_return_items`
//! - `handlers::purchase_order_handler::list_order_items`
//! - `handlers::purchase_return_handler::list_purchase_return_items`
//! 存什么 / 存哪里：拒绝写口（`/sales/orders/{id}/reject`）成功改 `sales_orders.status`；
//! 越权拒绝必须零漂移（回读真库比对 status），放行必须真写（status 翻转为 rejected）。
//!
//! 每站点两判据（+ 写站另钉状态零漂移）：
//! - 本人（OWNER_A 读/写自己名下父单的子面）：2xx 且数据如实回显真实子行（非空、非 mock）；
//! - 他人（OWNER_A 读/写 OWNER_B 名下父单的子面）：403 + 机器码 FORBIDDEN，绝不降级成
//!   2xx 空列表（那是前端"该单无变更/无发货/无明细"的假绿）；对写站，403 后回读真库确认
//!   status 零漂移（拒绝必须先于落库，不允许先写再拒）；
//! - 失败信封键齐全（code/message/trace_id/timestamp），message 为固定脱敏常量，不含单号/记录 ID。
//!
//! 判红根因（修复前为何必红）：六枚 handler 把会话提取器写成 `_auth`（丢弃），子面查询只按
//! 父 id 过滤、拒绝写口只按 status==pending 放行，均无归属校验；故"他人子面应 403"在修复前
//! 会拿到 2xx（读：直接返回他人子行；写：直接翻转他人订单状态），断言必红。判红的关键分叉
//! 即修复后各 handler 在子查询/落库点之前新增的取单门 —— 该 `?` 让归属门失败时立即向上传
//! `AppError::permission_denied`（403 + FORBIDDEN），不再触达其后的子面查询或状态翻转。
//!
//! 夹具防法：`setup_test_db()` 自带 TRUNCATE 台账双向守卫（清表前"待清集合 ∩ 台账集合必须为空"
//! 守卫 + 清表后 `seaql_migrations` 记账行数 > 0 反向自检，判据与排除集同源），本文件不调
//! Migrator、不依赖台账内容，故不受"TRUNCATE 抹 `seaql_migrations` 致真库断言空转"影响。
//!
//! 判据主体取 data_scope=self 的普通用户（同时是 RBAC 键的持证岗位），不用 all/dept ——
//! 用 all 会把行级归属拒绝冒充成"全可见放行"，用非持证角色会把 RBAC 拒绝冒充成行级归属拒绝。

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
use bingxi_backend::handlers::purchase_order_handler::list_order_items;
use bingxi_backend::handlers::purchase_return_handler::list_purchase_return_items;
use bingxi_backend::handlers::sales_order_handler::{
    get_order_deliveries, get_order_history, reject_order,
};
use bingxi_backend::handlers::sales_return_handler::list_return_items;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    customer, product, purchase_order, purchase_order_item, purchase_return, purchase_return_item,
    sales_delivery, sales_order, sales_order_change_history, sales_return, sales_return_item,
    supplier, user, warehouse,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryOrder};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// 固定播种 id：owner_a 名下父单及其子行可读/可写，owner_b 名下父单为越权目标。
const OWNER_A: i32 = 8001;
const OWNER_B: i32 = 8002;
const DEPT_ID: i32 = 1; // 迁移种子参照表 departments 的稳定主部门
const CUSTOMER_ID: i32 = 8100;
const PRODUCT_ID: i32 = 8120;
const WAREHOUSE_ID: i32 = 8130;

const SO_OWN: i32 = 8201; // sales_orders：owner_a 名下
const SO_CROSS: i32 = 8202; // sales_orders：owner_b 名下
const HISTORY_ID: i32 = 8211; // sales_order_change_history：SO_OWN 的变更行（显式固定 id）
const DELIVERY_ID: i32 = 8221; // sales_delivery：SO_OWN 的一条发货
const SR_OWN: i32 = 8301; // sales_return：owner_a 名下
const SR_CROSS: i32 = 8302; // sales_return：owner_b 名下
const SR_ITEM_ID: i32 = 8311; // sales_return_item：SR_OWN 的一条明细
const PO_OWN: i32 = 8401; // purchase_orders：owner_a 名下
const PO_CROSS: i32 = 8402; // purchase_orders：owner_b 名下
const PO_ITEM_ID: i32 = 8411; // purchase_order_item：PO_OWN 的一条明细
const PR_OWN: i32 = 8501; // purchase_return：owner_a 名下
const PR_CROSS: i32 = 8502; // purchase_return：owner_b 名下
const PR_ITEM_ID: i32 = 8511; // purchase_return_item：PR_OWN 的一条明细

/// 构造 self 范围会话（role_id=2 非 admin；owner 与跨主都靠行级归属判定，不靠 RBAC）。
fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("scope_owner_{user_id}"),
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

/// 取迁移种子的既有供应商主键（purchase_orders/purchase_return 的 supplier_id 外键父级）。
/// suppliers 属迁移种子参照表（不参与 setup_test_db 的 TRUNCATE），故按最小 id 稳定取一条。
async fn first_supplier_id(db: &sea_orm::DatabaseConnection) -> i32 {
    supplier::Entity::find()
        .order_by_asc(supplier::Column::Id)
        .one(db)
        .await
        .unwrap()
        .expect("迁移种子应至少有一条 suppliers 行（供应商目录种子）")
        .id
}

/// 装配仅含单个被测端点 + 指定会话的 Router，并返回真库句柄供回读落库结果。
/// 同一条连接：seed 用、handler 写用、回读用均指向同一已迁移 PostgreSQL，故回读可见 handler 写入。
async fn seeded_app(
    auth: AuthContext,
    route: impl Fn(Router<AppState>) -> Router<AppState>,
) -> (Router, Arc<sea_orm::DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = route(Router::new())
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth));
    (app, db)
}

/// 真库播种：两条父单分属两个 owner，各带子面行；跨主父单（*_CROSS）无子行预置。
async fn seed(db: &sea_orm::DatabaseConnection) {
    let supplier_id = first_supplier_id(db).await;

    for uid in [OWNER_A, OWNER_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("scope_owner_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(DEPT_ID)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    product::ActiveModel {
        id: Set(PRODUCT_ID),
        name: Set("归属锁测试面料".to_string()),
        code: Set("GATE-FAB-0001".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("成品布".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    warehouse::ActiveModel {
        id: Set(WAREHOUSE_ID),
        warehouse_code: Set("GATE-WH-0001".to_string()),
        name: Set("归属锁测试仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    customer::ActiveModel {
        id: Set(CUSTOMER_ID),
        customer_code: Set("GATE-CUS-0001".to_string()),
        customer_name: Set("归属锁测试客户".to_string()),
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

    // ---- sales_orders（D09/D10/D11）----
    for (oid, owner) in [(SO_OWN, OWNER_A), (SO_CROSS, OWNER_B)] {
        sales_order::ActiveModel {
            id: Set(oid),
            order_no: Set(format!("SO-GATE-{oid}")),
            customer_id: Set(CUSTOMER_ID),
            order_date: Set(Utc::now()),
            // D10 拒绝写口的状态门要求 pending（小写，词表见 models/status 销售订单节）
            status: Set("pending".to_string()),
            subtotal: Set(Decimal::ZERO),
            tax_amount: Set(Decimal::ZERO),
            discount_amount: Set(Decimal::ZERO),
            shipping_cost: Set(Decimal::ZERO),
            total_amount: Set(Decimal::ZERO),
            paid_amount: Set(Decimal::ZERO),
            balance_amount: Set(Decimal::ZERO),
            created_by: Set(Some(owner)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    sales_order_change_history::ActiveModel {
        id: Set(HISTORY_ID),
        order_id: Set(SO_OWN),
        change_type: Set("STATUS_CHANGE".to_string()),
        field_name: Set(Some("status".to_string())),
        old_value: Set(Some("draft".to_string())),
        new_value: Set(Some("pending".to_string())),
        changed_by: Set(OWNER_A),
        changed_at: Set(Utc::now()),
        change_reason: Set(Some("提交待审".to_string())),
        ip_address: Set(None),
        user_agent: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    }
    .insert(db)
    .await
    .unwrap();

    sales_delivery::ActiveModel {
        id: Set(DELIVERY_ID),
        delivery_no: Set("DN-GATE-0001".to_string()),
        order_id: Set(SO_OWN),
        customer_id: Set(CUSTOMER_ID),
        warehouse_id: Set(WAREHOUSE_ID),
        delivery_date: Set(Utc::now().date_naive()),
        status: Set("pending".to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        remarks: Set(Some("归属锁测试发货".to_string())),
        created_by: Set(OWNER_A),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    }
    .insert(db)
    .await
    .unwrap();

    // ---- sales_return（D12，created_by NOT NULL、无部门冗余列）----
    for (rid, owner) in [(SR_OWN, OWNER_A), (SR_CROSS, OWNER_B)] {
        sales_return::ActiveModel {
            id: Set(rid),
            return_no: Set(format!("SR-GATE-{rid}")),
            sales_order_id: Set(Some(SO_OWN)),
            customer_id: Set(CUSTOMER_ID),
            return_date: Set(Utc::now().date_naive()),
            warehouse_id: Set(WAREHOUSE_ID),
            reason: Set("归属锁测试退货原因".to_string()),
            status: Set("DRAFT".to_string()),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(None),
            created_by: Set(owner),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    sales_return_item::ActiveModel {
        id: Set(SR_ITEM_ID),
        return_id: Set(SR_OWN),
        line_no: Set(1),
        product_id: Set(PRODUCT_ID),
        quantity: Set(Decimal::ONE),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::ZERO),
        unit_price_foreign: Set(Decimal::ZERO),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        notes: Set(Some("归属锁测试退货明细".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        color_no: Set(String::new()),
        dye_lot_no: Set(String::new()),
        batch_no: Set(String::new()),
    }
    .insert(db)
    .await
    .unwrap();

    // ---- purchase_orders（D13，created_by/department_id 均 NOT NULL）----
    for (oid, owner) in [(PO_OWN, OWNER_A), (PO_CROSS, OWNER_B)] {
        purchase_order::ActiveModel {
            id: Set(oid),
            order_no: Set(format!("PO-GATE-{oid}")),
            supplier_id: Set(supplier_id),
            order_date: Set(Utc::now().date_naive()),
            warehouse_id: Set(WAREHOUSE_ID),
            department_id: Set(DEPT_ID),
            purchaser_id: Set(owner),
            currency: Set("CNY".to_string()),
            exchange_rate: Set(Decimal::ONE),
            total_amount: Set(Decimal::ZERO),
            total_amount_foreign: Set(Decimal::ZERO),
            total_quantity: Set(Decimal::ZERO),
            total_quantity_alt: Set(Decimal::ZERO),
            order_status: Set("DRAFT".to_string()),
            created_by: Set(owner),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    purchase_order_item::ActiveModel {
        id: Set(PO_ITEM_ID),
        order_id: Set(PO_OWN),
        line_no: Set(1),
        product_id: Set(PRODUCT_ID),
        quantity: Set(Decimal::ONE),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::ZERO),
        unit_price_foreign: Set(Decimal::ZERO),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        received_quantity: Set(Decimal::ZERO),
        received_quantity_alt: Set(Decimal::ZERO),
        quantity_tolerance_pct: Set(None),
        notes: Set(Some("归属锁测试采购明细".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // ---- purchase_return（D14，created_by/department_id 均可空；本波按 owner 归属播种）----
    for (rid, owner) in [(PR_OWN, OWNER_A), (PR_CROSS, OWNER_B)] {
        purchase_return::ActiveModel {
            id: Set(rid),
            return_no: Set(format!("PR-GATE-{rid}")),
            receipt_id: Set(None),
            order_id: Set(Some(PO_OWN)),
            supplier_id: Set(supplier_id),
            return_date: Set(Utc::now().date_naive()),
            warehouse_id: Set(Some(WAREHOUSE_ID)),
            department_id: Set(Some(DEPT_ID)),
            reason_type: Set(Some("质量问题".to_string())),
            reason_detail: Set(None),
            return_status: Set(Some("DRAFT".to_string())),
            total_quantity: Set(Some(Decimal::ZERO)),
            total_quantity_alt: Set(Some(Decimal::ZERO)),
            total_amount: Set(Some(Decimal::ZERO)),
            notes: Set(None),
            created_by: Set(Some(owner)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    purchase_return_item::ActiveModel {
        id: Set(PR_ITEM_ID),
        return_id: Set(PR_OWN),
        line_no: Set(1),
        product_id: Set(PRODUCT_ID),
        quantity: Set(Decimal::ONE),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::ZERO),
        unit_price_foreign: Set(Decimal::ZERO),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        notes: Set(Some("归属锁测试采购退货明细".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        color_no: Set(String::new()),
        dye_lot_no: Set(String::new()),
        batch_no: Set(String::new()),
    }
    .insert(db)
    .await
    .unwrap();
}

/// 断言失败信封形态：403 + FORBIDDEN，键齐全，脱敏文案不含任何被测 id。
fn assert_cross_denied(status: StatusCode, v: &Value, leaked_ids: &[i32]) {
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主子面必须 403（修复前为 2xx 返回他人子行/翻转他人状态）: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN", "失败信封 code 契约: {v}");
    assert_ne!(v["code"], "INTERNAL_ERROR", "越权不得拍平成 500");
    assert_ne!(status, StatusCode::OK, "越权严禁伪装成正常 2xx 空列表");
    assert!(
        v["message"].is_string() && v["trace_id"].is_string() && v["timestamp"].is_number(),
        "失败信封键齐全（code/message/trace_id/timestamp）: {v}"
    );
    let msg = v["message"].as_str().unwrap_or_default();
    for id in leaked_ids {
        assert!(
            !msg.contains(&id.to_string()),
            "对外文案不得拼记录 ID {id}: {v}"
        );
    }
}

/// 回读某销售订单当前状态（同一已迁移连接，证明写口放行真写 / 越权零漂移）。
async fn read_so_status(db: &sea_orm::DatabaseConnection, id: i32) -> String {
    sales_order::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .status
}

// =============================================================
// D09 GET /sales/orders/{id}/history
// =============================================================
#[tokio::test]
async fn d09_history_of_own_order_is_2xx_and_truthful() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/orders/{id}/history", get(get_order_history))
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/sales/orders/{SO_OWN}/history"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 订单历史应 2xx: {v}");
    assert_eq!(v["code"], 200);
    let rows = v["data"]["list"]
        .as_array()
        .unwrap_or_else(|| panic!("data.list 应为历史数组: {v}"));
    assert!(
        rows.iter().any(|r| r["order_id"] == SO_OWN),
        "应如实回显本单真实变更行（非空、非 mock）: {v}"
    );
}

#[tokio::test]
async fn d09_history_of_others_order_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/orders/{id}/history", get(get_order_history))
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/sales/orders/{SO_CROSS}/history"),
        None,
    )
    .await;
    assert_cross_denied(status, &v, &[SO_CROSS]);
}

// =============================================================
// D10 POST /sales/orders/{id}/reject（写口，另钉状态零漂移）
// =============================================================
#[tokio::test]
async fn d10_reject_own_order_is_2xx_and_flips_status() {
    let (app, db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/orders/{id}/reject", post(reject_order))
    })
    .await;
    assert_eq!(read_so_status(&db, SO_OWN).await, "pending");
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/sales/orders/{SO_OWN}/reject"),
        Some(json!({ "reason": "客户信用超限" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 拒单应 2xx: {v}");
    // 放行必须真写：回读真库确认状态从 pending 翻转为 rejected（证明不是先拒后放的假绿）
    assert_eq!(
        read_so_status(&db, SO_OWN).await,
        "rejected",
        "合法 owner 拒单应真实翻转状态为 rejected"
    );
}

#[tokio::test]
async fn d10_reject_others_order_is_403_and_zero_status_drift() {
    let (app, db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/orders/{id}/reject", post(reject_order))
    })
    .await;
    assert_eq!(
        read_so_status(&db, SO_CROSS).await,
        "pending",
        "前提：他人订单初始 pending"
    );

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/sales/orders/{SO_CROSS}/reject"),
        Some(json!({ "reason": "越权拒绝" })),
    )
    .await;
    assert_cross_denied(status, &v, &[SO_CROSS]);

    assert_eq!(
        read_so_status(&db, SO_CROSS).await,
        "pending",
        "越权拒单被拒后订单状态必须零漂移（不得先写再拒）"
    );
}

// =============================================================
// D11 GET /sales/orders/{id}/deliveries
// =============================================================
#[tokio::test]
async fn d11_deliveries_of_own_order_is_2xx_and_truthful() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/orders/{id}/deliveries", get(get_order_deliveries))
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/sales/orders/{SO_OWN}/deliveries"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 发货记录应 2xx: {v}");
    assert_eq!(v["code"], 200);
    let rows = v["data"]["list"]
        .as_array()
        .unwrap_or_else(|| panic!("data.list 应为发货数组: {v}"));
    assert!(
        rows.iter().any(|r| r["id"] == DELIVERY_ID),
        "应如实回显本单真实发货行（非空、非 mock）: {v}"
    );
}

#[tokio::test]
async fn d11_deliveries_of_others_order_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/orders/{id}/deliveries", get(get_order_deliveries))
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/sales/orders/{SO_CROSS}/deliveries"),
        None,
    )
    .await;
    assert_cross_denied(status, &v, &[SO_CROSS]);
}

// =============================================================
// D12 GET /sales/sales-returns/{id}/items
// =============================================================
#[tokio::test]
async fn d12_return_items_of_own_return_is_2xx_and_truthful() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/sales-returns/{id}/items", get(list_return_items))
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/sales/sales-returns/{SR_OWN}/items"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 退货明细应 2xx: {v}");
    assert_eq!(v["code"], 200);
    let rows = v["data"]
        .as_array()
        .unwrap_or_else(|| panic!("data 应为退货明细数组: {v}"));
    assert!(
        rows.iter().any(|r| r["id"] == SR_ITEM_ID),
        "应如实回显本退货单真实明细行（非空、非 mock）: {v}"
    );
}

#[tokio::test]
async fn d12_return_items_of_others_return_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/sales-returns/{id}/items", get(list_return_items))
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/sales/sales-returns/{SR_CROSS}/items"),
        None,
    )
    .await;
    assert_cross_denied(status, &v, &[SR_CROSS]);
}

// =============================================================
// D13 GET /purchase/orders/{id}/items
// =============================================================
#[tokio::test]
async fn d13_order_items_of_own_order_is_2xx_and_truthful() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/purchase/orders/{id}/items", get(list_order_items))
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/purchase/orders/{PO_OWN}/items"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 采购明细应 2xx: {v}");
    assert_eq!(v["code"], 200);
    let rows = v["data"]
        .as_array()
        .unwrap_or_else(|| panic!("data 应为采购明细数组: {v}"));
    assert!(
        rows.iter().any(|r| r["id"] == PO_ITEM_ID),
        "应如实回显本采购单真实明细行（非空、非 mock）: {v}"
    );
}

#[tokio::test]
async fn d13_order_items_of_others_order_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/purchase/orders/{id}/items", get(list_order_items))
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/purchase/orders/{PO_CROSS}/items"),
        None,
    )
    .await;
    assert_cross_denied(status, &v, &[PO_CROSS]);
}

// =============================================================
// D14 GET /purchase/returns/{id}/items
// =============================================================
#[tokio::test]
async fn d14_return_items_of_own_return_is_2xx_and_truthful() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route(
            "/purchase/returns/{id}/items",
            get(list_purchase_return_items),
        )
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/purchase/returns/{PR_OWN}/items"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 采购退货明细应 2xx: {v}");
    assert_eq!(v["code"], 200);
    let rows = v["data"]
        .as_array()
        .unwrap_or_else(|| panic!("data 应为采购退货明细数组: {v}"));
    assert!(
        rows.iter().any(|r| r["id"] == PR_ITEM_ID),
        "应如实回显本采购退货单真实明细行（非空、非 mock）: {v}"
    );
}

#[tokio::test]
async fn d14_return_items_of_others_return_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route(
            "/purchase/returns/{id}/items",
            get(list_purchase_return_items),
        )
    })
    .await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/purchase/returns/{PR_CROSS}/items"),
        None,
    )
    .await;
    assert_cross_denied(status, &v, &[PR_CROSS]);
}

// =============================================================
// 存在性：路径 id 不存在走既有 not_found（404），不得裸 500 / 空集合冒充无权限
// =============================================================
#[tokio::test]
async fn missing_parent_order_history_is_404_not_empty_200() {
    let (app, _db) = seeded_app(make_auth(OWNER_A), |r| {
        r.route("/sales/orders/{id}/history", get(get_order_history))
    })
    .await;
    let (status, v) = call(&app, Method::GET, "/sales/orders/999999/history", None).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "不存在订单历史应走 not_found（404），不裸 500、不空列表冒充: {v}"
    );
    assert_ne!(status, StatusCode::OK, "不存在严禁伪装成正常 2xx 空列表");
}
