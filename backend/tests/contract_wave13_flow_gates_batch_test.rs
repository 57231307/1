//! 状态流转/写侧归属门的活体锁：为本波新补的「越权即可翻他人单据」这一类写端点
//! 钉死"本人/可见成员放行、他人拒绝且零落库、状态与关键数值零漂移"，防水平越权回潮。
//!
//! 功能：以真 HTTP 装配驱动七条写侧端点，逐条覆盖——
//!   - `POST /purchase/receipts/{id}/confirm`（确认入库：库存增、订单收货态推进、
//!     入库单 DRAFT→COMPLETED）；
//!   - `POST /purchase/receipts/{id}/concession`（让步接收：检验态 PENDING/REJECTED→
//!     CONCESSION_ACCEPTED，理由/操作人/时间落专用列）；
//!   - `POST /purchase/receipts/{id}/rejudge`（复检改判：让步态→PASSED/REJECTED，
//!     rejudge_count 累加）；归属经 `created_by` + `department_id` 过部门族门；
//!   - `POST /production-recipes/{id}/approve` 与
//!     `POST /production-recipes/additions/{id}/approve`（大货/加料处方 draft→approved、
//!     approved_by 落会话）：门在 handler 先 `service.get_by_id(id, Some(&ctx))`，
//!     归属经 `created_by` 过成员集合门（本表无 department_id 列）；
//!   - `POST /crm/transfer-approvals/{id}/manager-approve`（经理审批）：门在 service
//!     `get_pending_approval` 内按申请人 `applicant_id` 过成员集合门；
//!   - `POST /quality-8d-reports/{id}/advance`（8D 推进 D 阶段）：归属经两级父链
//!     `quality_8d_report → quality_issue → custom_order.created_by` 继承过成员集合门。
//! 调用方：集成测试层（真已迁移 PostgreSQL，`TEST_DATABASE_URL`；CI 串行、独占 service 库）。
//! 入参：经 `test_common::setup_test_db()` 清空业务表后，按私有 id 段播种分属两个 self
//! 范围用户的父资源链（收货单/处方/审批单/定制订单-质量异常-8D 报告），再以不同
//! `AuthContext`（data_scope=self，user_id=归属人 / 非归属人）走真 HTTP 装配。
//! 传给谁：上述七个 handler 及其写前置归属门。
//! 存什么：放行时目标行状态如实流转、审计列如实写入；越权时门在任何 service 落库点之前
//! return 403，状态列/关键数值列零漂移、不产生审计行。
//! 存哪里：`purchase_receipt` / `purchase_order_item` / `purchase_order` / `inventory_stock` /
//! `production_recipe` / `production_recipe_addition` / `customer_transfer_approvals` /
//! `quality_8d_reports` / `audit_log`（一律回读真库比对，绝不信任响应体）。
//!
//! 判据（逐端点，两条）：
//! - 本人 2xx 且目标行状态列如实流转、关键数值列如实落库（回读断言，非仅判 status==200）；
//! - 他人 ⇒ HTTP 403 + 机器码 `FORBIDDEN` + 文案恒为固定脱敏常量「无权限」（不含记录 ID）；
//!   回读目标行状态列/关键数值列零漂移；confirm 额外钉「库存行计数与订单已收数量、
//!   订单收货态零变化」；manager-approve 额外钉「不产生审计行」。
//! - confirm 反向还断言：越权被拒时入库单仍 DRAFT、`inventory_stock` 中该批次行数为 0、
//!   `purchase_order_item.received_quantity` 与 `purchase_order.order_status` 原样不动，
//!   证明门在 `confirm_receipt` 事务（进度/库存/COMPLETED 三写入口）之前即拦截。
//!
//! 反空操作自证：修复前这些 handler 的会话提取器写成 `_auth`（提取后丢弃）、函数体内零
//! 归属校验，写侧直接进 service 落库；故每条「他人 ⇒ 403 + 零漂移」断言在修复前会拿到
//! 2xx 且状态被真实翻动，必判红。判红分叉即本波新增的写前置归属门：门校验失败立即
//! `return Err(403)`，不触达后续事务/落库点，故越权路径既不流转状态、不改数值、不留审计。
//!
//! 夹具防法：`setup_test_db()` 自带 TRUNCATE 台账双向守卫；本文件不调 Migrator、不依赖
//! 迁移台账内容。suppliers / departments 为迁移种子参照表（id=1 恒在、不删改），仅复用之。

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
use bingxi_backend::handlers::{
    customer_transfer_approval_handler, production_recipe_handler, purchase_receipt_handler,
    quality_8d_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::purchase_inventory::purchase_receipt_inspection;
use bingxi_backend::models::{
    audit_log, custom_order, customer, inventory_stock, production_recipe,
    production_recipe_addition, purchase_order, purchase_order_item, purchase_receipt,
    purchase_receipt_item, quality_8d_report, quality_issue, user,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, Statement,
};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

// 别名：处方/加料处方/采购收货/采购订单/收货检验 状态词表（写入方权威常量，禁手写 token）
use bingxi_backend::models::status::production_recipe as recipe_status;
use bingxi_backend::models::status::production_recipe_addition as addition_status;
use bingxi_backend::models::status::purchase_order as po_status;
use bingxi_backend::models::status::purchase_receipt as receipt_status;

// ---- 私有 id 段（90xx-99xx），每用例经 setup_test_db 先 TRUNCATE 业务表，互不串库 ----
const U_A: i32 = 9010; // 归属人（放行方）
const U_B: i32 = 9011; // 非归属人（越权方）
const DEPT_SEED: i32 = 1; // 迁移种子部门（users/PO 复用，id=1 恒在）

const WH: i32 = 9020; // 自建仓库（收货/库存 FK）
const PROD: i32 = 9030; // 自建产品（明细/库存 FK）
const CUSTOMER: i32 = 9090; // 自建客户（定制订单 FK）
const SUPPLIER_SEED: i32 = 1; // 迁移种子供应商（收货单 FK，恒在）

const PO: i32 = 9040; // 采购订单
const POI: i32 = 9041; // 采购订单明细
const R_CONFIRM: i32 = 9050; // 收货单：PASSED，confirm 目标（挂 PO/POI）
const R_CONCEDE: i32 = 9051; // 收货单：PENDING，concession 目标
const R_REJUDGE: i32 = 9052; // 收货单：CONCESSION_ACCEPTED，rejudge 目标
const BATCH: &str = "G13-CONFIRM-BATCH"; // confirm 目标明细批次（库存行定位键）

const RECIPE_A: i32 = 9060; // 大货处方（draft，created_by=A）approve 目标
const ADDITION_A: i32 = 9061; // 加料处方（draft，created_by=A，父=RECIPE_A）approve 目标
const APPROVAL_A: i32 = 9072; // 客户转移审批单（pending，申请人=A）manager-approve 目标
const CO: i64 = 9080; // 定制订单（created_by=A）8D 归属根
const QI: i64 = 9081; // 质量异常（父=CO）
const RPT: i64 = 9082; // 8D 报告（d0_plan，父=QI）advance 目标

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn self_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("flowgate_{user_id}"),
        role_id: Some(2),
        department_id: Some(DEPT_SEED),
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

async fn exec_raw(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

async fn seed(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();

    for uid in [U_A, U_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("flowgate_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(DEPT_SEED)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    exec_raw(
        db,
        &format!(
            "INSERT INTO warehouses (id, name, warehouse_code, is_active) \
             VALUES ({WH}, '流转门锁仓', 'G13-W1', true)"
        ),
    )
    .await;
    exec_raw(
        db,
        &format!(
            "INSERT INTO products (id, code, name) \
             VALUES ({PROD}, 'G13-P1', '流转门契约测试坯布面料')"
        ),
    )
    .await;

    customer::ActiveModel {
        id: Set(CUSTOMER),
        customer_code: Set("CUS-G13-LOCK".to_string()),
        customer_name: Set("流转门锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(U_A),
        department_id: Set(Some(DEPT_SEED)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 采购订单 + 明细（confirm 目标挂此单，用于钉收货态零漂移）
    purchase_order::ActiveModel {
        id: Set(PO),
        order_no: Set("PO-G13-A".to_string()),
        supplier_id: Set(SUPPLIER_SEED),
        order_date: Set(date(2026, 3, 1)),
        warehouse_id: Set(WH),
        department_id: Set(DEPT_SEED),
        purchaser_id: Set(U_A),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(dec("100.00")),
        total_amount_foreign: Set(dec("100.00")),
        total_quantity: Set(dec("10.0000")),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(po_status::APPROVED.to_string()),
        created_by: Set(U_A),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    purchase_order_item::ActiveModel {
        id: Set(POI),
        order_id: Set(PO),
        line_no: Set(1),
        product_id: Set(PROD),
        quantity: Set(dec("10.0000")),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::ONE),
        unit_price_foreign: Set(Decimal::ONE),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(dec("10.00")),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(dec("10.00")),
        received_quantity: Set(Decimal::ZERO),
        received_quantity_alt: Set(Decimal::ZERO),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 收货单：confirm 目标（PASSED + DRAFT + 挂 PO + 一行批次明细，满足确认入库全部前置）
    purchase_receipt::ActiveModel {
        id: Set(R_CONFIRM),
        receipt_no: Set("GR-G13-CONFIRM".to_string()),
        order_id: Set(Some(PO)),
        supplier_id: Set(SUPPLIER_SEED),
        receipt_date: Set(date(2026, 9, 1)),
        warehouse_id: Set(WH),
        department_id: Set(None),
        inspection_status: Set(purchase_receipt_inspection::PASSED.to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(U_A),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    purchase_receipt_item::ActiveModel {
        receipt_id: Set(R_CONFIRM),
        order_item_id: Set(Some(POI)),
        line_no: Set(1),
        product_id: Set(PROD),
        material_code: Set("FAB-G13".to_string()),
        material_name: Set("流转门契约测试坯布".to_string()),
        batch_no: Set(Some(BATCH.to_string())),
        quantity: Set(dec("10.0000")),
        quantity_alt: Set(Some(dec("5.0000"))),
        unit_master: Set("米".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 收货单：concession 目标（PENDING，无明细即可，让步只翻检验态）
    purchase_receipt::ActiveModel {
        id: Set(R_CONCEDE),
        receipt_no: Set("GR-G13-CONCEDE".to_string()),
        supplier_id: Set(SUPPLIER_SEED),
        receipt_date: Set(date(2026, 9, 1)),
        warehouse_id: Set(WH),
        department_id: Set(None),
        inspection_status: Set(purchase_receipt_inspection::PENDING.to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(U_A),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 收货单：rejudge 目标（已让步态，改判前置）
    purchase_receipt::ActiveModel {
        id: Set(R_REJUDGE),
        receipt_no: Set("GR-G13-REJUDGE".to_string()),
        supplier_id: Set(SUPPLIER_SEED),
        receipt_date: Set(date(2026, 9, 1)),
        warehouse_id: Set(WH),
        department_id: Set(None),
        inspection_status: Set(purchase_receipt_inspection::CONCESSION_ACCEPTED.to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(U_A),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 大货处方（draft，created_by=A，明细非空满足审核前置）
    production_recipe::ActiveModel {
        id: Set(RECIPE_A),
        recipe_no: Set("PR-G13-A".to_string()),
        work_order_id: Set(None),
        dye_batch_id: Set(None),
        source_recipe_id: Set(None),
        lab_dip_resample_id: Set(None),
        customer_id: Set(Some(CUSTOMER)),
        color_no: Set(Some("G13-C1".to_string())),
        fabric_name: Set(Some("坯布".to_string())),
        fabric_spec: Set(None),
        fabric_width: Set(None),
        gram_weight: Set(None),
        fabric_weight: Set(dec("100.00")),
        equipment_no: Set(None),
        liquor_ratio: Set("1:8".to_string()),
        bath_volume: Set(None),
        adjustment_factor: Set(None),
        recipe_detail: Set(Some(vec![production_recipe::RecipeMaterialItem {
            material_code: "DYE-1".to_string(),
            material_name: "活性染料".to_string(),
            concentration: Some(dec("1.5")),
            unit: "kg".to_string(),
            amount: dec("1.5"),
            category: "dye".to_string(),
        }])),
        total_dye_cost: Set(None),
        total_auxiliary_cost: Set(None),
        status: Set(recipe_status::DRAFT.to_string()),
        approved_by: Set(None),
        approved_at: Set(None),
        issued_by: Set(Some(U_A)),
        printed_count: Set(Some(0)),
        remarks: Set(None),
        is_deleted: Set(false),
        created_by: Set(Some(U_A)),
        created_at: Set(now.fixed_offset()),
        updated_at: Set(now.fixed_offset()),
    }
    .insert(db)
    .await
    .unwrap();

    // 加料处方（draft，created_by=A，父=RECIPE_A，明细非空）
    production_recipe_addition::ActiveModel {
        id: Set(ADDITION_A),
        addition_no: Set("PA-G13-A".to_string()),
        production_recipe_id: Set(RECIPE_A),
        work_order_id: Set(None),
        dye_batch_id: Set(None),
        addition_reason: Set(Some("色差补料".to_string())),
        addition_detail: Set(Some(vec![
            production_recipe_addition::AdditionMaterialItem {
                material_code: "AUX-1".to_string(),
                material_name: "匀染剂".to_string(),
                amount: dec("0.5"),
                unit: "kg".to_string(),
                category: "auxiliary".to_string(),
            },
        ])),
        total_cost: Set(None),
        status: Set(addition_status::DRAFT.to_string()),
        approved_by: Set(None),
        approved_at: Set(None),
        issued_by: Set(Some(U_A)),
        remarks: Set(None),
        is_deleted: Set(false),
        created_by: Set(Some(U_A)),
        created_at: Set(now.fixed_offset()),
        updated_at: Set(now.fixed_offset()),
    }
    .insert(db)
    .await
    .unwrap();

    // 客户转移审批单（pending，level1/max1，申请人=A；本表无 FK，lead_id/user_id 仅业务整型）
    customer_transfer_approval_seed(db, APPROVAL_A, U_A).await;

    // 8D 归属链：custom_order(created_by=A) → quality_issue → quality_8d_report(d0_plan)
    custom_order::ActiveModel {
        id: Set(CO),
        order_no: Set("CO-G13-A".to_string()),
        customer_id: Set(i64::from(CUSTOMER)),
        product_id: Set(i64::from(PROD)),
        color_id: Set(None),
        spec: Set("g13-spec".to_string()),
        quantity: Set(Decimal::ONE),
        unit: Set("m".to_string()),
        custom_requirements: Set(json!({})),
        status: Set("draft".to_string()),
        currency: Set("CNY".to_string()),
        created_by: Set(Some(CO_OWNER_A)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    quality_issue::ActiveModel {
        id: Set(QI),
        custom_order_id: Set(CO),
        process_node_id: Set(None),
        issue_type: Set("quality".to_string()),
        // severity 取值权威为 DB CHECK `chk_issue_severity`（m0044）IN
        // ('low','medium','high','critical')；`"major"` 是词表外值、插入即 CHECK 违例。
        // 本处是 8D 归属链的正向前置种子（非测越界被拒的负例），按同族
        // custom_order_process_gates_test.rs 的种法取合法高值 high。
        severity: Set("high".to_string()),
        description: Set("流转门契约质量异常".to_string()),
        discovered_at: Set(now),
        resolved_at: Set(None),
        resolution: Set(None),
        status: Set("open".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        root_cause_method: Set(None),
        root_cause_detail: Set(None),
        permanent_action_owner: Set(None),
        permanent_action_due_date: Set(None),
        permanent_action_completed_at: Set(None),
    }
    .insert(db)
    .await
    .unwrap();

    quality_8d_report::ActiveModel {
        id: Set(RPT),
        quality_issue_id: Set(QI),
        status: Set("d0_plan".to_string()),
        d0_date: Set(Some(now)),
        d0_prepared_by: Set(Some(U_A)),
        d0_plan: Set(Some("组建团队前的准备".to_string())),
        d1_date: Set(None),
        d1_team_members: Set(None),
        d2_date: Set(None),
        d2_problem_description: Set(None),
        d3_date: Set(None),
        d3_interim_action: Set(None),
        d4_date: Set(None),
        d4_root_cause_method: Set(None),
        d4_root_cause_detail: Set(None),
        d4_root_cause_summary: Set(None),
        d5_date: Set(None),
        d5_permanent_action: Set(None),
        d5_action_owner: Set(None),
        d5_due_date: Set(None),
        d5_completed_at: Set(None),
        d6_date: Set(None),
        d6_verification_result: Set(None),
        d7_date: Set(None),
        d7_prevention_action: Set(None),
        d8_date: Set(None),
        d8_closure_summary: Set(None),
        closed_at: Set(None),
        closed_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .unwrap();
}

// 定制订单归属人列是 i64；与收货/处方域的 i32 用户 id 数值一致，此处单列常量声明。
const CO_OWNER_A: i64 = U_A as i64;

async fn customer_transfer_approval_seed(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    applicant: i32,
) {
    let now = Utc::now();
    bingxi_backend::models::customer_transfer_approval::ActiveModel {
        id: Set(id),
        approval_no: Set(format!("TA-G13-{id}")),
        lead_id: Set(1),
        company_name: Set(Some("流转门锁线索客户".to_string())),
        from_user_id: Set(applicant),
        from_user_name: Set(Some(format!("flowgate_{applicant}"))),
        to_user_id: Set(U_B),
        to_user_name: Set(None),
        applicant_id: Set(applicant),
        reason: Set("客户要求转归属".to_string()),
        is_large_customer: Set(false),
        approval_status: Set(
            bingxi_backend::models::customer_transfer_approval::STATUS_PENDING.to_string(),
        ),
        current_level: Set(1),
        max_level: Set(1),
        manager_approver_id: Set(None),
        manager_comment: Set(None),
        manager_approved_at: Set(None),
        director_approver_id: Set(None),
        director_comment: Set(None),
        director_approved_at: Set(None),
        completed_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
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
            "/purchase/receipts/{id}/confirm",
            post(purchase_receipt_handler::confirm_receipt),
        )
        .route(
            "/purchase/receipts/{id}/concession",
            post(purchase_receipt_handler::concede_receipt),
        )
        .route(
            "/purchase/receipts/{id}/rejudge",
            post(purchase_receipt_handler::rejudge_receipt),
        )
        .route(
            "/production-recipes/{id}/approve",
            post(production_recipe_handler::approve),
        )
        .route(
            "/production-recipes/additions/{id}/approve",
            post(production_recipe_handler::approve_addition),
        )
        .route(
            "/crm/transfer-approvals/{id}/manager-approve",
            post(customer_transfer_approval_handler::manager_approve),
        )
        .route(
            "/quality-8d-reports/{id}/advance",
            post(quality_8d_handler::advance),
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

/// 断言越权信封：HTTP 403 + 机器码 FORBIDDEN + 固定脱敏文案「无权限」（不含记录 ID）+ 绝不 2xx。
fn assert_forbidden(status: StatusCode, v: &Value, leaked_id: i64, endpoint: &str) {
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "{endpoint} 越权必须 403（绝不放行、绝不降级成 2xx）: {v}"
    );
    assert_eq!(
        v["code"].as_str(),
        Some("FORBIDDEN"),
        "{endpoint} 越权机器码必须为 FORBIDDEN: {v}"
    );
    let msg = v["message"]
        .as_str()
        .unwrap_or_else(|| panic!("{endpoint} 403 响应应含 message 字段: {v}"));
    assert_eq!(
        msg, "无权限",
        "{endpoint} 越权文案必须是固定脱敏常量「无权限」，实得: {msg}"
    );
    assert!(
        !msg.contains(&leaked_id.to_string()),
        "{endpoint} 脱敏文案严禁泄露记录 ID（{leaked_id}）: {msg}"
    );
}

// ==================================================================
// 组一 · 采购收货确认 / 让步接收 / 复检改判（门 ensure_receipt_access，经 created_by+department_id）
// ==================================================================

#[tokio::test]
async fn confirm_receipt_owner_advances_stock_and_po_state() {
    let (app, db) = seeded_app(self_auth(U_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/purchase/receipts/{R_CONFIRM}/confirm"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人确认入库应 2xx: {v}");

    let row = purchase_receipt::Entity::find_by_id(R_CONFIRM)
        .one(&*db)
        .await
        .unwrap()
        .expect("收货单应存在");
    assert_eq!(
        row.receipt_status,
        receipt_status::COMPLETED.to_string(),
        "本人确认后入库状态应如实迁到 COMPLETED，实得: {}",
        row.receipt_status
    );

    let stock = inventory_stock::Entity::find()
        .filter(inventory_stock::Column::BatchNo.eq(BATCH))
        .one(&*db)
        .await
        .unwrap()
        .expect("本人确认必须真实落库存行");
    assert_eq!(
        stock.quantity_meters,
        dec("10.0000"),
        "库存数量应等于明细数量"
    );

    let oi = purchase_order_item::Entity::find_by_id(POI)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        oi.received_quantity,
        dec("10.0000"),
        "订单明细已收数量应如实累加"
    );

    let po = purchase_order::Entity::find_by_id(PO)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        po.order_status,
        po_status::COMPLETED.to_string(),
        "全部收货后订单收货态应真实推进，实得: {}",
        po.order_status
    );
}

#[tokio::test]
async fn confirm_receipt_cross_owner_denied_with_zero_drift() {
    let (app, db) = seeded_app(self_auth(U_B)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/purchase/receipts/{R_CONFIRM}/confirm"),
        None,
    )
    .await;
    assert_forbidden(status, &v, i64::from(R_CONFIRM), "confirm");

    // 回读真库：门在 confirm 事务三写入口之前拦截，状态/数值零漂移。
    let row = purchase_receipt::Entity::find_by_id(R_CONFIRM)
        .one(&*db)
        .await
        .unwrap()
        .expect("越权被拒后收货单应原样存在");
    assert_eq!(
        row.receipt_status,
        receipt_status::DRAFT.to_string(),
        "越权确认严禁把他人入库单状态翻成 COMPLETED"
    );

    let stock_count = inventory_stock::Entity::find()
        .filter(inventory_stock::Column::BatchNo.eq(BATCH))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(stock_count, 0, "越权确认严禁新增他人批次库存行");

    let oi = purchase_order_item::Entity::find_by_id(POI)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        oi.received_quantity,
        Decimal::ZERO,
        "越权确认严禁推进他人订单明细已收数量"
    );

    let po = purchase_order::Entity::find_by_id(PO)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        po.order_status,
        po_status::APPROVED.to_string(),
        "越权确认严禁改动他人订单收货态"
    );
}

#[tokio::test]
async fn concede_receipt_owner_transitions_inspection_and_lands_operator() {
    let (app, db) = seeded_app(self_auth(U_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/purchase/receipts/{R_CONCEDE}/concession"),
        Some(json!({ "reason": "色差轻微特采" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人让步接收应 2xx: {v}");

    let row = purchase_receipt::Entity::find_by_id(R_CONCEDE)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::CONCESSION_ACCEPTED.to_string(),
        "本人让步接收检验态应如实流转"
    );
    assert_eq!(row.concession_by, Some(U_A), "让步操作人应如实落会话用户");
    assert!(row.concession_at.is_some(), "让步时间应真实写入");
}

#[tokio::test]
async fn concede_receipt_cross_owner_denied_with_zero_drift() {
    let (app, db) = seeded_app(self_auth(U_B)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/purchase/receipts/{R_CONCEDE}/concession"),
        Some(json!({ "reason": "越权特采" })),
    )
    .await;
    assert_forbidden(status, &v, i64::from(R_CONCEDE), "concession");

    let row = purchase_receipt::Entity::find_by_id(R_CONCEDE)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::PENDING.to_string(),
        "越权让步严禁翻动他人检验态"
    );
    assert_eq!(row.concession_by, None, "越权让步严禁写入他人操作人");
    assert_eq!(row.concession_reason, None, "越权让步严禁写入他人理由");
}

#[tokio::test]
async fn rejudge_receipt_owner_transitions_to_passed_and_counts() {
    let (app, db) = seeded_app(self_auth(U_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/purchase/receipts/{R_REJUDGE}/rejudge"),
        Some(json!({ "inspection_result": "pass", "reason": "复检合格" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人复检改判应 2xx: {v}");

    let row = purchase_receipt::Entity::find_by_id(R_REJUDGE)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::PASSED.to_string(),
        "本人改判 pass 检验态应如实流转到 PASSED"
    );
    assert_eq!(row.rejudge_count, 1, "改判累计次数应如实加一");
    assert_eq!(row.rejudge_by, Some(U_A), "改判操作人应如实落会话用户");
}

#[tokio::test]
async fn rejudge_receipt_cross_owner_denied_with_zero_drift() {
    let (app, db) = seeded_app(self_auth(U_B)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/purchase/receipts/{R_REJUDGE}/rejudge"),
        Some(json!({ "inspection_result": "pass", "reason": "越权改判" })),
    )
    .await;
    assert_forbidden(status, &v, i64::from(R_REJUDGE), "rejudge");

    let row = purchase_receipt::Entity::find_by_id(R_REJUDGE)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::CONCESSION_ACCEPTED.to_string(),
        "越权改判严禁翻动他人检验态"
    );
    assert_eq!(row.rejudge_count, 0, "越权改判严禁累加他人改判次数");
    assert_eq!(row.rejudge_by, None, "越权改判严禁写入他人操作人");
}

// ==================================================================
// 组二 · 大货处方 / 加料处方 approve（门 handler 先 get_by_id(Some(ctx))，经 created_by 成员集合）
// ==================================================================

#[tokio::test]
async fn recipe_approve_owner_transitions_and_lands_approver() {
    let (app, db) = seeded_app(self_auth(U_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/production-recipes/{RECIPE_A}/approve"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人审核大货处方应 2xx: {v}");

    let row = production_recipe::Entity::find_by_id(RECIPE_A)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status,
        recipe_status::APPROVED.to_string(),
        "本人审核处方状态应如实 draft→approved"
    );
    assert_eq!(row.approved_by, Some(U_A), "审核人应如实落会话用户");
}

#[tokio::test]
async fn recipe_approve_cross_owner_denied_stays_draft() {
    let (app, db) = seeded_app(self_auth(U_B)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/production-recipes/{RECIPE_A}/approve"),
        None,
    )
    .await;
    assert_forbidden(status, &v, i64::from(RECIPE_A), "recipe approve");

    let row = production_recipe::Entity::find_by_id(RECIPE_A)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status,
        recipe_status::DRAFT.to_string(),
        "越权审核严禁把他人处方翻成 approved"
    );
    assert_eq!(row.approved_by, None, "越权审核严禁写入他人 approved_by");
}

#[tokio::test]
async fn addition_approve_owner_transitions_and_lands_approver() {
    let (app, db) = seeded_app(self_auth(U_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/production-recipes/additions/{ADDITION_A}/approve"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人审核加料处方应 2xx: {v}");

    let row = production_recipe_addition::Entity::find_by_id(ADDITION_A)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status,
        addition_status::APPROVED.to_string(),
        "本人审核加料处方状态应如实 draft→approved"
    );
    assert_eq!(row.approved_by, Some(U_A), "加料审核人应如实落会话用户");
}

#[tokio::test]
async fn addition_approve_cross_owner_denied_stays_draft() {
    let (app, db) = seeded_app(self_auth(U_B)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/production-recipes/additions/{ADDITION_A}/approve"),
        None,
    )
    .await;
    assert_forbidden(status, &v, i64::from(ADDITION_A), "addition approve");

    let row = production_recipe_addition::Entity::find_by_id(ADDITION_A)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status,
        addition_status::DRAFT.to_string(),
        "越权审核严禁把他人加料处方翻成 approved"
    );
    assert_eq!(row.approved_by, None, "越权审核严禁写入他人 approved_by");
}

// ==================================================================
// 组三 · 客户转移审批经理审批（门在 service.get_pending_approval 内按 applicant_id 过成员集合）
// ==================================================================

#[tokio::test]
async fn transfer_manager_approve_owner_reject_transitions() {
    let (app, db) = seeded_app(self_auth(U_A)).await;
    // 走拒绝分支：普通审批 pending→rejected，不触发实际转移链（保持断言聚焦于归属门）
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/transfer-approvals/{APPROVAL_A}/manager-approve"),
        Some(json!({ "approval_id": APPROVAL_A, "comment": "证据不足，驳回", "approved": false })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "本人（申请人可见）经理审批应 2xx: {v}"
    );

    let row = bingxi_backend::models::customer_transfer_approval::Entity::find_by_id(APPROVAL_A)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.approval_status,
        bingxi_backend::models::customer_transfer_approval::STATUS_REJECTED,
        "放行后审批行状态应如实流转（驳回→rejected）"
    );
    assert_eq!(row.manager_approver_id, Some(U_A), "审批人应如实落会话用户");
}

#[tokio::test]
async fn transfer_manager_approve_cross_owner_denied_no_drift_no_audit() {
    let (app, db) = seeded_app(self_auth(U_B)).await;
    let audit_before = audit_log::Entity::find().count(&*db).await.unwrap();

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/transfer-approvals/{APPROVAL_A}/manager-approve"),
        Some(json!({ "approval_id": APPROVAL_A, "comment": "越权驳回", "approved": false })),
    )
    .await;
    assert_forbidden(status, &v, i64::from(APPROVAL_A), "manager-approve");

    let row = bingxi_backend::models::customer_transfer_approval::Entity::find_by_id(APPROVAL_A)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.approval_status,
        bingxi_backend::models::customer_transfer_approval::STATUS_PENDING,
        "越权审批严禁流转他人审批单状态"
    );
    assert_eq!(row.manager_approver_id, None, "越权审批严禁写入他人审批人");
    assert_eq!(row.manager_comment, None, "越权审批严禁写入他人审批意见");

    let audit_after = audit_log::Entity::find().count(&*db).await.unwrap();
    assert_eq!(
        audit_after, audit_before,
        "越权审批路径严禁产生任何审计行（门在任何落库点之前）"
    );
}

// ==================================================================
// 组四 · 8D 报告推进 D 阶段（归属经两级父链 quality_issue→custom_order.created_by 继承）
// ==================================================================

#[tokio::test]
async fn eight_d_advance_owner_transitions_to_d1() {
    let (app, db) = seeded_app(self_auth(U_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/quality-8d-reports/{RPT}/advance"),
        Some(json!({ "step": "d1_team", "team_members": "张三/李四" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "本人（父定制订单归属人）推进 8D 应 2xx: {v}"
    );

    let row = quality_8d_report::Entity::find_by_id(RPT)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status, "d1_team",
        "本人推进后状态应如实 d0_plan→d1_team，实得: {}",
        row.status
    );
    assert_eq!(
        row.d1_team_members.as_deref(),
        Some("张三/李四"),
        "D1 团队成员应如实落库"
    );
}

#[tokio::test]
async fn eight_d_advance_cross_owner_denied_stays_d0() {
    let (app, db) = seeded_app(self_auth(U_B)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/quality-8d-reports/{RPT}/advance"),
        Some(json!({ "step": "d1_team", "team_members": "越权写入" })),
    )
    .await;
    assert_forbidden(status, &v, RPT, "8d advance");

    let row = quality_8d_report::Entity::find_by_id(RPT)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.status, "d0_plan",
        "越权推进严禁翻动他人 8D 报告的 D 阶段状态"
    );
    assert_eq!(
        row.d1_team_members, None,
        "越权推进严禁写入他人 D1 团队成员"
    );
}
