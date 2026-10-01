//! 契约波次 6 · 任务 #209：CRM 建单/更新/关单成功响应不得整行原文回传
//! （线索写响应旁路收口 + 商机字段级出参四出口收敛的等价性锁）
//!
//! 根因（与已修的公海领取/回收 #204 同一旁路类，只是换了端点）：
//! 1. **线索写响应回原文 PII**：`crm_handler::create_lead` / `update_lead` 直接
//!    `serde_json::to_value(res)?` 返回整行 `crm_lead::Model`（含 `mobile_phone`/
//!    `tel_phone`/`email`/`address` 明文）。`update_lead` 虽受行级 `check_resource_owner`
//!    约束，但 **Dept 数据范围用户可合法更新他人名下的行**，其响应即把他人联系方式原文
//!    送出——而同一个人 `GET /crm/leads/:id` 拿到的是打码值。即"读打码、写原文"。
//!    修复：写响应复用读路径唯一实现 `apply_lead_field_permission`（与列表/详情/公海写响应同源）。
//! 2. **商机字段级出参各写一份内联分支**：`list_opportunities`/`get_opportunity` 各自内联
//!    `if let Ok(Some(..))` + `obj.remove("amount")`，而 `create_opportunity`/`update_opportunity`/
//!    `close_opportunity_as_lost` 完全没有字段处理（整行原文直出）。
//!    修复：把"商机的字段级出参处理"收敛为唯一实现 `apply_opportunity_field_permission`，
//!    五个出口共用。**本轮只做等价重构**：改造前默认分支写的是 `obj.remove("amount")`，
//!    而 `crm_opportunity` 出参根本没有 `amount` 键（真实金额列是 `estimated_amount`/
//!    `actual_amount`），该"移除"当前恒不生效——本函数原样保留这一事实（见交付报告"待决点"：
//!    金额默认隐藏范围属待用户拍板项，本轮不收紧），仅让配了数据权限行的角色在**全部五个出口**
//!    （含此前完全没有字段处理的建单/更新/关单写响应）都走同一个 `filter_fields_batch`。
//!
//! 附带收口（#209 B）：`create_lead` / `create_opportunity` 的 `owner_name` 由
//! `format!("用户{user_id}")` 改为传入的真实登录名（`&auth.username`，与公海领取
//! `services/crm/pool.rs` 同一口径）；本文件用运行时断言锁定"落库 owner_name == 传入登录名
//! 且不以『用户』开头"。
//!
//! 前端消费面（A.4 证据）：`frontend/src/views/crm/**` 的建单/更新提交（LeadFormTab.vue /
//! OpportunityFormTab.vue / opportunities/index.vue）与 `frontend/e2e/crm/**` 均只 `await`
//! 写响应后靠 `getList()`/GET 重新拉取，**不读取写响应体的 address/mobile_phone/tel_phone/email**
//! （e2e 仅取 `created.data.id`）→ 写响应 mask/移除 address 不破坏任何前端流程。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler::{create_lead, update_lead, update_opportunity};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::customer;
use bingxi_backend::services::crm::cust::CrmService;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 常量：原始 PII 与期望掩码值（与 utils/field_mask.rs::mask_phone/mask_email 逐字符一致，
// 同 contract_wave6_crm_lead_pii_convergence_test.rs）
// ---------------------------------------------------------------------------

const USER_A: i32 = 50; // 线索/商机原归属人
const USER_ADMIN: i32 = 70;
const A_PHONE: &str = "13812348888";
const A_TEL: &str = "03191234567";
const A_EMAIL: &str = "alice@example.com";
const A_ADDRESS: &str = "河北省邢台市某某路 1 号";
const MASKED_PHONE: &str = "138****8888";
const MASKED_TEL: &str = "031****4567";
const MASKED_EMAIL: &str = "a***@example.com";

/// Dept 更新者登录名（真实身份字段，用于 B 的 owner_name 断言与"更新他人行"身份）
const DEPT_UPDATER: i32 = 55;
const DEPT_UPDATER_LOGIN: &str = "dept_sales_zhangsan";
/// 建单人登录名（B：新建线索的 owner_name 期望值）
const CREATOR_LOGIN: &str = "creator_lisi";

fn make_auth(user_id: i32, username: &str, role_id: i32, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
        role_id: Some(role_id),
        department_id: Some(1),
        data_scope: Some(data_scope.to_string()),
        dept_ids: if data_scope == "dept" {
            Some(Arc::new("1".to_string()))
        } else {
            None
        },
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

/// 与 models/crm_opportunity.rs::Model 逐列对应（同 contract_wave6_crm_opp_export_scope_test.rs）
const CREATE_CRM_OPPORTUNITY: &str = r#"CREATE TABLE crm_opportunity (
    id INTEGER PRIMARY KEY,
    opportunity_no TEXT NOT NULL UNIQUE, opportunity_name TEXT NOT NULL,
    customer_id INTEGER NOT NULL, lead_id INTEGER, opportunity_type TEXT,
    opportunity_stage TEXT, win_probability TEXT,
    estimated_amount TEXT, actual_amount TEXT, currency TEXT,
    expected_close_date TEXT, actual_close_date TEXT,
    product_ids TEXT, product_names TEXT, product_desc TEXT,
    owner_id INTEGER NOT NULL, department_id INTEGER, owner_name TEXT NOT NULL,
    last_follow_up_date TEXT, next_follow_up_date TEXT, follow_up_plan TEXT,
    competitor_names TEXT, competitive_advantage TEXT, opportunity_status TEXT,
    won_reason TEXT, lost_reason TEXT, priority TEXT, rating INTEGER, tags TEXT,
    created_at TEXT, updated_at TEXT, created_by INTEGER, updated_by INTEGER
)"#;

/// 与 models/customer.rs::Model 逐列对应（表名 customers）。
/// 全部列允许 NULL：create_opportunity 只做 `find_by_id().one()` 存在性校验，
/// 种子经 SeaORM ActiveModel 写入（Decimal/DateTime 走其自身编解码，避免手写字符串口径漂移）。
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

async fn base_state(permissions: Option<&str>) -> AppState {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    for ddl in [
        CREATE_CRM_LEAD,
        CREATE_CRM_OPPORTUNITY,
        CREATE_CUSTOMERS,
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
         (55,'dept_sales_zhangsan','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'admin_user','x',1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // 一条归属 USER_A、department_id=1 的线索，携带原始手机号/座机/邮箱/地址。
    // updated_at 早于保护期不影响本用例（不走领取）。
    exec(
        &db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,mobile_phone,tel_phone,email,address,owner_id,owner_name,
             department_id,priority,created_at,updated_at) VALUES
             (1,'LD001','website','new','甲公司','张三','{A_PHONE}','{A_TEL}','{A_EMAIL}',
              '{A_ADDRESS}',{USER_A},'销售甲',1,'low','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    // 一条归属 USER_A、department_id=1 的商机（金额非空可判定），供商机写响应锁使用。
    exec(
        &db,
        "INSERT INTO crm_opportunity (id,opportunity_no,opportunity_name,customer_id,
         opportunity_stage,estimated_amount,actual_amount,currency,owner_id,owner_name,
         department_id,opportunity_status,priority,created_at,updated_at) VALUES
         (1,'OPP001','商机甲',1,'QUALIFICATION','111111','222222','CNY',50,'销售甲',
          1,'OPEN','high','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
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

fn build_lead_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/leads", post(create_lead))
        .route(
            "/erp/crm/leads/{id}",
            put(update_lead).get(bingxi_backend::handlers::crm_handler::get_lead),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

fn build_opp_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route(
            "/erp/crm/opportunities/{id}",
            put(update_opportunity).get(bingxi_backend::handlers::crm_handler::get_opportunity),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn put_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
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

/// 默认脱敏四件套断言（复用读路径同一实现的效果锁）：mobile_phone/tel_phone/email 掩码 +
/// address 整键移除；出参任何字符串值都不得含原文 PII。
fn assert_default_masked(lead: &Value, where_label: &str) {
    assert_eq!(
        lead["mobile_phone"],
        json!(MASKED_PHONE),
        "{where_label}：mobile_phone 未掩码"
    );
    assert_eq!(
        lead["tel_phone"],
        json!(MASKED_TEL),
        "{where_label}：tel_phone 未掩码"
    );
    assert_eq!(
        lead["email"],
        json!(MASKED_EMAIL),
        "{where_label}：email 未掩码"
    );
    assert!(
        lead.get("address").is_none(),
        "{where_label}：address 应被整键移除: {lead}"
    );
    let raw = lead.to_string();
    for banned in [A_PHONE, A_TEL, A_EMAIL, A_ADDRESS] {
        assert!(
            !raw.contains(banned),
            "{where_label}：出参含未脱敏个人信息 {banned}"
        );
    }
}

fn assert_raw_lead(lead: &Value, where_label: &str) {
    assert_eq!(
        lead["mobile_phone"],
        json!(A_PHONE),
        "{where_label}：admin 原值契约"
    );
    assert_eq!(
        lead["tel_phone"],
        json!(A_TEL),
        "{where_label}：admin 原值契约"
    );
    assert_eq!(
        lead["email"],
        json!(A_EMAIL),
        "{where_label}：admin 原值契约"
    );
    assert_eq!(
        lead["address"],
        json!(A_ADDRESS),
        "{where_label}：admin 原值契约（address 不得移除）"
    );
}

// ---------------------------------------------------------------------------
// A · 线索：Dept 范围用户更新他人名下线索，写响应必须走默认脱敏（读打码、写原文旁路收口）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dept_user_update_lead_response_is_masked_not_raw_pii() {
    let app = build_lead_app(
        base_state(None).await,
        make_auth(DEPT_UPDATER, DEPT_UPDATER_LOGIN, 2, "dept"),
    );
    // Dept 更新者非归属人（owner=USER_A=50），但可合法更新同部门他人行（check_resource_owner Dept 通过）
    let (status, v) = put_json(&app, "/erp/crm/leads/1", json!({"contact_name": "张三改"})).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "Dept 用户更新同部门他人线索应成功: {v}"
    );
    assert_eq!(
        v["data"]["contact_name"],
        json!("张三改"),
        "更新生效（非只回原文）"
    );
    assert_eq!(v["data"]["owner_id"], json!(USER_A), "更新他人行不改归属人");
    assert_default_masked(&v["data"], "PUT /crm/leads/:id 写响应（Dept 更新他人行）");
}

#[tokio::test]
async fn admin_update_lead_response_keeps_raw_pii() {
    let app = build_lead_app(
        base_state(None).await,
        make_auth(USER_ADMIN, "admin_user", 1, "all"),
    );
    let (status, v) = put_json(
        &app,
        "/erp/crm/leads/1",
        json!({"contact_name": "张三admin改"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin 更新应成功: {v}");
    assert_raw_lead(&v["data"], "admin PUT /crm/leads/:id 写响应");
}

// ---------------------------------------------------------------------------
// A+B · 线索建单成功响应：PII 走默认脱敏，且 owner_name 落真实登录名（不再 format!）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_lead_response_is_masked_and_owner_name_is_real_login() {
    let app = build_lead_app(
        base_state(None).await,
        make_auth(USER_A, CREATOR_LOGIN, 2, "self"),
    );
    // 提供 lead_no 以跳过 PG advisory_xact_lock 取号（sqlite 无该函数），其余走真实建单链
    let (status, v) = post_json(
        &app,
        "/erp/crm/leads",
        json!({
            "lead_no": "LD-NEW-9001",
            "company_name": "新建甲公司",
            "contact_name": "新建联系人",
            "mobile_phone": A_PHONE,
            "tel_phone": A_TEL,
            "email": A_EMAIL,
            "address": A_ADDRESS,
            "lead_status": "new",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "建单应成功: {v}");
    let lead = &v["data"];
    // A：写响应不得整行原文（与非 admin 读路径同源）
    assert_default_masked(lead, "POST /crm/leads 建单写响应");
    // B：owner_name 来自真实登录名（auth.username），不是 format!("用户{id}")
    assert_eq!(
        lead["owner_name"],
        json!(CREATOR_LOGIN),
        "建单 owner_name 应等于传入的真实登录名"
    );
    let owner_name = lead["owner_name"].as_str().unwrap_or_default();
    assert!(
        !owner_name.starts_with("用户"),
        "B 负向锁：owner_name 不得为 format! 造出的展示名，实际: {owner_name}"
    );
}

// ---------------------------------------------------------------------------
// A · 商机：配了 hidden_fields 的角色，更新写响应必须走同一个 filter_fields_batch
// （修复前 update/create/close 写响应完全无字段处理 = 整行原文直出，本用例锁其收口）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn opp_update_response_honors_hidden_fields_shared_with_read_exits() {
    let insert = r#"INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
        allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
        VALUES (1,2,'crm_opportunity','SELF',NULL,'["estimated_amount"]',1,
        '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#;
    let app = build_opp_app(
        base_state(Some(insert)).await,
        make_auth(USER_A, "sales_a", 2, "self"),
    );
    // 归属校验（self：owner==user）通过，更新生效
    let (status, detail) = put_json(
        &app,
        "/erp/crm/opportunities/1",
        json!({"priority": "medium"}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "self 用户更新自己商机应成功: {detail}"
    );
    assert_eq!(detail["data"]["priority"], json!("medium"), "更新应生效");
    assert!(
        detail["data"].get("estimated_amount").is_none(),
        "商机更新写响应应与详情/列表走同一个 filter_fields_batch 移除 hidden 列（修复前整行原文直出）: {}",
        detail["data"]
    );
    // 其余金额列（未列入 hidden）保持原值，证明是精准移除而非清空整行
    assert_eq!(
        detail["data"]["actual_amount"],
        json!("222222"),
        "hidden_fields 仅移除该列，不叠加默认处理（等价于列表既有语义）"
    );
}

#[tokio::test]
async fn admin_opp_update_response_keeps_amounts() {
    let app = build_opp_app(
        base_state(None).await,
        make_auth(USER_ADMIN, "admin_user", 1, "all"),
    );
    let (status, v) = put_json(
        &app,
        "/erp/crm/opportunities/1",
        json!({"priority": "high"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin 更新商机应成功: {v}");
    // admin（无 hidden 配置）→ filter_fields_batch 空操作 → 金额原值保留（既有原值契约不伤）
    assert_eq!(
        v["data"]["estimated_amount"],
        json!("111111"),
        "admin 金额原值契约"
    );
    assert_eq!(
        v["data"]["actual_amount"],
        json!("222222"),
        "admin 金额原值契约"
    );
}

// ---------------------------------------------------------------------------
// B · 商机建单：owner_name 落真实登录名（服务层，用显式 opportunity_no 跳过 PG 取号锁）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_opportunity_owner_name_is_real_login() {
    let state = base_state(None).await;
    let db = state.db.clone();
    // 建一条客户行（create_opportunity 会 find_by_id 校验其存在）
    customer::ActiveModel {
        id: Set(1),
        customer_code: Set("CUS-0001".to_string()),
        customer_name: Set("契约客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(USER_A),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("customers 种子插入失败");

    let svc = CrmService::new(db.clone());
    let opp = svc
        .create_opportunity(
            bingxi_backend::models::dto::crm_dto::CreateOpportunityRequest {
                opportunity_no: Some("OPP-NEW-9001".to_string()),
                opportunity_name: "新建商机".to_string(),
                customer_id: 1,
                lead_id: None,
                opportunity_type: None,
                opportunity_stage: None,
                win_probability: None,
                estimated_amount: None,
                actual_amount: None,
                currency: None,
                expected_close_date: None,
                actual_close_date: None,
                product_ids: None,
                product_names: None,
                product_desc: None,
                priority: None,
                rating: None,
                tags: None,
            },
            USER_A,
            CREATOR_LOGIN,
        )
        .await
        .expect("create_opportunity 应成功（显式商机号跳过取号锁）");
    assert_eq!(
        opp.owner_name, CREATOR_LOGIN,
        "商机 owner_name 应等于传入的真实登录名"
    );
    assert!(
        !opp.owner_name.starts_with("用户"),
        "B 负向锁：owner_name 不得为 format! 造出的展示名，实际: {}",
        opp.owner_name
    );
}

// ---------------------------------------------------------------------------
// C · 源码扫描棘轮（shrink-only）：
//   1. 五个写/读出口不得再退回整行原文直出，必须过对应字段级唯一实现；
//   2. 造展示名零命中集合按**显式文件清单**判定（lead.rs / opp.rs / pool.rs /
//      customer_team_share_service.rs / customer_transfer_approval_service.rs 五份，
//      见函数内注释；最后一份随 to_user_name 透传裁定落地纳入，运行时锁见
//      contract_wave6_crm_transfer_approval_to_user_name_test.rs）。
// ---------------------------------------------------------------------------

#[test]
fn crm_handler_write_and_read_exits_all_route_through_field_permission() {
    let handler = include_str!("../src/handlers/crm_handler.rs");

    // 唯一实现存在
    assert!(
        handler.contains("pub(crate) async fn apply_lead_field_permission"),
        "线索字段级权限唯一实现缺失"
    );
    assert!(
        handler.contains("pub(crate) async fn apply_opportunity_field_permission"),
        "商机字段级权限唯一实现缺失（本轮四/五出口收敛目标）"
    );

    // 取某 handler 的函数体（从 `pub async fn NAME` 到下一个 `pub async fn `）
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

    for name in ["create_lead", "update_lead"] {
        let body = body_of(name);
        assert!(
            body.contains("apply_lead_field_permission("),
            "回潮棘轮：{name} 成功响应未走线索字段级权限唯一实现（整行原文回传 PII 旁路）"
        );
        assert!(
            !body
                .contains("serde_json::to_value(res)?;\n    Ok(Json(ApiResponse::success(value)))"),
            "回潮棘轮：{name} 又出现 to_value(res) 后直接原文返回"
        );
    }

    for name in [
        "list_opportunities",
        "get_opportunity",
        "create_opportunity",
        "update_opportunity",
        "close_opportunity_as_lost",
    ] {
        let body = body_of(name);
        assert!(
            body.contains("apply_opportunity_field_permission("),
            "回潮棘轮：{name} 未走商机字段级权限唯一实现（建单/更新/关单旁路重新分叉）"
        );
    }

    // 导出对行长形状硬校验（fail-closed）：行长 ≠ 列定义数必须拒绝出文件，
    // 原"多余单元格不处理即出文件 / 短行按空值继续 / 回写 push 补位"三种兜底不得回潮
    // （运行时负例见 crm_handler.rs 的 export_row_shape_guard_tests 单测模块）。
    assert!(
        handler.contains("fn validate_export_row_shape"),
        "导出形状硬校验入口缺失（行长与列定义不一致必须整体拒绝）"
    );
    for banned in [
        "该列按空值处理",
        "导出行掩码回写时找不到对应行对象，跳过该行",
        "敏感列未参与处理",
        "None => row.push(value)",
    ] {
        assert!(
            !handler.contains(banned),
            "回潮棘轮：导出字段处理又出现兜底放行分支 → {banned}"
        );
    }
}

#[test]
fn services_crm_must_not_fabricate_owner_name_or_ignore_operator_param() {
    let lead_service = include_str!("../src/services/crm/lead.rs");
    let opp_service = include_str!("../src/services/crm/opp.rs");
    let pool_service = include_str!("../src/services/crm/pool.rs");
    // 看板 #212-B 纳入同一零命中集合：共享落库展示名改由调用方传真实登录名
    let share_service = include_str!("../src/services/crm/customer_team_share_service.rs");
    // to_user_name 透传裁定落地后纳入：被转移人姓名由 transfer_lead 的
    // TransferLeadResult 回传（真实 users.username，零额外查询），不再 format! 造名
    let transfer_service =
        include_str!("../src/services/crm/customer_transfer_approval_service.rs");

    for (name, src) in [
        ("lead.rs", lead_service),
        ("opp.rs", opp_service),
        ("pool.rs", pool_service),
        ("customer_team_share_service.rs", share_service),
        ("customer_transfer_approval_service.rs", transfer_service),
    ] {
        assert!(
            !src.contains("format!(\"用户{}"),
            "回潮棘轮：services/crm/{name} 用 format! 伪造 owner_name/to_user_name 展示名"
        );
        assert!(
            !src.contains("_operator_name"),
            "回潮棘轮：services/crm/{name} 出现被忽略的 _operator_name 形参（真实登录名被丢弃）"
        );
    }
    // 正向锁：create_lead / create_opportunity 落 operator_name（非 user_id 拼接）
    assert!(
        lead_service.contains("let owner_name = operator_name.to_string();"),
        "create_lead 的 owner_name 必须来自 operator_name 形参"
    );
    assert!(
        opp_service.contains("let owner_name = operator_name.to_string();"),
        "create_opportunity 的 owner_name 必须来自 operator_name 形参"
    );
    // 正向锁：审批行 to_user_name 来自 execute_transfer 透传的 TransferLeadResult
    // （真实姓名的单次查询来源，零额外 SELECT；转移失败显式上抛不落空）
    assert!(
        transfer_service.contains("Set(Some(transfer_result.to_user_name))"),
        "回潮棘轮：customer_transfer_approval_service.rs 的 to_user_name 未挂透传来源"
    );
}
