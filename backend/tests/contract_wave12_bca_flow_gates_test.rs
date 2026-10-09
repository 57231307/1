//! 大货批色流程写端点（剪样/通过/拒绝/返工）行级归属门契约锁。
//!
//! 功能：钉死 cut_sample、customer_approve、customer_reject、customer_rework 四端点在
//! 进入 service 写事务前必须先过父销售订单归属门，防水平越权（IDOR）回潮。
//!
//! 调用方：集成测试层（真已迁移 PostgreSQL，TEST_DATABASE_URL）。
//!
//! 入参：经 `test_common::setup_test_db()` 连接、清空业务表后播种分属两个 self 范围用户
//! OWNER_A / OWNER_B 的销售订单、染色批次、库存记录、批色记录；再以不同 AuthContext
//! （data_scope=self/dept）走真 HTTP 装配。
//!
//! 判据（逐端点三条）：
//! - 本人可见(Self) => 2xx 且回读状态与联动数据如实变更；
//! - 他人(Self) => 403 + FORBIDDEN，回读断言状态/库存/history 零漂移；
//! - Dept 范围 => 父单属于可见部门时 2xx、不属于时 403。
//!
//! 反空操作自证：修复前四 handler 把会话提取器写成 `auth`（仅取 user_id 传 service，
//! 从不判归属），故"他人应 403 / 零漂移"各条在修复前拿到 2xx，断言必红；判红分叉即
//! 各 handler 新增的 `ensure_parent_sales_order_access` 早退点——门返回 Err 后直接
//! 短路，不触达 service.cut_sample/customer_approve/customer_reject/customer_rework，
//! 故越权路径既不扣库存也不流转状态也不留 history 行。
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
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::bulk_color_approval_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    bulk_color_approval, bulk_color_approval_history, customer, department, dye_batch,
    inventory_piece, inventory_stock, product, sales_order, user, warehouse,
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
const DEPT_A: i32 = 9500;
const DEPT_B: i32 = 9600;

const SO_OWN: i32 = 9201;
const SO_CROSS: i32 = 9202;
const SO_DEPT_B: i32 = 9203;
const CUS_ID: i32 = 9150;
const DYE_ID: i32 = 9301;
const STOCK_ID: i32 = 9350;

// 批色记录 id
const BCA_CUT: i64 = 9401; // pending，剪样目标，父 SO_OWN（归 OWNER_A / DEPT_A）
const BCA_CUT_CROSS: i64 = 9402; // pending，剪样跨主目标，父 SO_CROSS（归 OWNER_B / DEPT_A）
const BCA_CUT_DEPT_B: i64 = 9403; // pending，剪样 Dept-B 不可见目标，父 SO_DEPT_B（归 OWNER_B / DEPT_B）

const BCA_APPR: i64 = 9411; // sent_to_customer，通过目标，父 SO_OWN
const BCA_APPR_CROSS: i64 = 9412; // sent_to_customer，通过跨主目标，父 SO_CROSS
const BCA_APPR_DEPT_B: i64 = 9413; // sent_to_customer，通过 Dept-B 不可见目标，父 SO_DEPT_B

const BCA_REJ: i64 = 9421; // sent_to_customer，拒绝目标，父 SO_OWN
const BCA_REJ_CROSS: i64 = 9422; // sent_to_customer，拒绝跨主目标，父 SO_CROSS
const BCA_REJ_DEPT_B: i64 = 9423; // sent_to_customer，拒绝 Dept-B 不可见目标，父 SO_DEPT_B

const BCA_RWK: i64 = 9431; // sent_to_customer，返工目标，父 SO_OWN
const BCA_RWK_CROSS: i64 = 9432; // sent_to_customer，返工跨主目标，父 SO_CROSS
const BCA_RWK_DEPT_B: i64 = 9433; // sent_to_customer，返工 Dept-B 不可见目标，父 SO_DEPT_B

// ---- Auth 构造 ----

fn make_self_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("bca_flow_gate_{user_id}"),
        role_id: Some(2),
        department_id: Some(DEPT_A),
        data_scope: Some("self".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

fn make_dept_auth(user_id: i32, visible_depts: &[i32]) -> AuthContext {
    let csv = visible_depts
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",");
    AuthContext {
        user_id,
        username: format!("bca_flow_gate_dept_{user_id}"),
        role_id: Some(2),
        department_id: Some(DEPT_A),
        data_scope: Some("dept".to_string()),
        dept_ids: Some(Arc::new(csv)),
        dept_member_user_ids: None,
    }
}

// ---- 中间件注入 ----

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

// ---- 数据播种 ----

async fn seed(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();

    // 部门参照种子（必须在插用户前）：夹具 `services/test_common.rs` 把 `departments`
    // 列入 SEALED_REFERENCE_TABLES（不参与 TRUNCATE），迁移又只播种 id 1~5（m0001），
    // 而本文件把 DEPT_A=9500 / DEPT_B=9600 写进 `users.department_id`（fk_users_department）
    // 与 `sales_order.department_id`，故必须自建这两行。因 departments 跨用例/跨文件在同一
    // 直库上持续累积、只有迁移种子 1~5 被复用，重复插固定主键会撞 23505，故照既有幂等口径
    // "按主键查、已存在即跳过"（CI 以 `--test-threads=1` 串行跑真库写删，见 ci-cd.yml，查后即插无竞态）。
    for (dept_id, code, name) in [
        (DEPT_A, "D-BCAFLOW-A", "批色流程门测试部门A"),
        (DEPT_B, "D-BCAFLOW-B", "批色流程门测试部门B"),
    ] {
        let exists = department::Entity::find_by_id(dept_id)
            .one(db)
            .await
            .unwrap()
            .is_some();
        if !exists {
            department::ActiveModel {
                id: Set(dept_id),
                name: Set(name.to_string()),
                code: Set(code.to_string()),
                sort_order: Set(0),
                is_active: Set(true),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .unwrap();
        }
    }

    // 用户
    for uid in [OWNER_A, OWNER_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("bca_flow_gate_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(DEPT_A)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 客户（sales_order FK 需要）
    customer::ActiveModel {
        id: Set(CUS_ID),
        customer_code: Set("CUS-BCAFLOW".to_string()),
        customer_name: Set("批色流程门测试客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OWNER_A),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 父销售订单：归属列 created_by + department_id
    // SO_OWN: created_by=OWNER_A, department_id=DEPT_A
    // SO_CROSS: created_by=OWNER_B, department_id=DEPT_A
    // SO_DEPT_B: created_by=OWNER_B, department_id=DEPT_B（Dept_A 不可见）
    for (so_id, owner, dept) in [
        (SO_OWN, OWNER_A, DEPT_A),
        (SO_CROSS, OWNER_B, DEPT_A),
        (SO_DEPT_B, OWNER_B, DEPT_B),
    ] {
        sales_order::ActiveModel {
            id: Set(so_id),
            order_no: Set(format!("SO-BCAFLOW-{so_id}")),
            customer_id: Set(CUS_ID),
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
            department_id: Set(Some(dept)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 染色批次（状态 "completed" 满足剪样前置）
    dye_batch::ActiveModel {
        id: Set(DYE_ID),
        batch_no: Set("DYE-BCAFLOW".to_string()),
        color_code: Set("C99".to_string()),
        color_name: Set("流程门测试色".to_string()),
        dye_lot_no: Set("DL-BCAFLOW".to_string()),
        status: Set(Some("completed".to_string())),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 库存父行：下方 inventory_stocks 行的 product_id=1、warehouse_id=1 分别受
    // `fk_inventory_product`→products(id)、`fk_inventory_warehouse`→warehouses(id) 两条外键约束
    // （定义见 migration/src/domain/system/m0001_initial_schema.rs）。products/warehouses
    // 均属逐用例 TRUNCATE 的业务表（不在 test_common SEALED_REFERENCE_TABLES），迁移不播种，
    // 清空后无 id=1 父行，故此处按被引用的固定主键自建：products 的 NOT NULL 列 code/name、
    // warehouses 的 NOT NULL 列 name/warehouse_code 均按建表 DDL 给值。
    warehouse::ActiveModel {
        id: Set(1),
        warehouse_code: Set("W-BCAFLOW".to_string()),
        name: Set("批色流程门测试仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    product::ActiveModel {
        id: Set(1),
        code: Set("P-BCAFLOW".to_string()),
        name: Set("批色流程门测试面料".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("成品布".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 库存记录（匹配 dye_batch 的 dye_lot_no + batch_no，quality_status="合格"）
    inventory_stock::ActiveModel {
        id: Set(STOCK_ID),
        warehouse_id: Set(1),
        product_id: Set(1),
        quantity_on_hand: Set(Decimal::from(100)),
        quantity_available: Set(Decimal::from(100)),
        quantity_reserved: Set(Decimal::ZERO),
        quantity_shipped: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(Decimal::ZERO),
        max_stock_point: Set(Decimal::ZERO),
        reorder_quantity: Set(Decimal::ZERO),
        bin_location: Set(None),
        last_count_date: Set(None),
        last_movement_date: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        batch_no: Set("DYE-BCAFLOW".to_string()),
        color_no: Set("C99".to_string()),
        dye_lot_no: Set(Some("DL-BCAFLOW".to_string())),
        grade: Set("一等品".to_string()),
        production_date: Set(None),
        expiry_date: Set(None),
        quantity_meters: Set(Decimal::from(100)),
        quantity_kg: Set(Decimal::from(50)),
        gram_weight: Set(None),
        width: Set(None),
        location_id: Set(None),
        shelf_no: Set(None),
        layer_no: Set(None),
        stock_status: Set("正常".to_string()),
        quality_status: Set("合格".to_string()),
        version: Set(1),
        replenishment_strategy: Set("reorder_point".to_string()),
    }
    .insert(db)
    .await
    .unwrap();

    // 批色记录
    let rows = [
        (BCA_CUT, SO_OWN, "pending"),
        (BCA_CUT_CROSS, SO_CROSS, "pending"),
        (BCA_CUT_DEPT_B, SO_DEPT_B, "pending"),
        (BCA_APPR, SO_OWN, "sent_to_customer"),
        (BCA_APPR_CROSS, SO_CROSS, "sent_to_customer"),
        (BCA_APPR_DEPT_B, SO_DEPT_B, "sent_to_customer"),
        (BCA_REJ, SO_OWN, "sent_to_customer"),
        (BCA_REJ_CROSS, SO_CROSS, "sent_to_customer"),
        (BCA_REJ_DEPT_B, SO_DEPT_B, "sent_to_customer"),
        (BCA_RWK, SO_OWN, "sent_to_customer"),
        (BCA_RWK_CROSS, SO_CROSS, "sent_to_customer"),
        (BCA_RWK_DEPT_B, SO_DEPT_B, "sent_to_customer"),
    ];
    for (id, so, status) in rows {
        bulk_color_approval::ActiveModel {
            id: Set(id),
            sales_order_id: Set(so),
            dye_batch_id: Set(DYE_ID),
            customer_id: Set(CUS_ID as i64),
            production_order_id: Set(None),
            product_id: Set(None),
            color_no: Set(Some("C99".to_string())),
            dye_lot_no: Set(Some("DL-BCAFLOW".to_string())),
            batch_no: Set(Some("DYE-BCAFLOW".to_string())),
            sample_type: Set("cut_sample".to_string()),
            sample_piece_id: Set(None),
            sample_length_m: Set(None),
            approval_status: Set(status.to_string()),
            approver_id: Set(None),
            approval_date: Set(None),
            sent_to_customer_at: Set(Some(now)),
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
}

// ---- App 组装 ----

fn build_app(auth: AuthContext, db: Arc<sea_orm::DatabaseConnection>) -> Router {
    let state = AppState {
        db,
        ..Default::default()
    };
    Router::new()
        .route(
            "/erp/bulk-color-approvals/{id}/cut-sample",
            post(bulk_color_approval_handler::cut_sample),
        )
        .route(
            "/erp/bulk-color-approvals/{id}/approve",
            post(bulk_color_approval_handler::customer_approve),
        )
        .route(
            "/erp/bulk-color-approvals/{id}/reject",
            post(bulk_color_approval_handler::customer_reject),
        )
        .route(
            "/erp/bulk-color-approvals/{id}/rework",
            post(bulk_color_approval_handler::customer_rework),
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

// ---- 回读辅助 ----

async fn approval_status(db: &sea_orm::DatabaseConnection, id: i64) -> String {
    bulk_color_approval::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .approval_status
}

async fn history_count(db: &sea_orm::DatabaseConnection, approval_id: i64) -> u64 {
    bulk_color_approval_history::Entity::find()
        .filter(bulk_color_approval_history::Column::BulkColorApprovalId.eq(approval_id))
        .count(db)
        .await
        .unwrap()
}

async fn stock_quantity_meters(db: &sea_orm::DatabaseConnection) -> Decimal {
    inventory_stock::Entity::find_by_id(STOCK_ID)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .quantity_meters
}

async fn piece_count_for_dye_lot(db: &sea_orm::DatabaseConnection) -> u64 {
    inventory_piece::Entity::find()
        .filter(inventory_piece::Column::DyeLotNo.eq("DL-BCAFLOW"))
        .filter(inventory_piece::Column::Status.eq("SAMPLE"))
        .count(db)
        .await
        .unwrap()
}

// ============ cut_sample：剪大货样 ============

#[tokio::test]
async fn cut_sample_owner_self_scope_succeeds_and_mutates() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_status = approval_status(&db, BCA_CUT).await;
    let before_qty = stock_quantity_meters(&db).await;
    let before_pieces = piece_count_for_dye_lot(&db).await;

    assert_eq!(before_status, "pending");
    assert_eq!(before_pieces, 0);

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_CUT}/cut-sample"),
        Some(json!({ "sample_length_m": "1.0" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人剪样应 2xx: {v}");
    assert_eq!(v["data"]["approval_status"].as_str(), Some("sampled"));

    let after_status = approval_status(&db, BCA_CUT).await;
    assert_eq!(after_status, "sampled", "回读审批状态应为 sampled");
    let after_qty = stock_quantity_meters(&db).await;
    assert_eq!(after_qty, before_qty - Decimal::ONE, "库存应扣减 1m");
    let after_pieces = piece_count_for_dye_lot(&db).await;
    assert_eq!(
        after_pieces,
        before_pieces + 1,
        "应生成 1 条 SAMPLE inventory_piece"
    );
}

#[tokio::test]
async fn cut_sample_cross_owner_is_forbidden_with_zero_drift() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_status = approval_status(&db, BCA_CUT_CROSS).await;
    let before_qty = stock_quantity_meters(&db).await;
    let before_pieces = piece_count_for_dye_lot(&db).await;
    let before_hist = history_count(&db, BCA_CUT_CROSS).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_CUT_CROSS}/cut-sample"),
        Some(json!({ "sample_length_m": "1.0" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主剪样必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    assert_eq!(
        approval_status(&db, BCA_CUT_CROSS).await,
        before_status,
        "越权后状态零漂移"
    );
    assert_eq!(
        stock_quantity_meters(&db).await,
        before_qty,
        "越权后库存零扣减"
    );
    assert_eq!(
        piece_count_for_dye_lot(&db).await,
        before_pieces,
        "越权后零新 inventory_piece"
    );
    assert_eq!(
        history_count(&db, BCA_CUT_CROSS).await,
        before_hist,
        "越权后 history 零新增"
    );
}

#[tokio::test]
async fn cut_sample_dept_scope_visible_dept_succeeds() {
    // Dept 用户 OWNER_A 可见 DEPT_A，父单 SO_OWN department_id=DEPT_A => 应放行
    let (app, db) = seeded_app(make_dept_auth(OWNER_A, &[DEPT_A])).await;
    let before_qty = stock_quantity_meters(&db).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_CUT}/cut-sample"),
        Some(json!({ "sample_length_m": "1.0" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Dept 可见部门剪样应 2xx: {v}");
    assert_eq!(v["data"]["approval_status"].as_str(), Some("sampled"));
    let after_qty = stock_quantity_meters(&db).await;
    assert!(after_qty < before_qty, "成功剪样后库存应扣减");
}

#[tokio::test]
async fn cut_sample_dept_scope_invisible_dept_is_forbidden() {
    // Dept 用户 OWNER_A 仅可见 DEPT_A，父单 SO_DEPT_B department_id=DEPT_B => 应 403
    let (app, db) = seeded_app(make_dept_auth(OWNER_A, &[DEPT_A])).await;
    let before_status = approval_status(&db, BCA_CUT_DEPT_B).await;
    let before_qty = stock_quantity_meters(&db).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_CUT_DEPT_B}/cut-sample"),
        Some(json!({ "sample_length_m": "1.0" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Dept 不可见部门剪样必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        approval_status(&db, BCA_CUT_DEPT_B).await,
        before_status,
        "零状态漂移"
    );
    assert_eq!(stock_quantity_meters(&db).await, before_qty, "零库存变动");
}

// ============ customer_approve：客户批色通过 ============

#[tokio::test]
async fn customer_approve_owner_self_scope_succeeds() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_status = approval_status(&db, BCA_APPR).await;
    assert_eq!(before_status, "sent_to_customer");

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_APPR}/approve"),
        Some(json!({ "feedback": "确认通过" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人通过应 2xx: {v}");
    assert_eq!(v["data"]["approval_status"].as_str(), Some("approved"));

    let after_status = approval_status(&db, BCA_APPR).await;
    assert_eq!(after_status, "approved", "回读审批状态应为 approved");
    let hist = history_count(&db, BCA_APPR).await;
    assert!(hist > 0, "通过后应有 history 记录");
}

#[tokio::test]
async fn customer_approve_cross_owner_is_forbidden_with_zero_drift() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_status = approval_status(&db, BCA_APPR_CROSS).await;
    let before_hist = history_count(&db, BCA_APPR_CROSS).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_APPR_CROSS}/approve"),
        Some(json!({ "feedback": "越权通过" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主通过必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    assert_eq!(
        approval_status(&db, BCA_APPR_CROSS).await,
        before_status,
        "越权后状态零漂移"
    );
    assert_eq!(
        history_count(&db, BCA_APPR_CROSS).await,
        before_hist,
        "越权后 history 零新增"
    );
}

#[tokio::test]
async fn customer_approve_dept_scope_visible_dept_succeeds() {
    let (app, db) = seeded_app(make_dept_auth(OWNER_A, &[DEPT_A])).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_APPR}/approve"),
        Some(json!({ "feedback": "Dept通过" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Dept 可见部门通过应 2xx: {v}");
    assert_eq!(v["data"]["approval_status"].as_str(), Some("approved"));
    assert_eq!(approval_status(&db, BCA_APPR).await, "approved");
}

#[tokio::test]
async fn customer_approve_dept_scope_invisible_dept_is_forbidden() {
    let (app, db) = seeded_app(make_dept_auth(OWNER_A, &[DEPT_A])).await;
    let before_status = approval_status(&db, BCA_APPR_DEPT_B).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_APPR_DEPT_B}/approve"),
        Some(json!({ "feedback": "越权通过" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Dept 不可见部门通过必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        approval_status(&db, BCA_APPR_DEPT_B).await,
        before_status,
        "零状态漂移"
    );
}

// ============ customer_reject：客户批色拒绝 ============

#[tokio::test]
async fn customer_reject_owner_self_scope_succeeds() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_status = approval_status(&db, BCA_REJ).await;
    assert_eq!(before_status, "sent_to_customer");

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_REJ}/reject"),
        Some(json!({ "reject_reason": "色差过大" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人拒绝应 2xx: {v}");
    assert_eq!(v["data"]["approval_status"].as_str(), Some("rejected"));

    let after_status = approval_status(&db, BCA_REJ).await;
    assert_eq!(after_status, "rejected", "回读审批状态应为 rejected");
}

#[tokio::test]
async fn customer_reject_cross_owner_is_forbidden_with_zero_drift() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_status = approval_status(&db, BCA_REJ_CROSS).await;
    let before_hist = history_count(&db, BCA_REJ_CROSS).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_REJ_CROSS}/reject"),
        Some(json!({ "reject_reason": "越权拒绝" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主拒绝必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    assert_eq!(
        approval_status(&db, BCA_REJ_CROSS).await,
        before_status,
        "越权后状态零漂移"
    );
    assert_eq!(
        history_count(&db, BCA_REJ_CROSS).await,
        before_hist,
        "越权后 history 零新增"
    );
}

#[tokio::test]
async fn customer_reject_dept_scope_visible_dept_succeeds() {
    let (app, db) = seeded_app(make_dept_auth(OWNER_A, &[DEPT_A])).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_REJ}/reject"),
        Some(json!({ "reject_reason": "Dept拒绝" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Dept 可见部门拒绝应 2xx: {v}");
    assert_eq!(v["data"]["approval_status"].as_str(), Some("rejected"));
    assert_eq!(approval_status(&db, BCA_REJ).await, "rejected");
}

#[tokio::test]
async fn customer_reject_dept_scope_invisible_dept_is_forbidden() {
    let (app, db) = seeded_app(make_dept_auth(OWNER_A, &[DEPT_A])).await;
    let before_status = approval_status(&db, BCA_REJ_DEPT_B).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_REJ_DEPT_B}/reject"),
        Some(json!({ "reject_reason": "越权拒绝" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Dept 不可见部门拒绝必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        approval_status(&db, BCA_REJ_DEPT_B).await,
        before_status,
        "零状态漂移"
    );
}

// ============ customer_rework：客户批色返工 ============

#[tokio::test]
async fn customer_rework_owner_self_scope_succeeds() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_status = approval_status(&db, BCA_RWK).await;
    assert_eq!(before_status, "sent_to_customer");

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_RWK}/rework"),
        Some(json!({ "reject_reason": "需重新染色" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人返工应 2xx: {v}");
    assert_eq!(v["data"]["approval_status"].as_str(), Some("rework"));

    let after_status = approval_status(&db, BCA_RWK).await;
    assert_eq!(after_status, "rework", "回读审批状态应为 rework");
}

#[tokio::test]
async fn customer_rework_cross_owner_is_forbidden_with_zero_drift() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;
    let before_status = approval_status(&db, BCA_RWK_CROSS).await;
    let before_hist = history_count(&db, BCA_RWK_CROSS).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_RWK_CROSS}/rework"),
        Some(json!({ "reject_reason": "越权返工" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "跨主返工必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    assert_eq!(
        approval_status(&db, BCA_RWK_CROSS).await,
        before_status,
        "越权后状态零漂移"
    );
    assert_eq!(
        history_count(&db, BCA_RWK_CROSS).await,
        before_hist,
        "越权后 history 零新增"
    );
}

#[tokio::test]
async fn customer_rework_dept_scope_visible_dept_succeeds() {
    let (app, db) = seeded_app(make_dept_auth(OWNER_A, &[DEPT_A])).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_RWK}/rework"),
        Some(json!({ "reject_reason": "Dept返工" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Dept 可见部门返工应 2xx: {v}");
    assert_eq!(v["data"]["approval_status"].as_str(), Some("rework"));
    assert_eq!(approval_status(&db, BCA_RWK).await, "rework");
}

#[tokio::test]
async fn customer_rework_dept_scope_invisible_dept_is_forbidden() {
    let (app, db) = seeded_app(make_dept_auth(OWNER_A, &[DEPT_A])).await;
    let before_status = approval_status(&db, BCA_RWK_DEPT_B).await;

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/erp/bulk-color-approvals/{BCA_RWK_DEPT_B}/rework"),
        Some(json!({ "reject_reason": "越权返工" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Dept 不可见部门返工必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        approval_status(&db, BCA_RWK_DEPT_B).await,
        before_status,
        "零状态漂移"
    );
}
