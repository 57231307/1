//! 契约波次 6 · 线索查重 detect-duplicates 行级数据权限（越权读）
//!
//! 根因（修复前实证）：`services/crm/lead.rs` 的 `detect_duplicate_leads` 直接
//! `crm_lead::Entity::find()` 按手机号/公司名全表匹配，不注入行级数据权限；
//! 出参 `DuplicateLeadGroup` 含 `lead_ids`/`lead_nos`/`company_names`。
//! 后果：任意登录用户用一个手机号即可枚举该号下**他人名下**线索的编号与公司名。
//!
//! 修复口径（全部为既有机制，未新增权限键、未改路由与 DTO）：
//! - service 签名加 `data_scope: Option<&DataScopeContext>`，两次查询套用与
//!   `list_leads` **同一个** `apply_department_scope_with_pool`
//!   （owner=OwnerId，dept=DepartmentId，公海 `lead_status='pool'` 放行同口径）；
//! - handler 与 `list_leads` 同法注入 `auth.to_data_scope_context()`；
//! - `match_key` 回显调用方自己提交的号码（非服务端查得的他人数据），保持不动；
//! - 过滤后不足 2 条不成组（既有 `leads.len() > 1` 判据不变）。
//!
//! 覆盖（sqlite 真跑 + 真 HTTP 装配，无硬编码 JSON 假断言）：
//! 1. B（self）用**他人**手机号查重复 → 组为空，响应体不含他人 lead_no（LD-A-*）；
//! 2. B 用**自己**手机号查重复 → 组只含 B 自己的两行（LD-B-001/LD-B-002）；
//! 3. B 用他人公司名（自己名下仅 1 行同名）查重复 → 可见行不足 2 条不成组，
//!    响应体不含他人 lead_no（同时锁"过滤后不成组"与"公司名分支也套 scope"）；
//! 4. admin（data_scope=all）用他人手机号查重复 → 可见全部，组含 LD-A-001/LD-A-002。

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
use bingxi_backend::handlers::crm_handler::detect_duplicate_leads;
use bingxi_backend::middleware::auth_context::AuthContext;
use sea_orm::ConnectionTrait;
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
        username: format!("wave6_dup_user_{user_id}"),
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
    db.execute_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Sqlite,
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

/// 种子（全部私海行，排除公海放行分支对断言的干扰）：
/// - id=1/2：A（owner=50）同手机号 13900000000 的两条重复，公司名均为「甲公司」；
/// - id=3/4：B（owner=60）同手机号 13800000000 的两条重复；
/// - id=5：B 名下第三条「甲公司」行（公司名分支：B 可见同名行仅 1 条 → 不成组）。
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
         (1,'LD-A-001','website','new','甲公司','张三A','13900000000','a1@example.com',
          50,'销售甲',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'LD-A-002','ad','new','甲公司','李四A','13900000000','a2@example.com',
          50,'销售甲',1,'2026-01-02T00:00:00Z','2026-01-02T00:00:00Z'),
         (3,'LD-B-001','referral','new','乙公司','王五B','13800000000','b1@example.com',
          60,'销售乙',1,'2026-01-03T00:00:00Z','2026-01-03T00:00:00Z'),
         (4,'LD-B-002','website','new','乙公司分部','赵六B','13800000000','b2@example.com',
          60,'销售乙',1,'2026-01-04T00:00:00Z','2026-01-04T00:00:00Z'),
         (5,'LD-B-003','ad','new','甲公司','钱七B','13777777777','b3@example.com',
          60,'销售乙',1,'2026-01-05T00:00:00Z','2026-01-05T00:00:00Z')",
    )
    .await;
    Arc::new(db)
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    let mut state = AppState::default();
    state.db = db.clone();
    Router::new()
        .route(
            "/erp/crm/leads/detect-duplicates",
            post(detect_duplicate_leads),
        )
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

// ---------------------------------------------------------------------------
// 1) self 用户 B 用他人手机号查重复 → 组为空且响应体不含他人 lead_no
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_cannot_enumerate_others_leads_via_their_mobile() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/detect-duplicates",
        json!({"mobile_phone": "13900000000"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "越权读应被过滤成空组而非报错: {v}");
    let groups = v["data"].as_array().expect("data 应为组数组");
    assert!(
        groups.is_empty(),
        "B 名下无 13900000000 的行，可见集过滤后不应成组: {v}"
    );
    let body_str = v.to_string();
    assert!(
        !body_str.contains("LD-A-001") && !body_str.contains("LD-A-002"),
        "响应体泄露他人线索编号（越权读回潮）: {body_str}"
    );
}

// ---------------------------------------------------------------------------
// 2) self 用户 B 用自己手机号查重复 → 组只含 B 自己的两行
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_detects_own_duplicate_group_only_own_rows() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/detect-duplicates",
        json!({"mobile_phone": "13800000000"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人查重应可用: {v}");
    let groups = v["data"].as_array().expect("data 应为组数组");
    assert_eq!(groups.len(), 1, "B 自己同号两行应成一组: {v}");
    let mut lead_nos: Vec<String> = groups[0]["lead_nos"]
        .as_array()
        .expect("lead_nos 应为数组")
        .iter()
        .map(|n| n.as_str().unwrap_or_default().to_string())
        .collect();
    lead_nos.sort();
    assert_eq!(
        lead_nos,
        vec!["LD-B-001".to_string(), "LD-B-002".to_string()],
        "组内只允许出现 B 自己的行: {v}"
    );
    assert_eq!(groups[0]["count"], json!(2));
    assert!(
        !v.to_string().contains("LD-A-"),
        "响应体出现他人编号: {}",
        v
    );
}

// ---------------------------------------------------------------------------
// 3) 公司名分支同样套 scope：B 查他人公司名（自己可见同名行仅 1 条）→ 不成组
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_company_name_query_filters_to_own_visible_rows() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/detect-duplicates",
        json!({"company_name": "甲公司"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "公司名查重应可用: {v}");
    let groups = v["data"].as_array().expect("data 应为组数组");
    assert!(
        groups.is_empty(),
        "B 可见同名行仅 1 条（不足 2 条不成组，判据不变）: {v}"
    );
    let body_str = v.to_string();
    assert!(
        !body_str.contains("LD-A-001") && !body_str.contains("LD-A-002"),
        "公司名分支响应体泄露他人线索编号（该分支漏套 scope 的回潮锁）: {body_str}"
    );
}

// ---------------------------------------------------------------------------
// 4) admin（data_scope=all）可见全部：他人同号两行照常成组
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_still_sees_all_duplicate_group() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_ADMIN, 1, "all"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/detect-duplicates",
        json!({"mobile_phone": "13900000000"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin 查重应成功: {v}");
    let groups = v["data"].as_array().expect("data 应为组数组");
    assert_eq!(groups.len(), 1, "admin 可见 A 的同号两行应成组: {v}");
    let body_str = v.to_string();
    assert!(
        body_str.contains("LD-A-001") && body_str.contains("LD-A-002"),
        "DataScope::All 的既有可见通道不得被收紧: {body_str}"
    );
}
