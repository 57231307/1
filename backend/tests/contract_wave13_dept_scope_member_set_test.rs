//! 功能：行级数据范围中 Dept 分支「按可见部门成员集合放行」这一条判据的活体契约锁，
//! 覆盖三条不同归属形态（无部门列按 created_by、经父资源继承、单行门 ⇔ 列表同源），
//! 钉死 Dept 岗不会被误收成了 self-only、也不会回退成「只看资源自身 department_id」。
//! 调用方：集成测试层（真已迁移 PostgreSQL，`TEST_DATABASE_URL`；本文件不调 Migrator，
//! 不依赖迁移台账内容）。
//! 入参：经 `test_common::setup_test_db()` 连接、TRUNCATE 业务表后播种分属三个用户
//! （A/B/C，其中 A、B 同属可见部门、C 属另一部门）的劳动合同 / 化验室打样通知单 /
//! 应收对账单（对账单归属经父客户继承），再以不同 data_scope 的 `AuthContext` 走真 HTTP 装配。
//! 传给谁：`labor_contract_handler::list`/`get_by_id`、`lab_dip_handler::list_requests`、
//! `ar_reconciliation_handler::list_reconciliations`。
//! 存什么：列表返回 id 集合与 `total` 必须精确等于「会话可见行集合」，且真库回读证明该表
//! 本就有全部三类行——收窄来自查询下推、不是没种数据。
//! 存哪里：`labor_contracts` / `lab_dip_request` / `customers` / `ar_reconciliations` 表。
//!
//! 判据（逐条，判红才算锁有效）：
//! - Dept 会话（成员集合含 B、不含 C）：三条通路的列表 id 集合都精确等于 {A 本人行 ∪ B 行}
//!   且不含 C 行，`total` 等于 items 长度；
//! - All 会话：同一请求必须看到全部行（钉 All 不被误收紧）；
//! - Self 会话：只看本人行；
//! - Dept 会话成员集合为空：必须退化成「仅本人」，B 行随之消失（钉空集合不放大可见面）；
//! - 合同单行门与列表同源：Dept 会话读可见成员 B 的行 ⇒ 2xx，读集合外 C 的行 ⇒ 403。
//!
//! 三条通路的判据来源不同、结论一致：劳动合同与化验室打样通知单无 `department_id` 列，
//! 列表与单行门均按 `created_by ∈ 可见部门成员集合` 下推；应收对账单无自身部门列、
//! 归属经父客户继承，列表按父客户 `owner_id=本人 OR department_id ∈ 可见部门集合` 下推。
//! 本锁用同一 Dept 会话（可见部门=DEPT_HOME 且成员集合=A,B）让三条通路得到完全相同的可见集，
//! 从而证明「同源」而非三处各写各的。
//!
//! 反空操作自证：若某条无部门列的 Dept 腿回退成「只看 department_id」，劳动合同与化验室打样
//! 会命中不存在的列而在真库报 5xx，其 2xx 断言必红；若回退成 self-only，则「列表 id 集合精确
//! 等于 {A∪B}」与「单行门读 B 行得 2xx」两条断言必红（B 行会从集合消失、B 行详情会从 2xx 变 403）。

#![allow(unused_imports, unused_variables)]

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
use bingxi_backend::handlers::{
    ar_reconciliation_handler, lab_dip_handler, labor_contract_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    ar_reconciliation, customer, department, lab_dip_request, labor_contract, user,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseBackend, EntityTrait, Statement,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// 私有大常量 ID 段，避开其它测试与迁移种子：用户 941x、可见部门参照 947x、
// 劳动合同 951x、打样通知单 990x、对账单 991x、客户 992x。
const USER_A: i32 = 9411;
const USER_B: i32 = 9412;
const USER_C: i32 = 9413;

// DEPT_HOME 为 A、B 的共同部门（Dept 会话可见），DEPT_OTHER 为 C 的部门（不可见）。
const DEPT_HOME: i32 = 9470;
const DEPT_OTHER: i32 = 9471;

// 劳动合同：无 department_id 列，归属按 created_by（A/B/C 各一行）。
const LC_A: i32 = 9511;
const LC_B: i32 = 9512;
const LC_C: i32 = 9513;

// 化验室打样通知单：无 department_id 列，归属按 created_by（A/B/C 各一行）。
const LDR_A: i32 = 9901;
const LDR_B: i32 = 9902;
const LDR_C: i32 = 9903;

// 客户（父资源，RLS 表，owner_id + department_id）：CUST_A→A、CUST_B→B（均在 DEPT_HOME），
// CUST_C→C（在 DEPT_OTHER）。
const CUST_A: i32 = 9921;
const CUST_B: i32 = 9922;
const CUST_C: i32 = 9923;

// 应收对账单：无自身部门列，归属经父客户继承（各自挂到一个客户，created_by 与父客户 owner 对齐）。
const AR_A: i32 = 9911;
const AR_B: i32 = 9912;
const AR_C: i32 = 9913;

fn make_auth(
    user_id: i32,
    scope: &str,
    dept_ids: Option<&str>,
    members: Option<&str>,
) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("dept_scope_fixture_{user_id}"),
        role_id: Some(2),
        department_id: Some(DEPT_HOME),
        data_scope: Some(scope.to_string()),
        dept_ids: dept_ids.map(|s| Arc::new(s.to_string())),
        dept_member_user_ids: members.map(|s| Arc::new(s.to_string())),
    }
}

fn self_auth(user_id: i32) -> AuthContext {
    make_auth(user_id, "self", None, None)
}

fn all_auth(user_id: i32) -> AuthContext {
    make_auth(user_id, "all", None, None)
}

// Dept 会话：可见部门集合含 DEPT_HOME（不含 DEPT_OTHER）、成员集合由入参决定。
// 成员集合显式带本人 A，与 `to_data_scope_context` 自动兜底 push 本人一致，不依赖兜底。
fn dept_auth(user_id: i32, members: &str) -> AuthContext {
    make_auth(user_id, "dept", Some(&DEPT_HOME.to_string()), Some(members))
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

async fn call(app: &Router, uri: &str) -> (StatusCode, Value) {
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
                "响应非 JSON（{uri}）: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

async fn seed(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();

    // 部门参照种子（必须在插用户前）：`departments` 在 SEALED_REFERENCE_TABLES 中不参与
    // TRUNCATE，迁移只播种 id 1~5，而本文件 DEPT_HOME=9470 / DEPT_OTHER=9471 被写入
    // users.department_id（fk_users_department），需自建。幂等：主键查已存在则跳过。
    for (dept_id, code, name) in [
        (DEPT_HOME, "D-DEPTSCOPE-9470", "部门范围测试主部门"),
        (DEPT_OTHER, "D-DEPTSCOPE-9471", "部门范围测试他部门"),
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

    // 三个用户：A、B 归 DEPT_HOME，C 归 DEPT_OTHER（父客户继承通路据此决定 C 不可见）。
    for (uid, dept) in [
        (USER_A, DEPT_HOME),
        (USER_B, DEPT_HOME),
        (USER_C, DEPT_OTHER),
    ] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("dept_scope_fixture_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(dept)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 劳动合同：created_by 决定归属，无部门列。
    for (id, owner) in [(LC_A, USER_A), (LC_B, USER_B), (LC_C, USER_C)] {
        labor_contract::ActiveModel {
            id: Set(id),
            worker_id: Set(id),
            contract_no: Set(format!("LC-DEPT-{id}")),
            contract_type: Set("fixed_term".to_string()),
            start_date: Set(now.date_naive()),
            end_date: Set(Some(now.date_naive() + chrono::Duration::days(365))),
            probation_end_date: Set(None),
            probation_salary: Set(Decimal::new(8000, 0)),
            regular_salary: Set(Decimal::new(10000, 0)),
            working_hours_system: Set("standard".to_string()),
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

    // 化验室打样通知单：created_by 决定归属，无部门列；列表过滤 is_deleted=false，须种未删行。
    for (id, owner) in [(LDR_A, USER_A), (LDR_B, USER_B), (LDR_C, USER_C)] {
        lab_dip_request::ActiveModel {
            id: Set(id),
            request_no: Set(format!("LD-DEPT-{id}")),
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

    // 父客户：owner_id + department_id（CUST_A/B→DEPT_HOME，CUST_C→DEPT_OTHER）。
    for (cid, owner, dept) in [
        (CUST_A, USER_A, DEPT_HOME),
        (CUST_B, USER_B, DEPT_HOME),
        (CUST_C, USER_C, DEPT_OTHER),
    ] {
        customer::ActiveModel {
            id: Set(cid),
            customer_code: Set(format!("CUS-DEPT-{cid}")),
            customer_name: Set(format!("部门范围测试客户{cid}")),
            credit_limit: Set(Decimal::ZERO),
            payment_terms: Set(30),
            status: Set("active".to_string()),
            customer_type: Set("retail".to_string()),
            owner_id: Set(owner),
            department_id: Set(Some(dept)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 应收对账单：无自身部门列，归属经父客户继承。
    for (id, cid, owner) in [
        (AR_A, CUST_A, USER_A),
        (AR_B, CUST_B, USER_B),
        (AR_C, CUST_C, USER_C),
    ] {
        ar_reconciliation::ActiveModel {
            id: Set(id),
            reconciliation_no: Set(format!("RC-DEPT-{id}")),
            reconciliation_date: Set(now.date_naive()),
            period_start: Set(now.date_naive()),
            period_end: Set(now.date_naive()),
            customer_id: Set(cid),
            customer_name: Set(Some(format!("部门范围测试客户{cid}"))),
            opening_balance: Set(Decimal::ONE),
            total_invoices: Set(Decimal::ONE),
            total_collections: Set(Decimal::ZERO),
            closing_balance: Set(Decimal::ONE),
            reconciliation_status: Set(Some("draft".to_string())),
            created_by: Set(Some(owner)),
            created_at: Set(now),
            updated_at: Set(now),
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
        .route("/erp/labor-contracts", get(labor_contract_handler::list))
        .route(
            "/erp/labor-contracts/{id}",
            get(labor_contract_handler::get_by_id),
        )
        .route("/erp/lab-dip/requests", get(lab_dip_handler::list_requests))
        .route(
            "/erp/ar-reconciliations",
            get(ar_reconciliation_handler::list_reconciliations),
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

// 真库直读目标表的全部行 id（不带任何 data_scope），用来证明「收窄是下推造成的、不是没种数据」。
// id 列在 PG 是 int4，显式 ::bigint 后按 i64 读，避免类型不匹配。
async fn all_row_ids(db: &sea_orm::DatabaseConnection, sql: &str) -> Vec<i64> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    );
    db.query_all_raw(stmt)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get::<i64>("", "id").unwrap())
        .collect()
}

// 从 `data.list`（劳动合同族）或 `data.items`（分页族）提取 id 集合，升序返回。
fn extract_ids(v: &Value, key: &str) -> Vec<i64> {
    let mut ids: Vec<i64> = v["data"][key]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|it| it["id"].as_i64())
        .collect();
    ids.sort_unstable();
    ids
}

fn total_of(v: &Value) -> u64 {
    v["data"]["total"].as_u64().unwrap_or(u64::MAX)
}

// ============ 劳动合同（无部门列，按 created_by 走成员集合） ============

#[tokio::test]
async fn labor_dept_member_set_sees_a_and_b_not_c() {
    let (app, db) = seeded_app(dept_auth(USER_A, &format!("{USER_A},{USER_B}"))).await;
    let whole = all_row_ids(
        &db,
        "SELECT id::bigint AS id FROM labor_contracts ORDER BY id",
    )
    .await;
    assert_eq!(
        whole.len(),
        3,
        "labor_contracts 应种有 A/B/C 三行后再验下推: {whole:?}"
    );

    let (status, v) = call(&app, "/erp/labor-contracts?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "本人 Dept 会话列表应 2xx: {v}");

    let ids = extract_ids(&v, "list");
    assert_eq!(
        ids,
        vec![LC_A as i64, LC_B as i64],
        "Dept 成员集合含 B 不含 C ⇒ 列表 id 集合精确等于 {{A∪B}}: {ids:?}"
    );
    assert!(
        !ids.contains(&(LC_C as i64)),
        "集合外 C 行绝不可见: {ids:?}"
    );
    assert_eq!(
        total_of(&v),
        ids.len() as u64,
        "total 必须与 items 同源（下推收窄，非后置过滤）"
    );
    assert_eq!(total_of(&v), 2, "可见集应恰为两行");
}

#[tokio::test]
async fn labor_all_scope_sees_every_row() {
    let (app, db) = seeded_app(all_auth(USER_A)).await;
    let whole = all_row_ids(
        &db,
        "SELECT id::bigint AS id FROM labor_contracts ORDER BY id",
    )
    .await;
    assert_eq!(whole.len(), 3, "labor_contracts 应种有全部三行");

    let (status, v) = call(&app, "/erp/labor-contracts?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "All 会话列表应 2xx: {v}");
    let ids = extract_ids(&v, "list");
    assert_eq!(
        ids,
        vec![LC_A as i64, LC_B as i64, LC_C as i64],
        "All 会话必须看到全部行（含 C），被误收成成员集合即此断言红: {ids:?}"
    );
    assert_eq!(total_of(&v), 3, "All 会话 total 应为全表行数");
}

#[tokio::test]
async fn labor_self_scope_sees_only_own_row() {
    let (app, _db) = seeded_app(self_auth(USER_A)).await;
    let (status, v) = call(&app, "/erp/labor-contracts?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "Self 会话列表应 2xx: {v}");
    let ids = extract_ids(&v, "list");
    assert_eq!(ids, vec![LC_A as i64], "Self 会话只看本人行: {ids:?}");
}

#[tokio::test]
async fn labor_dept_empty_member_set_degrades_to_self_only() {
    // 空成员集合：`to_data_scope_context` 兜底 push 本人 ⇒ 有效集 {A} ⇒ 仅本人可见。
    // 钉「空集合不放大可见面」：一旦放大，B 行会重新出现、此断言即红。
    let (app, _db) = seeded_app(dept_auth(USER_A, "")).await;
    let (status, v) = call(&app, "/erp/labor-contracts?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "空成员 Dept 会话列表应 2xx: {v}");
    let ids = extract_ids(&v, "list");
    assert_eq!(
        ids,
        vec![LC_A as i64],
        "Dept 成员集合为空必须退化为仅本人（B 行不得出现）: {ids:?}"
    );
}

// ============ 化验室打样通知单（第二条无部门列通路，证明与劳动合同同源） ============

#[tokio::test]
async fn labdip_dept_member_set_sees_a_and_b_not_c() {
    let (app, db) = seeded_app(dept_auth(USER_A, &format!("{USER_A},{USER_B}"))).await;
    let whole = all_row_ids(
        &db,
        "SELECT id::bigint AS id FROM lab_dip_request ORDER BY id",
    )
    .await;
    assert_eq!(
        whole.len(),
        3,
        "lab_dip_request 应种有 A/B/C 三行: {whole:?}"
    );

    let (status, v) = call(&app, "/erp/lab-dip/requests?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "本人 Dept 会话打样列表应 2xx: {v}");
    let ids = extract_ids(&v, "items");
    assert_eq!(
        ids,
        vec![LDR_A as i64, LDR_B as i64],
        "打样列表与劳动合同同源：成员集合含 B 不含 C ⇒ {{A∪B}}: {ids:?}"
    );
    assert!(
        !ids.contains(&(LDR_C as i64)),
        "集合外 C 打样行不可见: {ids:?}"
    );
    assert_eq!(total_of(&v), ids.len() as u64, "打样 total 与 items 同源");
    assert_eq!(total_of(&v), 2, "打样可见集应恰为两行");
}

#[tokio::test]
async fn labdip_all_scope_sees_every_row() {
    let (app, db) = seeded_app(all_auth(USER_A)).await;
    let whole = all_row_ids(
        &db,
        "SELECT id::bigint AS id FROM lab_dip_request ORDER BY id",
    )
    .await;
    assert_eq!(whole.len(), 3, "lab_dip_request 应种有全部三行");
    let (status, v) = call(&app, "/erp/lab-dip/requests?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "All 会话打样列表应 2xx: {v}");
    let ids = extract_ids(&v, "items");
    assert_eq!(
        ids,
        vec![LDR_A as i64, LDR_B as i64, LDR_C as i64],
        "All 会话打样必须看到全部行（含 C）: {ids:?}"
    );
}

#[tokio::test]
async fn labdip_self_scope_sees_only_own_row() {
    let (app, _db) = seeded_app(self_auth(USER_A)).await;
    let (status, v) = call(&app, "/erp/lab-dip/requests?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "Self 会话打样列表应 2xx: {v}");
    let ids = extract_ids(&v, "items");
    assert_eq!(ids, vec![LDR_A as i64], "Self 只看本人打样行: {ids:?}");
}

#[tokio::test]
async fn labdip_dept_empty_member_set_degrades_to_self_only() {
    let (app, _db) = seeded_app(dept_auth(USER_A, "")).await;
    let (status, v) = call(&app, "/erp/lab-dip/requests?page_size=100").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "空成员 Dept 会话打样列表应 2xx: {v}"
    );
    let ids = extract_ids(&v, "items");
    assert_eq!(
        ids,
        vec![LDR_A as i64],
        "打样通路空成员集合必须退化为仅本人: {ids:?}"
    );
}

// ============ 应收对账单（经父客户继承归属的第三条通路） ============

#[tokio::test]
async fn ar_dept_scope_parent_home_dept_sees_a_and_b_not_c() {
    // 同一 Dept 会话：可见部门={DEPT_HOME}、成员集合={A,B}。对账单按父客户 RLS 列下推
    // （owner_id=本人 OR department_id ∈ 可见部门集合），结论与两条 created_by 通路一致。
    let (app, db) = seeded_app(dept_auth(USER_A, &format!("{USER_A},{USER_B}"))).await;
    let whole = all_row_ids(
        &db,
        "SELECT id::bigint AS id FROM ar_reconciliations ORDER BY id",
    )
    .await;
    assert_eq!(
        whole.len(),
        3,
        "ar_reconciliations 应种有 A/B/C 三行: {whole:?}"
    );

    let (status, v) = call(&app, "/erp/ar-reconciliations?page_size=100").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "本人 Dept 会话对账单列表应 2xx: {v}"
    );
    let ids = extract_ids(&v, "items");
    assert_eq!(
        ids,
        vec![AR_A as i64, AR_B as i64],
        "经父客户继承的 Dept 通路必须与另两条同源 ⇒ {{A∪B}}不含C: {ids:?}"
    );
    assert!(
        !ids.contains(&(AR_C as i64)),
        "父客户在他部门的 C 行不可见: {ids:?}"
    );
    assert_eq!(total_of(&v), ids.len() as u64, "对账单 total 与 items 同源");
}

#[tokio::test]
async fn ar_all_scope_sees_every_row() {
    let (app, db) = seeded_app(all_auth(USER_A)).await;
    let whole = all_row_ids(
        &db,
        "SELECT id::bigint AS id FROM ar_reconciliations ORDER BY id",
    )
    .await;
    assert_eq!(whole.len(), 3, "ar_reconciliations 应种有全部三行");
    let (status, v) = call(&app, "/erp/ar-reconciliations?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "All 会话对账单列表应 2xx: {v}");
    let ids = extract_ids(&v, "items");
    assert_eq!(
        ids,
        vec![AR_A as i64, AR_B as i64, AR_C as i64],
        "All 会话对账单必须看到全部行（含 C）: {ids:?}"
    );
}

#[tokio::test]
async fn ar_self_scope_sees_only_own_row() {
    let (app, _db) = seeded_app(self_auth(USER_A)).await;
    let (status, v) = call(&app, "/erp/ar-reconciliations?page_size=100").await;
    assert_eq!(status, StatusCode::OK, "Self 会话对账单列表应 2xx: {v}");
    let ids = extract_ids(&v, "items");
    assert_eq!(ids, vec![AR_A as i64], "Self 只看本人对账单: {ids:?}");
}

// ============ 单行门 ⇔ 列表同源（劳动合同详情：B 行 2xx、C 行 403） ============

#[tokio::test]
async fn labor_detail_member_row_is_readable() {
    // Dept 成员集合含 B ⇒ 列表可见 B 行，详情也必须 2xx，钉「列表 ⇔ 详情」不矛盾。
    let (app, _db) = seeded_app(dept_auth(USER_A, &format!("{USER_A},{USER_B}"))).await;
    let (status, v) = call(&app, &format!("/erp/labor-contracts/{LC_B}")).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "Dept 会话读可见成员 B 的劳动合同详情必须 2xx（与列表同源）: {v}"
    );
    assert_eq!(v["data"]["id"].as_i64(), Some(LC_B as i64));
    assert_eq!(v["data"]["created_by"].as_i64(), Some(USER_B as i64));
}

#[tokio::test]
async fn labor_detail_outsider_row_is_forbidden() {
    // 成员集合不含 C ⇒ C 行既不在列表也不该能读详情，钉 403（非降级成 null/2xx 假绿）。
    let (app, _db) = seeded_app(dept_auth(USER_A, &format!("{USER_A},{USER_B}"))).await;
    let (status, v) = call(&app, &format!("/erp/labor-contracts/{LC_C}")).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Dept 会话读集合外 C 的劳动合同详情必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN", "越权必须回 FORBIDDEN 错误码: {v}");
}

#[tokio::test]
async fn labor_detail_empty_member_set_blocks_peer_row() {
    // 空成员集合（退化为仅本人）⇒ 读他人 B 行必须 403，钉「空集合不放大」在单行门侧同样成立。
    let (app, _db) = seeded_app(dept_auth(USER_A, "")).await;
    let (status, v) = call(&app, &format!("/erp/labor-contracts/{LC_B}")).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "空成员 Dept 会话读他人 B 行必须 403（退化仅本人）: {v}"
    );
    // 本人行仍可读，证明「退化」是把可见面收到本人而非判死全部。
    let (own_status, own_v) = call(&app, &format!("/erp/labor-contracts/{LC_A}")).await;
    assert_eq!(
        own_status,
        StatusCode::OK,
        "空成员 Dept 会话仍应能读本人自己的行: {own_v}"
    );
}
