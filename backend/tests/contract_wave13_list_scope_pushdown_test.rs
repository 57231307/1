//! 功能：化验室打样通知单（`/api/v1/erp/lab-dip/requests`）、应收发票（`/api/v1/erp/ar/invoices`）、
//! 劳动合同（`/api/v1/erp/labor-contracts`）三张表列表侧行级数据范围（data_scope=self）下推的
//! 活体契约锁。钉死「同一张表里分属两个 self 范围用户的行，A 会话的列表响应只含 A 的行、绝不含
//! B 的行，且分页 total 与可见集同范围（随下推收窄，非全表计数）」。
//! 调用方：集成测试层（真已迁移 PostgreSQL，`TEST_DATABASE_URL`）。
//! 入参：经 `test_common::setup_test_db()` 连接并清空业务表后，为每张表播种 3 行——两行
//! `created_by=A`、一行 `created_by=B`；再以 data_scope=self、user_id 分别为 A / B 的两个
//! `AuthContext` 走真 HTTP 装配发起 GET 列表。
//! 传给谁：`lab_dip_handler::list_requests` → `lab_dip_ops/request.rs::list`（按
//! `lab_dip_request.created_by` 经 `apply_data_scope` 下推）；`ar_invoice_handler::list_ar_invoices`
//! → `ar_invoice_service::get_list`（按 `ar_invoice.created_by` 下推）；`labor_contract_handler::list`
//! → `labor_contract_service::list`（按 `labor_contract.created_by` 下推）。三表均无 department_id
//! 列，self 判据一律是 `created_by = 会话 user_id`。
//! 存什么：三表列表只按归属人列收窄行集与计数，不回显他人行；越权（B 读 A）不放大可见集。
//! 存哪里：`lab_dip_request` / `ar_invoices` / `labor_contracts` 表；回读真库证明「两主行物理都在
//! 同一张表内」，再以真 HTTP 响应断言可见集与 total 的收窄。
//!
//! 判据（逐表，A 与 B 各一组）：
//! - 本人会话：HTTP 200；返回列表 id 集合逐 id 等于「该会话 user_id 拥有的行集合」（含本人全部行、
//!   且不含他人行）；`total` 等于本人可见行数（A 为 2、B 为 1），绝不等全表 3。
//! - 真库回读：该表 COUNT(*) 恒为 3（两主行都物理存在），按 created_by 分组 A=2、B=1——证明收窄
//!   来自下推过滤，而非「他人行没种进去」的假绿。
//!
//! 反空操作自证（若 service 查询构造处漏按 created_by 下推 data_scope）：
//! - `*_list_self_A_returns_only_own_rows`：列表会返回全部 3 行、且含 B 行 id ⇒「id 集合 == 本人集合」
//!   与「不含他人行」两条同时判红；
//! - 同用例的 `total == 2` 判红（漏下推时 total 落到全表 3）；
//! - `*_list_self_B_returns_only_own_rows`：列表会含 A 的两行 ⇒「id 集合 == {B 行}」判红，`total == 1`
//!   判红（漏下推时为全表 3）。
//! 即：只要下推缺失，本文件六条列表断言中每张表的两条都会判红，锁不可能因「handler 补了 auth 但
//! service 查询未过滤」而空转通过。
//!
//! 夹具防法：`setup_test_db()` 自带 TRUNCATE 台账双向守卫，本文件不调 Migrator、不依赖台账内容；
//! 全程真 PostgreSQL + 真 HTTP，禁 sqlite / mock / 条件跳过 / `#[ignore]`。

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
use bingxi_backend::handlers::{ar_invoice_handler, lab_dip_handler, labor_contract_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{ar_invoice, customer, lab_dip_request, labor_contract};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

// 会话用户（self 范围）：列表侧行级归属判据即 created_by == 会话 user_id。
const OWNER_A: i32 = 9701;
const OWNER_B: i32 = 9702;

// 外键父体：应收发票 customer_id NOT NULL 且有 FK 到 customers，播种一个共享客户即可
// （customer_id 只是业务列，不参与归属门；归属门一律按 created_by）。
const CUS_SHARED: i32 = 9751;

// lab_dip_request 三行：A 两行、B 一行（created_by 决定可见性，is_deleted=false 才入列表）。
const LDR_A1: i32 = 9711;
const LDR_A2: i32 = 9712;
const LDR_B1: i32 = 9713;

// ar_invoices 三行：A 两行、B 一行。
const ARI_A1: i32 = 9721;
const ARI_A2: i32 = 9722;
const ARI_B1: i32 = 9723;

// labor_contracts 三行：A 两行、B 一行。
const LC_A1: i32 = 9731;
const LC_A2: i32 = 9732;
const LC_B1: i32 = 9733;

fn make_self_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("list_scope_fixture_{user_id}"),
        role_id: Some(2),
        department_id: None,
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

async fn get_list(app: &Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "列表响应非 JSON（{uri}）: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

fn ids_of(items: &[Value]) -> Vec<i64> {
    let mut ids: Vec<i64> = items.iter().filter_map(|it| it["id"].as_i64()).collect();
    ids.sort_unstable();
    ids
}

async fn seed(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();

    // 外键父体：共享客户（应收发票 customer_id 需命中 customers）。
    customer::ActiveModel {
        id: Set(CUS_SHARED),
        customer_code: Set(format!("CUS-LST-{CUS_SHARED}")),
        customer_name: Set(format!("列表下推测试客户{CUS_SHARED}")),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OWNER_A),
        department_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // lab_dip_request：三行均 is_deleted=false，归属由 created_by 决定。
    for (id, owner) in [(LDR_A1, OWNER_A), (LDR_A2, OWNER_A), (LDR_B1, OWNER_B)] {
        lab_dip_request::ActiveModel {
            id: Set(id),
            request_no: Set(format!("LD-LST-{id}")),
            light_source: Set("D65".to_string()),
            sample_versions: Set(4),
            required_date: Set(now.date_naive()),
            status: Set("pending".to_string()),
            is_deleted: Set(false),
            created_by: Set(Some(owner)),
            created_at: Set(now.fixed_offset()),
            updated_at: Set(now.fixed_offset()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // ar_invoices：三行 created_by 决定归属；customer_id 共享、不参与归属门。
    for (id, owner) in [(ARI_A1, OWNER_A), (ARI_A2, OWNER_A), (ARI_B1, OWNER_B)] {
        ar_invoice::ActiveModel {
            id: Set(id),
            invoice_no: Set(format!("AR-LST-{id}")),
            invoice_date: Set(now.date_naive()),
            due_date: Set(now.date_naive()),
            customer_id: Set(CUS_SHARED),
            invoice_amount: Set(Decimal::ONE),
            received_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(Decimal::ONE),
            status: Set("DRAFT".to_string()),
            approval_status: Set("PENDING".to_string()),
            created_by: Set(owner),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // labor_contracts：三行 created_by 决定归属；contract_no 唯一、worker_id NOT NULL 无 FK。
    for (id, owner) in [(LC_A1, OWNER_A), (LC_A2, OWNER_A), (LC_B1, OWNER_B)] {
        labor_contract::ActiveModel {
            id: Set(id),
            worker_id: Set(id),
            contract_no: Set(format!("LC-LST-{id}")),
            contract_type: Set("fixed_term".to_string()),
            start_date: Set(now.date_naive()),
            sign_date: Set(now.date_naive()),
            status: Set("active".to_string()),
            created_by: Set(Some(owner)),
            created_at: Set(now.fixed_offset()),
            updated_at: Set(now.fixed_offset()),
            ..Default::default()
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
        .route("/lab-dip/requests", get(lab_dip_handler::list_requests))
        .route("/ar/invoices", get(ar_invoice_handler::list_ar_invoices))
        .route("/labor-contracts", get(labor_contract_handler::list))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn seeded_app(auth: AuthContext) -> (Router, Arc<sea_orm::DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let app = build_app(auth, db.clone());
    (app, db)
}

// ============ lab_dip_request 列表 ============

#[tokio::test]
async fn ldr_list_self_a_returns_only_own_rows_and_narrowed_total() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;

    // 真库回读：全表物理存在 3 行，其中 created_by=A 恰 2 行 —— 证明"他人行确实种进了同一张表"，
    // 收窄只可能来自下推过滤，而非缺失播种数据。
    let full = lab_dip_request::Entity::find()
        .filter(lab_dip_request::Column::IsDeleted.eq(false))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(full, 3, "回读前置：三行物理都在 lab_dip_request 表内");
    let own = lab_dip_request::Entity::find()
        .filter(lab_dip_request::Column::CreatedBy.eq(OWNER_A))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(own, 2, "回读前置：A 名下恰两行");

    let (status, v) = get_list(&app, "/lab-dip/requests?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "A 会话打样通知单列表应 200: {v}");

    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        ids_of(&items),
        vec![LDR_A1 as i64, LDR_A2 as i64],
        "A 列表 id 集合应逐 id 恰等于本人两行，绝不含他人行"
    );
    assert!(
        !items
            .iter()
            .any(|it| it["id"].as_i64() == Some(LDR_B1 as i64)),
        "B 名下打样通知单行不得越权出现在 A 列表"
    );
    assert_eq!(
        v["data"]["total"].as_u64(),
        Some(2),
        "total 必须随下推收窄为 2（漏下推会落全表 3）"
    );
}

#[tokio::test]
async fn ldr_list_self_b_returns_only_own_rows_and_narrowed_total() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_B)).await;
    let (status, v) = get_list(&app, "/lab-dip/requests?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "B 会话打样通知单列表应 200: {v}");

    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        ids_of(&items),
        vec![LDR_B1 as i64],
        "B 列表 id 集合应逐 id 恰等于本人那一行，绝不含 A 的两行"
    );
    assert_eq!(
        v["data"]["total"].as_u64(),
        Some(1),
        "total 必须等于 B 可见行数 1（漏下推会落全表 3）"
    );
}

// ============ ar_invoices 列表 ============

#[tokio::test]
async fn ari_list_self_a_returns_only_own_rows_and_narrowed_total() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;

    let full = ar_invoice::Entity::find().count(&*db).await.unwrap();
    assert_eq!(full, 3, "回读前置：三行物理都在 ar_invoices 表内");
    let own = ar_invoice::Entity::find()
        .filter(ar_invoice::Column::CreatedBy.eq(OWNER_A))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(own, 2, "回读前置：A 名下恰两行");

    let (status, v) = get_list(&app, "/ar/invoices?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "A 会话应收发票列表应 200: {v}");

    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        ids_of(&items),
        vec![ARI_A1 as i64, ARI_A2 as i64],
        "A 列表 id 集合应逐 id 恰等于本人两行，绝不含他人行"
    );
    assert!(
        !items
            .iter()
            .any(|it| it["id"].as_i64() == Some(ARI_B1 as i64)),
        "B 名下应收发票行不得越权出现在 A 列表"
    );
    assert_eq!(
        v["data"]["total"].as_u64(),
        Some(2),
        "total 必须随下推收窄为 2（漏下推会落全表 3）"
    );
}

#[tokio::test]
async fn ari_list_self_b_returns_only_own_rows_and_narrowed_total() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_B)).await;
    let (status, v) = get_list(&app, "/ar/invoices?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "B 会话应收发票列表应 200: {v}");

    let items = v["data"]["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        ids_of(&items),
        vec![ARI_B1 as i64],
        "B 列表 id 集合应逐 id 恰等于本人那一行，绝不含 A 的两行"
    );
    assert_eq!(
        v["data"]["total"].as_u64(),
        Some(1),
        "total 必须等于 B 可见行数 1（漏下推会落全表 3）"
    );
}

// ============ labor_contracts 列表（出参形态为 data.list + data.total，非 items） ============

#[tokio::test]
async fn lc_list_self_a_returns_only_own_rows_and_narrowed_total() {
    let (app, db) = seeded_app(make_self_auth(OWNER_A)).await;

    let full = labor_contract::Entity::find().count(&*db).await.unwrap();
    assert_eq!(full, 3, "回读前置：三行物理都在 labor_contracts 表内");
    let own = labor_contract::Entity::find()
        .filter(labor_contract::Column::CreatedBy.eq(OWNER_A))
        .count(&*db)
        .await
        .unwrap();
    assert_eq!(own, 2, "回读前置：A 名下恰两行");

    let (status, v) = get_list(&app, "/labor-contracts?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "A 会话劳动合同列表应 200: {v}");

    let items = v["data"]["list"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        ids_of(&items),
        vec![LC_A1 as i64, LC_A2 as i64],
        "A 列表 id 集合应逐 id 恰等于本人两行，绝不含他人行"
    );
    assert!(
        !items
            .iter()
            .any(|it| it["id"].as_i64() == Some(LC_B1 as i64)),
        "B 名下劳动合同行不得越权出现在 A 列表"
    );
    assert_eq!(
        v["data"]["total"].as_u64(),
        Some(2),
        "total 必须随下推收窄为 2（漏下推会落全表 3）"
    );
}

#[tokio::test]
async fn lc_list_self_b_returns_only_own_rows_and_narrowed_total() {
    let (app, _db) = seeded_app(make_self_auth(OWNER_B)).await;
    let (status, v) = get_list(&app, "/labor-contracts?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "B 会话劳动合同列表应 200: {v}");

    let items = v["data"]["list"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        ids_of(&items),
        vec![LC_B1 as i64],
        "B 列表 id 集合应逐 id 恰等于本人那一行，绝不含 A 的两行"
    );
    assert_eq!(
        v["data"]["total"].as_u64(),
        Some(1),
        "total 必须等于 B 可见行数 1（漏下推会落全表 3）"
    );
}
