//! 契约波次 6 · 任务 #200：CRM 线索默认字段脱敏读错键防回潮锁
//!
//! 根因（修复前实证）：crm_handler.rs 的 P1-08-5「默认脱敏」块（无数据权限行 +
//! 非 admin 分支）读写的键是 `contact_phone`，但 crm_lead 出参由
//! `serde_json::to_value(crm_lead::Model)` 生成，手机号真实列名是
//! `mobile_phone`（models/crm_lead.rs:40），`contact_phone` 键根本不存在
//! （只存在于 customer / customer_address / sales_order / supplier 模型）——
//! 手机号脱敏恒不生效，非 admin 拿到完全未打码的手机号；同块的 email/address 是生效的。
//!
//! 覆盖（全部 sqlite 真跑 + 真 HTTP 装配，无伪装断言）：
//! 1. 无数据权限行的非 admin（role_id=2）→ 列表与详情的 `mobile_phone` 为
//!    mask_phone 形态（前 3 后 4，不含原文中段数字）、`email` 掩码、`address` 移除；
//! 2. 有数据权限行（hidden_fields）→ hidden_fields 生效、不走默认打码（既有语义不回归）；
//! 3. admin（role_id=1，roles.code='admin'）→ get_role_data_permission 返回
//!    Ok(Some(ALL, None, None))，filter_fields_batch 为空操作且默认打码分支不可达
//!    （role_id != 1 为假）→ 出参保持原值（这是既有契约，依据即该两行代码路径）；
//! 4. 源码扫描锁：crm_handler.rs 中不得再出现 `get("contact_phone")`（回潮即红）。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler::{get_lead, list_leads};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 脚手架
// ---------------------------------------------------------------------------

const RAW_PHONE: &str = "13812348888";
const RAW_EMAIL: &str = "alice@example.com";
const RAW_ADDRESS: &str = "河北省邢台市某某路 1 号";

fn make_auth(user_id: i32, role_id: i32, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave6_user_{user_id}"),
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

/// 与 models/role.rs::Model 对应（is_admin_role 判定源，admin_checker.rs:86）
const CREATE_ROLES: &str = r#"CREATE TABLE roles (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, code TEXT NOT NULL,
    description TEXT, permissions TEXT, is_system INTEGER NOT NULL,
    data_scope TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

/// 与 models/data_permission.rs::Model 对应（表 data_permissions）
const CREATE_DATA_PERMISSIONS: &str = r#"CREATE TABLE data_permissions (
    id INTEGER PRIMARY KEY, role_id INTEGER NOT NULL,
    resource_type TEXT NOT NULL, scope_type TEXT NOT NULL,
    custom_condition TEXT, allowed_fields TEXT, hidden_fields TEXT,
    is_enabled INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

/// 建表 + 种子：
/// - roles：1=admin（契约 3 的判定源）、2=sales（非 admin）——每个用例同构种子，
///   保证 admin_checker 全局缓存（ADMIN_ROLE_CACHE，按 role_id 缓存）在任何执行顺序下取值一致；
/// - crm_lead：一条 owner_id=50 的线索，携带原始手机号/邮箱/地址。
async fn seeded_state(hidden_fields: Option<&[&str]>) -> AppState {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    exec(&db, CREATE_CRM_LEAD).await;
    exec(&db, CREATE_ROLES).await;
    exec(&db, CREATE_DATA_PERMISSIONS).await;

    exec(
        &db,
        "INSERT INTO roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES
         (1,'系统管理员','admin',1,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'销售专员','sales',0,'self','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;

    exec(
        &db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,contact_name,
             mobile_phone,email,address,owner_id,owner_name,created_at,updated_at)
             VALUES (1,'LD001','website','new','张三','{RAW_PHONE}','{RAW_EMAIL}',
             '{RAW_ADDRESS}',50,'销售甲','2026-09-01T00:00:00Z','2026-09-01T00:00:00Z')"
        ),
    )
    .await;

    if let Some(hidden) = hidden_fields {
        let hidden_json = json!(hidden.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        exec(
            &db,
            &format!(
                "INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
                 allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
                 VALUES (1,2,'crm_lead','SELF',NULL,'{hidden_json}',1,
                 '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
            ),
        )
        .await;
    }

    let mut state = AppState::default();
    state.db = Arc::new(db);
    // data_permission_service 内部持有独立 db 引用，必须与 state.db 同源重建，
    // 否则 get_role_data_permission 落在 Disconnected 连接上恒 Err（假绿）。
    state.data_permission_service = Arc::new(DataPermissionService::new(state.db.clone()));
    state
}

fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/leads", get(list_leads))
        .route("/erp/crm/leads/{id}", get(get_lead))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
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

/// 信封锚点：ApiResponse::success → {code,data}；list_leads 的 data 内层再嵌
/// {data:[...],total,page,page_size}（services/crm/lead.rs:163）。
fn list_items(v: &Value) -> &Vec<Value> {
    v["data"]["data"]
        .as_array()
        .unwrap_or_else(|| panic!("分页列表形状漂移（期望 data.data 数组）: {v}"))
}

// ---------------------------------------------------------------------------
// 1) 非 admin 无数据权限行：默认脱敏必须作用于真实键 mobile_phone
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_admin_list_masks_mobile_phone_email_and_drops_address() {
    let app = build_app(seeded_state(None).await, make_auth(50, 2, "self"));
    let (status, v) = get_json(&app, "/erp/crm/leads?page=1&page_size=10").await;
    assert_eq!(status, StatusCode::OK, "列表 200 契约: {v}");
    let items = list_items(&v);
    assert_eq!(items.len(), 1, "self 范围应命中本人 owner 线索: {v}");
    let lead = &items[0];

    // 核心回归点：修复前该键读不到（contact_phone 不存在）→ 原值直通
    assert_eq!(
        lead["mobile_phone"],
        json!("138****8888"),
        "mask_phone（前3后4，utils/field_mask.rs:5）未生效——手机号回潮泄露"
    );
    let phone = lead["mobile_phone"].as_str().unwrap();
    assert!(!phone.contains("1234"), "掩码后仍含原文中段数字: {phone}");
    assert_ne!(phone, RAW_PHONE);
    assert_eq!(lead["email"], json!("a***@example.com"), "email 掩码回归");
    assert!(
        lead.get("address").is_none(),
        "address 应被移除（P1-08-5 既有语义）: {lead}"
    );
}

#[tokio::test]
async fn non_admin_detail_masks_mobile_phone_email_and_drops_address() {
    let app = build_app(seeded_state(None).await, make_auth(50, 2, "self"));
    let (status, v) = get_json(&app, "/erp/crm/leads/1").await;
    assert_eq!(status, StatusCode::OK, "详情 200 契约: {v}");
    let lead = &v["data"];
    assert_eq!(
        lead["mobile_phone"],
        json!("138****8888"),
        "get_lead 与 list_leads 必须同源打码（同一根因，两处站点）"
    );
    assert_eq!(lead["email"], json!("a***@example.com"));
    assert!(lead.get("address").is_none());
}

// ---------------------------------------------------------------------------
// 2) 有数据权限行：hidden_fields 生效，不走默认打码（既有语义不回归）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn role_with_data_permission_hidden_fields_takes_precedence() {
    let app = build_app(
        seeded_state(Some(&["mobile_phone", "address"])).await,
        make_auth(50, 2, "self"),
    );
    let (status, v) = get_json(&app, "/erp/crm/leads?page=1&page_size=10").await;
    assert_eq!(status, StatusCode::OK);
    let lead = &list_items(&v)[0];
    assert!(
        lead.get("mobile_phone").is_none(),
        "配置行 hidden_fields 应直接移除该键（非打码）: {lead}"
    );
    assert!(lead.get("address").is_none());
    // 该分支不叠加默认打码：email 保持原值（既有行为锁）
    assert_eq!(lead["email"], json!(RAW_EMAIL));
}

// ---------------------------------------------------------------------------
// 3) admin（role_id=1）：保持原值
//    依据：get_role_data_permission 对 roles.code='admin' 返回
//    Ok(Some{ allowed:None, hidden:None })（data_permission_service.rs:64）→
//    filter_fields_batch 空操作；默认打码分支 `role_id != 1` 亦不可达。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_list_and_detail_keep_raw_values() {
    let app = build_app(seeded_state(None).await, make_auth(50, 1, "all"));
    let (_, v) = get_json(&app, "/erp/crm/leads?page=1&page_size=10").await;
    let lead = &list_items(&v)[0];
    assert_eq!(lead["mobile_phone"], json!(RAW_PHONE), "admin 原值契约");
    assert_eq!(lead["email"], json!(RAW_EMAIL));
    assert_eq!(lead["address"], json!(RAW_ADDRESS));

    let (_, v) = get_json(&app, "/erp/crm/leads/1").await;
    assert_eq!(v["data"]["mobile_phone"], json!(RAW_PHONE));
}

// ---------------------------------------------------------------------------
// 4) 源码扫描锁：crm_handler.rs 禁止再出现 contact_phone 读键（回潮即红）
//    contact_phone 属 customer/customer_address/sales_order/supplier 模型，
//    crm_lead 出参不存在该键——读错键 = 脱敏恒不生效。
// ---------------------------------------------------------------------------

#[test]
fn crm_handler_never_masks_by_nonexistent_contact_phone_key() {
    let src = include_str!("../src/handlers/crm_handler.rs");
    assert!(
        !src.contains(r#"get("contact_phone")"#),
        "crm_handler.rs 回潮：对 crm_lead 出参读不存在的 contact_phone 键打码"
    );
    assert!(
        src.contains(r#"get("mobile_phone")"#),
        "crm_handler.rs 默认脱敏必须读真实列 mobile_phone（models/crm_lead.rs:40）"
    );
}
