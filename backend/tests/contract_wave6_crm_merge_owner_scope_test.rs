//! 契约波次 6 · 线索合并 /leads/merge 行级归属校验（越权写，不可逆）
//!
//! 根因（修复前实证）：`services/crm/lead.rs` 的 `merge_leads` 用
//! `find_by_id(master_lead_id)` 与循环内 `find_by_id(*dup_id)` 取行，
//! 不做任何归属/数据范围校验 → 任何用户可把自己看不到的他人线索标记为 lost
//! 并写入合并原因（不可逆的越权写）。
//!
//! 修复口径（全部为既有机制，未新增权限键、未改路由与 DTO）：
//! - service 签名加 `data_scope: Option<&DataScopeContext>`；
//! - master 与**每一条** duplicate（存在行）都必须落在可见集内，判定与
//!   `get_lead` 的 IDOR 防护同一来源（`check_resource_owner`：
//!   All=通过 / Self_=归属人本人 / Dept=部门集合，与 list_leads 同口径）；
//! - 任一行不可见即整笔拒绝（预校验先判后写，拒绝时零漂移），
//!   禁止"跳过不可见行继续合并其余"的静默降级；
//! - 出参 403 走 AppError::permission_denied 固定脱敏文案 + `FORBIDDEN` 码，
//!   真实原因只进日志；本文件只断 status 与信封 code，不断案文案原文。
//!
//! 覆盖（sqlite 真跑 + 真 HTTP 装配；行状态一律 `crm_lead::Entity::find_by_id`
//! 回读真库比对，证明"零漂移"）：
//! 1. B（self）合并 A 名下两条 → 403 + code=FORBIDDEN，两行 lead_status/lost_reason 零漂移；
//! 2. B 合并自己名下两条重复 → 200 且真生效（dup→lost、master 不动）；
//! 3. B 混合提交（自有 master + 自有 dup + 他人 dup）→ 403 整笔拒绝，
//!    自有 dup 也**不得**被先行合并（静默降级回潮锁）；
//! 4. admin（data_scope=all）合并 A 名下两条 → 200（既有越界通道未收紧）。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler::merge_leads;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::crm_lead;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 脚手架（与 contract_wave6_crm_pool_owner_test.rs 同款）
// ---------------------------------------------------------------------------

const USER_B: i32 = 60;
const USER_ADMIN: i32 = 70;

fn make_auth(user_id: i32, role_id: i32, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave6_merge_user_{user_id}"),
        role_id: Some(role_id),
        department_id: Some(1),
        data_scope: Some(data_scope.to_string()),
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

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL/种子 执行失败: {e}\nSQL: {sql}"));
}

/// 与 models/crm_lead.rs::Model 逐列对应（表 crm_lead）
const CREATE_CRM_LEAD: &str = r#"CREATE TABLE crm_lead (
    id INTEGER PRIMARY KEY,
    lead_no TEXT NOT NULL UNIQUE, lead_source TEXT NOT NULL,
    lead_status TEXT, company_name TEXT,
    contact_name TEXT NOT NULL, contact_title TEXT,
    mobile_phone TEXT, tel_phone TEXT, email TEXT, wechat TEXT, qq TEXT,
    address TEXT, product_interest TEXT,
    estimated_quantity TEXT, estimated_amount TEXT,
    expected_delivery_date TEXT, requirement_desc TEXT,
    owner_id INTEGER NOT NULL, department_id INTEGER, owner_name TEXT NOT NULL,
    last_follow_up_date TEXT, next_follow_up_date TEXT, follow_up_plan TEXT,
    converted_at TEXT, converted_customer_id INTEGER, converted_opportunity_id INTEGER,
    lost_reason TEXT, priority TEXT, rating INTEGER, tags TEXT, industry TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    created_by INTEGER, updated_by INTEGER, custom_fields TEXT
)"#;

/// 审计落库（handler record_async 写 audit_logs，缺表即 500 假红）
const CREATE_AUDIT_LOGS: &str = r#"CREATE TABLE audit_logs (
    id INTEGER PRIMARY KEY,
    user_id INTEGER, username TEXT, action TEXT NOT NULL,
    resource_type TEXT, resource_id TEXT, resource_name TEXT, description TEXT,
    ip_address TEXT, user_agent TEXT, request_method TEXT, request_path TEXT,
    request_body TEXT, response_status INTEGER, duration_ms INTEGER,
    old_value TEXT, new_value TEXT, created_at TEXT,
    operation_type TEXT, severity TEXT, request_id TEXT,
    before_snapshot TEXT, after_snapshot TEXT, condition TEXT,
    export_record_count INTEGER, export_query_filter TEXT, export_file_format TEXT,
    export_approval_token TEXT, export_watermark_user TEXT
)"#;

/// 种子（两条独立重复对，全部私海 new 行）：
/// - id=1/2：A（owner=50）名下重复对（越权目标）；
/// - id=3/4：B（owner=60）名下重复对（合法合并目标）。
async fn seeded_db() -> Arc<sea_orm::DatabaseConnection> {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    exec(&db, CREATE_CRM_LEAD).await;
    exec(&db, CREATE_AUDIT_LOGS).await;
    exec(
        &db,
        "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
         contact_name,mobile_phone,email,owner_id,owner_name,department_id,
         created_at,updated_at) VALUES
         (1,'LD-A-001','website','new','甲公司','张三A','13900000001','a1@example.com',
          50,'销售甲',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'LD-A-002','ad','new','甲公司','李四A','13900000002','a2@example.com',
          50,'销售甲',1,'2026-01-02T00:00:00Z','2026-01-02T00:00:00Z'),
         (3,'LD-B-001','referral','new','乙公司','王五B','13800000001','b1@example.com',
          60,'销售乙',1,'2026-01-03T00:00:00Z','2026-01-03T00:00:00Z'),
         (4,'LD-B-002','website','new','乙公司分部','赵六B','13800000002','b2@example.com',
          60,'销售乙',1,'2026-01-04T00:00:00Z','2026-01-04T00:00:00Z')",
    )
    .await;
    Arc::new(db)
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    let mut state = AppState::default();
    state.db = db.clone();
    Router::new()
        .route("/erp/crm/leads/merge", post(merge_leads))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn post_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
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

/// 回读真库的 (lead_status, lost_reason)——"零漂移/真生效"唯一可信证据
async fn row_state(db: &sea_orm::DatabaseConnection, id: i32) -> (String, Option<String>) {
    let lead = crm_lead::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("线索 {id} 不应被删除"));
    (lead.lead_status.unwrap_or_default(), lead.lost_reason)
}

// ---------------------------------------------------------------------------
// 1) self 用户合并他人名下两条 → 403 + FORBIDDEN，两行零漂移
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_merging_others_leads_is_403_and_rows_untouched() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));
    let before1 = row_state(&db, 1).await;
    let before2 = row_state(&db, 2).await;
    assert_eq!(before1.0, "new");
    assert_eq!(before2.0, "new");

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 1, "duplicate_ids": [2]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "非归属人合并他人线索必须整笔 403（修复前为 200，不可逆越权写）: 信封={}",
        v["code"]
    );
    assert_eq!(v["code"], "FORBIDDEN", "失败信封 code 契约: {}", v["code"]);

    assert_eq!(row_state(&db, 1).await, before1, "master 行必须零漂移");
    assert_eq!(row_state(&db, 2).await, before2, "dup 行必须零漂移");
}

// ---------------------------------------------------------------------------
// 2) self 用户合并自己名下两条重复 → 200 且真生效
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_merging_own_duplicates_succeeds() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 3, "duplicate_ids": [4]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人合并应成功: {v}");
    assert_eq!(v["data"]["merged_count"], json!(1));

    let master = row_state(&db, 3).await;
    assert_eq!(master.0, "new", "master 应保持原状态");
    let dup = row_state(&db, 4).await;
    assert_eq!(dup.0, "lost", "dup 应真落库为 lost");
    assert!(
        dup.1.is_some(),
        "dup 应记录合并原因（lost_reason 非空，不断文案原文）"
    );
}

// ---------------------------------------------------------------------------
// 3) 混合提交（自有 master + 自有 dup + 他人 dup）→ 403 整笔拒绝；
//    自有 dup 也不得被先行合并——"跳过不可见行继续合并其余"的静默降级回潮锁
// ---------------------------------------------------------------------------

#[tokio::test]
async fn partial_invisible_batch_rejects_entire_merge_without_silent_skip() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));
    let before4 = row_state(&db, 4).await;
    let before2 = row_state(&db, 2).await;

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 3, "duplicate_ids": [4, 2]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "批次内任一行不可见必须整笔拒绝而非部分合并: 信封={}",
        v["code"]
    );
    assert_eq!(v["code"], "FORBIDDEN");

    assert_eq!(
        row_state(&db, 4).await,
        before4,
        "静默降级回潮：他人 dup 不可见时，B 自己的 dup 行被先行合并（部分提交）"
    );
    assert_eq!(row_state(&db, 2).await, before2, "他人 dup 行必须零漂移");
}

// ---------------------------------------------------------------------------
// 4) admin（data_scope=all）合并他人名下两条 → 200（既有越界通道未收紧）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_can_merge_others_leads() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_ADMIN, 1, "all"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 1, "duplicate_ids": [2]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "DataScope::All（admin）的既有越界通道不得被收紧: {v}"
    );
    assert_eq!(row_state(&db, 2).await.0, "lost");
}
