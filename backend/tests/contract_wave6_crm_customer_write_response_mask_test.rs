//! 契约波次 6 · 看板 #212-A/#212-B：CRM 客户域写响应不得整行原文回传 PII，
//! 共享落库展示名不得 `format!` 造假
//!
//! 根因（与已修的线索/公海写响应 #204/#209 同一旁路类，换了端点与资源）：
//! 1. **客户域写响应整行原文回传**：`crm_customer_handler` 的
//!    `create_customer`（增强建档，落 `crm_lead` 行）/ `update_customer`（增强更新，落
//!    `customers` 行）/ `add_tags`（更新线索行）/ `create_contact` / `update_contact`
//!    （`customer_contacts` 行）五个成功响应此前都是 `serde_json::to_value(Model)?` 直出，
//!    携带 `contact_phone`/`contact_email`/`mobile_phone`/`tel_phone`/`email`/`phone`/
//!    `address` 明文。
//!    收口：新增客户域字段级权限唯一实现
//!    `crm_customer_handler::apply_customer_field_permission`（判定源
//!    `get_role_data_permission(role_id, "customer")` —— 与既有客户域读出口
//!    `customer_handler.rs:154/:177/:222/:239` 用的同一个真实 resource_type 取值，不是新键；
//!    字段过滤复用同一个 `filter_fields_batch`；默认脱敏复用
//!    `CrmService::mask_customer_pii_defaults` → 权威列集合 `utils/field_mask`），
//!    线索形状的两出口（`create_customer`/`add_tags`）复用线索侧既有
//!    `crm_handler::apply_lead_field_permission`（行形状决定 resource_type，不新造口径）。
//! 2. **共享落库展示名造假**：`customer_team_share_service::share_customer` 为取操作人姓名
//!    额外查一次 `users`，查不到即 `format!("用户{operator_id}")` 落 `shared_by_user_name`。
//!    收口照 `services/crm/{pool,lead,opp}.rs` 的做法：操作人姓名由调用方以
//!    `AuthContext.username`（真实登录名）作入参传入，service 内那次取名查询整体删除。
//!
//! 【本文件刻意没有覆盖的点 · 待用户裁定，非遗漏】
//! `customer_transfer_approval_service.rs` 的 `manager_approve`（:319 区）与
//! `director_approve`（:396 区）里 `to_user_name = Set(Some(format!("用户{to_user_id}")))`
//! **本轮未改**：该列语义是**被转移人（第三方）**的展示名，不是操作人本人，按硬规则不得用
//! `auth.username`/审批人姓名顶替（顶替即另一种造假）。换成真值需要么额外查一次
//! `users.username`（与 `execute_transfer → transfer_lead` 内部
//! `fetch_and_validate_new_owner` 那次查询重复），要么改写入次序（审批行先置 approved、
//! 转移成功后再回写姓名，新增"approved 但姓名为空"窗口），二者都**不是行为等价改动**，
//! 交主编排拍板。因此下方 B 的造名零命中棘轮**不含** `customer_transfer_approval_service.rs`
//! （见 `customer_team_share_must_not_fabricate_operator_name` 的清单注释）。
//!
//! 【读出口本轮未收口 · 另有其事先，非本文件范围】
//! 同一 `crm_customer_handler.rs` 的**读**出口 `list_customers`（GET /crm/customers/enhanced）、
//! `get_customer`（GET /crm/customers/enhanced/:id）、`list_contacts`
//! （GET /crm/customers/:id/contacts）此前与现在都仍是整行 `to_value` 直出（服务层
//! `CrmService::list_leads`/`get_lead` 只施行级过滤、不打码，打码一直在 handler 层）。
//! 本轮按指示**只收口写响应一侧**，读出口的生效范围（含 `customer_handler.rs` 那套读打码是否
//! 要并入同一实现）属需裁定项，已写入交付报告；因此下方写出口棘轮刻意不做"整文件级"
//! 负断言，避免误伤读出口或把未裁定的范围顺手改掉。
//!
//! 前端消费面证据（写响应 mask 的影响面）：
//! - `frontend/src/views/crm/tabs/CustomerListTab.vue:604/607`：`updateCustomer`/`createCustomer`
//!   之后 `await` 丢弃响应体，靠 `fetchCustomerList()` 重新拉列表；
//! - `frontend/src/views/crm/detail.vue:496/499`：联系人写之后 `fetchContacts()` 重新 GET；
//! - `frontend/e2e/crm/05-assign-share-merge.spec.ts:264`：`POST /crm/customers/{id}/contacts`
//!   的响应体不取任何键，联系方式回读走 `GET /crm/customers/{id}/contacts` 且只断言 `name`；
//! - `frontend/src/api/crm-enhanced.ts`：`POST /crm/customers/enhanced/{id}/tags` 无调用方
//!   （前端用 `/tags/{tagId}`），`add_tags` 出参无人消费。
//! → 写响应掩码/移除 address 不破坏任何既有前端流程；本仓亦不以"为了让断言过"放行原文。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_customer_handler::{
    add_tags, create_contact, create_customer, update_contact, update_customer,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::customer_share;
use bingxi_backend::services::crm::customer_team_share_service::{
    CustomerTeamShareService, ShareCustomerRequest,
};
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 种子与夹具（与 contract_wave6_crm_write_response_mask_test.rs 同一套 sqlite DDL 口径）
// ---------------------------------------------------------------------------

const OWNER: i32 = 50;
const ADMIN: i32 = 70;
const OWNER_LOGIN: &str = "sales_a";
const ADMIN_LOGIN: &str = "admin_user";
/// 被共享方（第三方）——其姓名取自己那行 `users.username`，不得被操作人姓名顶替
const SHARED_TO: i32 = 51;
const SHARED_TO_LOGIN: &str = "shared_to_wangwu";
/// 操作人真实登录名（B：`shared_by_user_name` 期望值）
const OPERATOR_LOGIN: &str = "sharer_zhangsan";

const C_PHONE: &str = "13812348888";
const C_EMAIL: &str = "alice@example.com";
const C_ADDRESS: &str = "河北省邢台市某某路 1 号";
const MASKED_PHONE: &str = "138****8888";
const MASKED_EMAIL: &str = "a***@example.com";
/// 联系人子资源行的电话/邮箱（列名与 customers 不同：`phone`/`email`）
const CONTACT_PHONE: &str = "13900002222";
const CONTACT_EMAIL: &str = "contact@example.com";
const MASKED_CONTACT_PHONE: &str = "139****2222";
const MASKED_CONTACT_EMAIL: &str = "c***@example.com";

fn make_auth(user_id: i32, username: &str, role_id: Option<i32>, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
        role_id,
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

const CREATE_ROLES: &str = r#"CREATE TABLE roles (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, code TEXT NOT NULL,
    description TEXT, permissions TEXT, is_system INTEGER NOT NULL,
    data_scope TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

const CREATE_DATA_PERMISSIONS: &str = r#"CREATE TABLE data_permissions (
    id INTEGER PRIMARY KEY, role_id INTEGER NOT NULL,
    resource_type TEXT NOT NULL, scope_type TEXT NOT NULL,
    custom_condition TEXT, allowed_fields TEXT, hidden_fields TEXT,
    is_enabled INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

const CREATE_USERS: &str = r#"CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL, password_hash TEXT NOT NULL,
    real_name TEXT, avatar TEXT, email TEXT, phone TEXT,
    role_id INTEGER, department_id INTEGER, is_active INTEGER NOT NULL,
    totp_secret TEXT, is_totp_enabled INTEGER NOT NULL, totp_recovery_codes TEXT,
    last_login_at TEXT, password_changed_at TEXT, agreed_to_terms_at TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    gender TEXT, birth_date TEXT
)"#;

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

/// 与 models/customer.rs::Model 逐列对应（表名 customers）
const CREATE_CUSTOMERS: &str = r#"CREATE TABLE customers (
    id INTEGER PRIMARY KEY,
    customer_code TEXT, customer_name TEXT,
    contact_person TEXT, contact_phone TEXT, contact_email TEXT,
    address TEXT, city TEXT, province TEXT, country TEXT, postal_code TEXT,
    credit_limit TEXT, payment_terms INTEGER, tax_id TEXT,
    bank_name TEXT, bank_account TEXT, status TEXT, customer_type TEXT,
    notes TEXT, created_by INTEGER, created_at TEXT, updated_at TEXT,
    customer_industry TEXT, main_products TEXT, annual_purchase TEXT,
    quality_requirement TEXT, inspection_standard TEXT,
    owner_id INTEGER, department_id INTEGER, owner_assigned_at TEXT,
    special_process TEXT, source TEXT, pool_recycle_reason TEXT
)"#;

/// 与 models/customer_contact.rs::Model 逐列对应（表名 customer_contacts）
const CREATE_CUSTOMER_CONTACTS: &str = r#"CREATE TABLE customer_contacts (
    id INTEGER PRIMARY KEY,
    customer_id INTEGER NOT NULL, name TEXT NOT NULL, title TEXT,
    phone TEXT NOT NULL, email TEXT, is_primary INTEGER NOT NULL,
    remarks TEXT, created_by INTEGER,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

/// 与 models/customer_share.rs::Model 逐列对应（表名 customer_shares）
const CREATE_CUSTOMER_SHARES: &str = r#"CREATE TABLE customer_shares (
    id INTEGER PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    shared_by_user_id INTEGER NOT NULL, shared_by_user_name TEXT,
    shared_to_user_id INTEGER NOT NULL, shared_to_user_name TEXT,
    permission TEXT NOT NULL, status TEXT NOT NULL,
    shared_at TEXT NOT NULL, expire_at TEXT, revoked_at TEXT,
    revoked_by INTEGER, revoke_reason TEXT, share_reason TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

async fn base_state(permissions: Option<&str>) -> AppState {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    for ddl in [
        CREATE_CRM_LEAD,
        CREATE_CUSTOMERS,
        CREATE_CUSTOMER_CONTACTS,
        CREATE_CUSTOMER_SHARES,
        CREATE_ROLES,
        CREATE_DATA_PERMISSIONS,
        CREATE_USERS,
        CREATE_AUDIT_LOGS,
    ] {
        exec(&db, ddl).await;
    }
    exec(
        &db,
        "INSERT INTO roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES
         (1,'系统管理员','admin',1,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'销售专员','sales',0,'self','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,created_at,updated_at) VALUES
         (50,'sales_a','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (51,'shared_to_wangwu','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'admin_user','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // 一条归属 OWNER 的客户主数据行，联系方式与地址均为原文（写响应必须打码）
    exec(
        &db,
        &format!(
            "INSERT INTO customers (id,customer_code,customer_name,contact_person,
             contact_phone,contact_email,address,credit_limit,payment_terms,status,
             customer_type,owner_id,department_id,created_at,updated_at) VALUES
             (1,'CUS-0001','甲客户','张三','{C_PHONE}','{C_EMAIL}','{C_ADDRESS}',
              '0',30,'active','retail',{OWNER},1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    // 一条归属 OWNER 的线索行（增强建档/挂标签出参是线索形状）
    exec(
        &db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,mobile_phone,tel_phone,email,address,owner_id,owner_name,
             department_id,priority,created_at,updated_at) VALUES
             (1,'LD001','website','new','甲公司','张三','{C_PHONE}','03191234567',
              '{C_EMAIL}','{C_ADDRESS}',{OWNER},'销售甲',1,'low',
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;

    if let Some(insert_sql) = permissions {
        exec(&db, insert_sql).await;
    }

    let mut state = AppState::default();
    state.db = Arc::new(db);
    state.data_permission_service = Arc::new(DataPermissionService::new(state.db.clone()));
    state
}

/// 客户域五个写出口按真实挂载路径（`routes/crm.rs::crm_customers()`）建路由
fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/customers/enhanced", post(create_customer))
        .route("/erp/crm/customers/enhanced/{id}", put(update_customer))
        .route("/erp/crm/customers/{id}/tags", post(add_tags))
        .route("/erp/crm/customers/{id}/contacts", post(create_contact))
        .route(
            "/erp/crm/customers/{id}/contacts/{contact_id}",
            put(update_contact),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn send(app: &Router, method: Method, uri: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
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

async fn put_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    send(app, Method::PUT, uri, body).await
}

async fn post_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    send(app, Method::POST, uri, body).await
}

/// 出参字符串里不得出现任何原文 PII（无论列名如何、无论嵌套形状）
fn assert_no_raw_pii(value: &Value, where_label: &str) {
    let raw = value.to_string();
    for banned in [C_PHONE, C_EMAIL, C_ADDRESS, CONTACT_PHONE, CONTACT_EMAIL] {
        assert!(
            !raw.contains(banned),
            "{where_label}：成功响应体含未脱敏个人信息 {banned}（整行原文回传旁路回潮）\n出参: {raw}"
        );
    }
}

/// 客户主数据行的默认脱敏断言（列名取自 models/customer.rs，掩码实现属 utils/field_mask 权威集合）
fn assert_customer_row_default_masked(row: &Value, where_label: &str) {
    assert_eq!(
        row["contact_phone"],
        json!(MASKED_PHONE),
        "{where_label}：contact_phone 未走 utils/field_mask 权威集合掩码"
    );
    assert_eq!(
        row["contact_email"],
        json!(MASKED_EMAIL),
        "{where_label}：contact_email 未走 utils/field_mask 权威集合掩码"
    );
    assert!(
        row.get("address").is_none(),
        "{where_label}：address 应被整键移除（与线索侧 mask_lead_pii_defaults 同一口径）: {row}"
    );
    assert_no_raw_pii(row, where_label);
}

// ---------------------------------------------------------------------------
// A-1 · 客户主数据写响应（PUT /crm/customers/enhanced/:id）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_admin_update_customer_response_is_masked_not_raw_pii() {
    let app = build_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = put_json(
        &app,
        "/erp/crm/customers/enhanced/1",
        json!({"customer_name": "甲客户改名"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "归属人更新自己客户应成功: {v}");
    let data = &v["data"];
    assert_eq!(
        data["customer_name"],
        json!("甲客户改名"),
        "更新须真实生效（非只回掩码）"
    );
    assert_customer_row_default_masked(data, "PUT /crm/customers/enhanced/:id 写响应");
}

#[tokio::test]
async fn admin_update_customer_response_keeps_raw_pii() {
    let app = build_app(
        base_state(None).await,
        make_auth(ADMIN, ADMIN_LOGIN, Some(1), "all"),
    );
    let (status, v) = put_json(
        &app,
        "/erp/crm/customers/enhanced/1",
        json!({"customer_name": "甲客户admin改"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin 更新应成功: {v}");
    // admin 原值契约（role_id==Some(1) 由 mask_contact_fields_for_role 自身放行，含 address）
    let data = &v["data"];
    assert_eq!(
        data["contact_phone"],
        json!(C_PHONE),
        "admin contact_phone 原值契约"
    );
    assert_eq!(
        data["contact_email"],
        json!(C_EMAIL),
        "admin contact_email 原值契约"
    );
    assert_eq!(
        data["address"],
        json!(C_ADDRESS),
        "admin address 原值契约（不得移除）"
    );
}

#[tokio::test]
async fn missing_role_update_customer_response_is_fail_closed_masked() {
    // role_id 缺失（None）属 fail-closed 分支：必须按无权限行走默认脱敏，不得原文放行
    let app = build_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, None, "self"),
    );
    let (status, v) = put_json(
        &app,
        "/erp/crm/customers/enhanced/1",
        json!({"customer_name": "无角色改"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "role_id 缺失不影响更新本身: {v}");
    assert_customer_row_default_masked(&v["data"], "role_id 缺失（fail-closed）写响应");
}

#[tokio::test]
async fn customer_write_response_honors_hidden_fields_of_customer_resource() {
    // 判定源真实取值 resource_type='customer'（与 customer_handler.rs 读出口同一取值）：
    // 配了权限行 → 走同一个 filter_fields_batch（hidden 移除，不叠加默认掩码）
    let insert = r#"INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
        allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
        VALUES (1,2,'customer','SELF',NULL,'["contact_phone"]',1,
        '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#;
    let app = build_app(
        base_state(Some(insert)).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = put_json(
        &app,
        "/erp/crm/customers/enhanced/1",
        json!({"customer_name": "甲客户改名"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "更新应成功: {v}");
    let data = &v["data"];
    assert!(
        data.get("contact_phone").is_none(),
        "客户写响应应与读出口共用同一个 filter_fields_batch 移除 hidden 列: {data}"
    );
    // 未被 hidden 覆盖的列保持原值：证明是精准移除，而非整行清空或额外掩码
    assert_eq!(
        data["contact_email"],
        json!(C_EMAIL),
        "hidden_fields 仅移除该列，不叠加默认掩码（等价于读出口既有语义）"
    );
}

// ---------------------------------------------------------------------------
// A-2 · 联系人子资源写响应（POST/PUT /crm/customers/:id/contacts）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_admin_create_contact_response_is_masked() {
    let app = build_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = post_json(
        &app,
        "/erp/crm/customers/1/contacts",
        json!({"name": "联系人甲", "phone": CONTACT_PHONE, "email": CONTACT_EMAIL}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "创建联系人应成功: {v}");
    let data = &v["data"];
    assert_eq!(
        data["phone"],
        json!(MASKED_CONTACT_PHONE),
        "联系人 phone 未走 utils/field_mask 权威集合掩码"
    );
    assert_eq!(
        data["email"],
        json!(MASKED_CONTACT_EMAIL),
        "联系人 email 未走 utils/field_mask 权威集合掩码"
    );
    assert_eq!(data["name"], json!("联系人甲"), "非 PII 列不受影响");
    assert_no_raw_pii(data, "POST /crm/customers/:id/contacts 写响应");
}

#[tokio::test]
async fn admin_create_contact_response_keeps_raw_pii() {
    let app = build_app(
        base_state(None).await,
        make_auth(ADMIN, ADMIN_LOGIN, Some(1), "all"),
    );
    let (status, v) = post_json(
        &app,
        "/erp/crm/customers/1/contacts",
        json!({"name": "联系人甲", "phone": CONTACT_PHONE, "email": CONTACT_EMAIL}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin 创建联系人应成功: {v}");
    assert_eq!(
        v["data"]["phone"],
        json!(CONTACT_PHONE),
        "admin phone 原值契约"
    );
    assert_eq!(
        v["data"]["email"],
        json!(CONTACT_EMAIL),
        "admin email 原值契约"
    );
}

#[tokio::test]
async fn contact_update_response_is_masked_and_shares_customer_judgement_source() {
    // 同一判定源验证：role 2 配了 resource_type='customer' 的 hidden=["phone"] 时，
    // 联系人写响应也走同一个 filter_fields_batch（phone 整键移除、email 不叠加掩码）
    let insert = r#"INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
        allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
        VALUES (1,2,'customer','SELF',NULL,'["phone"]',1,
        '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#;
    let state = base_state(Some(insert)).await;
    exec(
        &*state.db,
        &format!(
            "INSERT INTO customer_contacts (id,customer_id,name,phone,email,is_primary,
             created_at,updated_at) VALUES
             (1,1,'联系人甲','{CONTACT_PHONE}','{CONTACT_EMAIL}',0,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    let app = build_app(state, make_auth(OWNER, OWNER_LOGIN, Some(2), "self"));
    let (status, v) = put_json(
        &app,
        "/erp/crm/customers/1/contacts/1",
        json!({"name": "联系人甲改", "phone": CONTACT_PHONE, "email": CONTACT_EMAIL}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "更新联系人应成功: {v}");
    assert!(
        v["data"].get("phone").is_none(),
        "联系人写响应应与客户域读出口共用同一个 filter_fields_batch（hidden 列移除）: {}",
        v["data"]
    );
    // 未被 hidden 覆盖的列保持原值（配了权限行 → 不叠加默认掩码，与线索/商机同一既有语义）
    assert_eq!(
        v["data"]["email"],
        json!(CONTACT_EMAIL),
        "hidden_fields 仅移除 phone，不得叠加默认掩码"
    );
    assert_eq!(v["data"]["name"], json!("联系人甲改"), "更新须真实生效");
}

// ---------------------------------------------------------------------------
// A-3 · 客户域中形状是"线索行"的两个写出口（复用线索侧同一实现，不新造口径）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn enhanced_create_customer_response_is_masked_lead_row() {
    let app = build_app(
        base_state(None).await,
        make_auth(OWNER, OPERATOR_LOGIN, Some(2), "self"),
    );
    // 显式 lead_no 跳过 PG advisory_xact_lock 取号（与既有线索写响应锁同一做法）
    let (status, v) = post_json(
        &app,
        "/erp/crm/customers/enhanced",
        json!({
            "lead_no": "LD-CUS-9001",
            "company_name": "建档甲公司",
            "contact_name": "建档联系人",
            "mobile_phone": C_PHONE,
            "tel_phone": "03191234567",
            "email": C_EMAIL,
            "address": C_ADDRESS,
            "lead_status": "new",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "增强建档应成功: {v}");
    let data = &v["data"];
    assert_eq!(
        data["mobile_phone"],
        json!(MASKED_PHONE),
        "建档响应 mobile_phone 未掩码"
    );
    assert_eq!(
        data["tel_phone"],
        json!("031****4567"),
        "建档响应 tel_phone 未掩码"
    );
    assert_eq!(data["email"], json!(MASKED_EMAIL), "建档响应 email 未掩码");
    assert!(
        data.get("address").is_none(),
        "建档响应 address 应整键移除: {data}"
    );
    assert_no_raw_pii(data, "POST /crm/customers/enhanced 写响应（线索形状）");
    // B 同源口径：建档归属名是真实登录名，不是 format! 造名
    assert_eq!(
        data["owner_name"],
        json!(OPERATOR_LOGIN),
        "建档 owner_name 应为传入登录名"
    );
}

#[tokio::test]
async fn admin_enhanced_create_customer_response_keeps_raw_pii() {
    let app = build_app(
        base_state(None).await,
        make_auth(ADMIN, ADMIN_LOGIN, Some(1), "all"),
    );
    let (status, v) = post_json(
        &app,
        "/erp/crm/customers/enhanced",
        json!({
            "lead_no": "LD-CUS-9002",
            "company_name": "建档乙公司",
            "contact_name": "建档联系人乙",
            "mobile_phone": C_PHONE,
            "email": C_EMAIL,
            "address": C_ADDRESS,
            "lead_status": "new",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin 建档应成功: {v}");
    assert_eq!(v["data"]["mobile_phone"], json!(C_PHONE), "admin 原值契约");
    assert_eq!(v["data"]["email"], json!(C_EMAIL), "admin 原值契约");
    assert_eq!(
        v["data"]["address"],
        json!(C_ADDRESS),
        "admin address 原值契约"
    );
}

#[tokio::test]
async fn add_tags_response_is_masked_lead_row() {
    let app = build_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = post_json(
        &app,
        "/erp/crm/customers/1/tags",
        json!({"tags": ["重点", "纺织"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "挂标签应成功: {v}");
    // 出参必须是那条真实线索行（非空壳）；以稳定字符串/数值列证明。
    // tags 的回显形状不属本锁范围（本锁只断言 PII 不再整行原文直出）。
    assert_eq!(
        v["data"]["company_name"],
        json!("甲公司"),
        "应回写真实线索行"
    );
    assert_eq!(v["data"]["owner_id"], json!(OWNER), "应回写真实线索行");
    assert_customer_lead_masked(&v["data"]);
    assert_no_raw_pii(
        &v["data"],
        "POST /crm/customers/:id/tags 写响应（线索形状）",
    );
}

fn assert_customer_lead_masked(lead: &Value) {
    assert_eq!(
        lead["mobile_phone"],
        json!(MASKED_PHONE),
        "线索形状 mobile_phone 未掩码"
    );
    assert_eq!(lead["email"], json!(MASKED_EMAIL), "线索形状 email 未掩码");
    assert!(
        lead.get("address").is_none(),
        "线索形状 address 应被移除: {lead}"
    );
}

// ---------------------------------------------------------------------------
// B · 共享落库展示名 = 传入的真实登录名（不造名、不为取名额外查库）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn share_customer_lands_real_operator_login_and_keeps_third_party_name() {
    let state = base_state(None).await;
    let db = state.db.clone();

    let svc = CustomerTeamShareService::new(db.clone());
    let dto = svc
        .share_customer(
            ShareCustomerRequest {
                customer_id: 1,
                shared_to_user_id: SHARED_TO,
                permission: Some("view".to_string()),
                duration_days: Some(30),
                share_reason: Some("E2E 协作".to_string()),
            },
            OWNER,
            OPERATOR_LOGIN,
        )
        .await
        .expect("owner 共享客户应成功");

    // 操作人列（shared_by_user_name）：等于调用方传入的真实登录名
    assert_eq!(
        dto.shared_by_user_name.as_deref(),
        Some(OPERATOR_LOGIN),
        "shared_by_user_name 必须等于传入的真实登录名"
    );
    let shared_by = dto.shared_by_user_name.clone().unwrap_or_default();
    assert!(
        !shared_by.starts_with("用户"),
        "B 负向锁：shared_by_user_name 不得是 format!(\"用户{{id}}\") 造出的展示名，实际: {shared_by}"
    );

    // 第三方列（shared_to_user_name）：必须是被共享方自己的 users.username，
    // 不得被操作人姓名顶替（本轮未改该列来源，此处即为其行为锁）
    assert_eq!(
        dto.shared_to_user_name.as_deref(),
        Some(SHARED_TO_LOGIN),
        "shared_to_user_name 必须是被共享方 users.username，不得用操作人姓名冒充"
    );

    // 回读落库行（不信 DTO 自说自话）
    let row = customer_share::Entity::find_by_id(dto.id)
        .one(&*db)
        .await
        .expect("查询落库共享记录失败")
        .expect("共享记录必须真实落库");
    assert_eq!(
        row.shared_by_user_name.as_deref(),
        Some(OPERATOR_LOGIN),
        "落库 shared_by_user_name 应为传入登录名"
    );
    assert!(
        !row.shared_by_user_name
            .unwrap_or_default()
            .starts_with("用户"),
        "B 负向锁：落库值不得为造名"
    );
}

#[tokio::test]
async fn share_customer_does_not_query_users_for_operator_name() {
    // 操作人 999 在 users 表不存在（修复前会 fallback 到 format!("用户999")）：
    // 现姓名完全来自入参，不查库 ⇒ 仍原样落传入值，且整体成功（不静默、不兜底）。
    let state = base_state(None).await;
    let db = state.db.clone();
    exec(
        &db,
        &format!(
            "INSERT INTO customers (id,customer_code,customer_name,contact_phone,contact_email,
             credit_limit,payment_terms,status,customer_type,owner_id,created_at,updated_at)
             VALUES (2,'CUS-0002','乙客户','13711112222','b@example.com','0',30,'active',
                     'retail',999,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    let svc = CustomerTeamShareService::new(db.clone());
    let dto = svc
        .share_customer(
            ShareCustomerRequest {
                customer_id: 2,
                shared_to_user_id: SHARED_TO,
                permission: None,
                duration_days: None,
                share_reason: None,
            },
            999,
            "ghost_operator_login",
        )
        .await
        .expect("操作人姓名来自入参，与 users 是否有该行无关");
    assert_eq!(
        dto.shared_by_user_name.as_deref(),
        Some("ghost_operator_login"),
        "不再为取名查库：入参登录名即落库值"
    );
    assert_eq!(
        dto.shared_to_user_name.as_deref(),
        Some(SHARED_TO_LOGIN),
        "第三方姓名仍取被共享方自己的 users.username"
    );
}

// ---------------------------------------------------------------------------
// C · 源码扫描棘轮（shrink-only）
// ---------------------------------------------------------------------------

/// 客户域五个写出口必须挂在字段级权限唯一实现上，不得退回整行原文直出。
/// 线索形状两出口 → `apply_lead_field_permission`；客户形状三出口 →
/// `apply_customer_field_permission`。
#[test]
fn customer_write_exits_all_route_through_field_permission() {
    let handler = include_str!("../src/handlers/crm_customer_handler.rs");

    assert!(
        handler.contains("pub(crate) async fn apply_customer_field_permission"),
        "客户域字段级权限唯一实现缺失"
    );
    // 同一判定源：resource_type 用库内真实取值 "customer"，不是新造键
    assert!(
        handler.contains(r#"get_role_data_permission(rid, "customer")"#),
        "apply_customer_field_permission 未使用真实 resource_type=customer 判定源"
    );
    // 同一字段过滤函数
    assert!(
        handler.contains("filter_fields_batch("),
        "客户域必须复用 data_permission_service::filter_fields_batch"
    );
    // fail-closed：role_id 缺失与权限查询 Err 都不得原文放行
    assert!(
        handler.contains("if let Some(rid) = role_id"),
        "客户域缺 role_id 缺失分支（无角色 = 原文放行旁路）"
    );
    assert!(
        handler.contains("tracing::warn!(") && handler.contains("fail-closed"),
        "权限查询失败必须显式记 warn 并 fail-closed，不得静默"
    );

    let body_of = |name: &str| -> String {
        handler
            .split(&format!("pub async fn {name}"))
            .nth(1)
            .unwrap_or_else(|| panic!("{name} 定义缺失"))
            .split("pub async fn ")
            .next()
            .unwrap_or_else(|| panic!("{name} 函数体边界缺失"))
            .to_string()
    };

    for name in ["update_customer", "create_contact", "update_contact"] {
        let body = body_of(name);
        assert!(
            body.contains("apply_customer_field_permission("),
            "回潮棘轮：客户域 {name} 成功响应未走客户字段级权限唯一实现（整行原文回传 PII）"
        );
    }
    for name in ["create_customer", "add_tags"] {
        let body = body_of(name);
        assert!(
            body.contains("apply_lead_field_permission("),
            "回潮棘轮：{name} 出参形状是线索行，却未复用线索侧同一实现（口径分叉）"
        );
    }
    // 每个**写出口体内**都不得把整行 Model 直接塞进 ApiResponse（读打码、写原文旁路的字面形态）。
    // 断言限定在写出口函数体：本文件的读出口（list_customers/get_customer/list_contacts）
    // 仍是整行 to_value 直出，那属另一条待裁定口径（见本文件头"读出口未收口"），
    // 不在本棘轮范围内，故不做整文件级负断言（做了会误伤读出口，也会掩盖真正要锁的写出口）。
    for name in [
        "create_customer",
        "update_customer",
        "add_tags",
        "create_contact",
        "update_contact",
    ] {
        let body = body_of(name);
        for raw_exit in [
            "ApiResponse::success(serde_json::to_value(lead)?)",
            "ApiResponse::success(serde_json::to_value(customer)?)",
            "serde_json::to_value(contact)?,",
        ] {
            assert!(
                !body.contains(raw_exit),
                "回潮棘轮：写出口 {name} 又出现整行原文直出 → {raw_exit}"
            );
        }
    }
}

/// 默认脱敏必须是"复用 utils/field_mask 权威集合 + address 整键移除"，且权威集合
/// 确实覆盖客户域真实列（读错键 = 该列脱敏恒不生效，正是本波次线索侧的根因）。
#[test]
fn customer_default_mask_is_single_implementation_on_real_columns() {
    let service = include_str!("../src/services/crm/cust.rs");
    let field_mask = include_str!("../src/utils/field_mask.rs");

    assert!(
        service.contains("pub fn mask_customer_pii_defaults"),
        "客户域默认脱敏唯一实现缺失"
    );
    assert!(
        service.contains("mask_contact_fields_for_role"),
        "客户域默认脱敏必须复用 utils/field_mask 权威列集合，不得另造第二套列名清单"
    );
    // 客户域不得在 handler/service 里手写掩码分支（列集合只在 utils/field_mask 一处）
    let handler = include_str!("../src/handlers/crm_customer_handler.rs");
    for banned in [
        "mask_phone(",
        "mask_email(",
        r#"obj.remove("contact_phone")"#,
    ] {
        assert!(
            !handler.contains(banned),
            "回潮棘轮：crm_customer_handler.rs 出现内联掩码写回分支 → {banned}"
        );
    }

    let phone_keys = field_mask
        .split("pub fn mask_contact_fields_for_role")
        .nth(1)
        .expect("mask_contact_fields_for_role 定义缺失")
        .split("let email_keys")
        .next()
        .expect("电话列集合边界缺失");
    for key in ["contact_phone", "phone", "mobile_phone", "tel_phone"] {
        assert!(
            phone_keys.contains(&format!("\"{key}\"")),
            "utils/field_mask.rs 电话列集合缺客户/联系人/线索真实列 {key}（默认脱敏对该列恒不生效）"
        );
    }
    let email_keys = field_mask
        .split("let email_keys")
        .nth(1)
        .expect("邮箱列集合边界缺失");
    for key in ["contact_email", "email"] {
        assert!(
            email_keys.contains(&format!("\"{key}\"")),
            "utils/field_mask.rs 邮箱列集合缺客户/联系人真实列 {key}"
        );
    }
}

/// B 造名零命中棘轮：把本批改掉的文件纳入既有集合（`contract_wave6_crm_write_response_mask_test.rs`
/// 已锁 `services/crm/{lead,opp,pool}.rs`），并显式登记**未纳入**者及原因——
/// 不为让锁通过而假装改掉。
#[test]
fn customer_team_share_must_not_fabricate_operator_name() {
    let share_service = include_str!("../src/services/crm/customer_team_share_service.rs");

    assert!(
        !share_service.contains("format!(\"用户{}"),
        "回潮棘轮：customer_team_share_service.rs 用 format! 伪造展示名"
    );
    assert!(
        !share_service.contains("user::Entity::find_by_id(operator_id)"),
        "回潮棘轮：又出现为取操作人姓名额外查 users（姓名应由调用方传入 auth.username）"
    );
    // 正向锁：操作人展示名来自入参；被共享方（第三方）姓名来自其自己的真实行
    assert!(
        share_service.contains("shared_by_user_name: Set(Some(shared_by_user_name))"),
        "share_customer 的 shared_by_user_name 必须来自 operator_name 入参"
    );
    assert!(
        share_service.contains("shared_to_user_name: Set(Some(to_user.username.clone()))"),
        "shared_to_user_name 必须是被共享方 users.username（第三方），不得由操作人姓名顶替"
    );

    // 【未纳入清单 · 待用户裁定，勿误读为已修】
    // customer_transfer_approval_service.rs（manager_approve / director_approve 两处
    // `to_user_name = Set(Some(format!("用户{to_user_id}")))`）**不在**本零命中集合内：
    // 该列语义是被转移人（第三方）的展示名，本轮无真实来源可等价替换 ——
    // 用审批人/操作人姓名顶替即造假，改查库或改写序均非行为等价。详见源文件同位置注释与本文件头。
    let transfer_service =
        include_str!("../src/services/crm/customer_transfer_approval_service.rs");
    assert!(
        transfer_service.contains("待用户裁定"),
        "客户转移审批的 to_user_name 造名点必须带显式待裁定标注（不得静默遗留）"
    );
    assert!(
        transfer_service.contains("Set(Some(format!(\"用户{}\", to_user_id)))"),
        "本断言仅用于确认该待裁定点仍在原位置（一旦按拍板改掉，须同步删除本断言并纳入零命中集合）"
    );
}

/// 调用点完整性锁：`share_customer` 服务方法的唯一调用方必须传真实登录名
/// （Windows 本地 `cargo check` 不带 `--all-targets` 抓不到跨文件签名漏改，故加字面锁）。
#[test]
fn share_customer_call_site_passes_real_login() {
    let handler = include_str!("../src/handlers/customer_team_share_handler.rs");
    assert!(
        handler.contains(".share_customer(req, auth.user_id, &auth.username)"),
        "share_customer 调用点必须以 &auth.username 传入操作人真实登录名"
    );
}
