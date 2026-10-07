//! 契约波次 6 · 客户域读出口与标准客户入口的出参一致性锁
//!
//! 根因（"同一份数据、不同入口出参不一致"= 打码策略被旁路的最后一族收口）：
//! 1. `crm_customer_handler` 增强页三个**读**出口（GET /crm/customers/enhanced 列表、
//!    GET /crm/customers/enhanced/:id 详情、GET /crm/customers/:id/contacts 联系人列表）
//!    整行 `to_value` 原文直出；同一批行走同域写响应是打码的 → 换入口即拿到原文 PII。
//!    收口（主编排终局裁定口径）：三读出口统一挂客户域唯一实现
//!    `apply_customer_field_permission`——同页列表/详情/联系人/更新四形状都走
//!    resource_type=customer 权限行 + `CrmService::mask_customer_pii_defaults` 默认掩码，
//!    **不引用**线索域 crm_lead 权限行（两域权限判定语义不同不强行统一）；掩码列集合
//!    全仓唯一 = `utils/field_mask`，非 admin `address` 整键移除。列表数组定位复用
//!    `paginated_list_array_mut`（兼容 `data`/`list` 双键 + 定位失败显式 error，不静默）。
//! 2. `customer_handler`（标准客户入口）读（GET list/detail）与写（POST/PUT）出口收口
//!    到同一客户域唯一实现（同资源同形状走同一行配置判定，不保留第二套内联分支；
//!    默认脱敏分支与旧 `mask_contact_fields_for_role` 直调同一权威列集合，行为只增
//!    address 移除 = 更严方向）。
//! 3. convert 出口形状锁：`convert_lead_to_customer`/`convert_opportunity_to_order`
//!    服务层出参为摘要键（id+编号+名称），不含任何掩码列；整行 Model 直出即回潮，
//!    本文件负断言第一时间变红（不在出参侧挂字段权限函数——摘要形状本身即收口）。
//!
//! 前端消费面证据（本波改动的可见性影响，均不构成对原文的功能依赖）：
//! - `frontend/src/views/crm/tabs/CustomerListTab.vue:129/258-271`：contact_phone/
//!   contact_email 仅表格展示与新建表单字段；编辑弹窗校验规则 `/^1[3-9]\d{9}$/`
//!   （:459-464）会拒绝掩码串回填覆盖，不存在"静默把掩码写回库"的通路；
//! - `frontend/e2e/crm/05-assign-share-merge.spec.ts:327-334`：联系人回读只断言 `name`
//!   键，不断言电话/邮箱原文；
//! - `frontend/e2e/flow/22-crm-full.spec.ts:199`：GET /crm/customers/enhanced 仅健康探针。
//! → 非 admin 在增强页开始看到掩码值，不破坏任何流程；本仓亦不以"让断言过"放行原文。
//!
//! 通道（路线一， 判责）：全部用例经 `test_common::setup_test_db` 连已迁移
//! PostgreSQL 真跑；**表结构唯一来源 = backend/migration**，本文件不再自建任何 DDL。
//! 种子按真表逐列对齐：users/customers/crm_lead/customer_contacts 属会被清空的业务
//! 表（裁定 R1 自种子合法父行）；roles 属迁移种子参照表（id=1 code='admin'、
//! id=2 manager 已由迁移播种），不得再插。

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
use bingxi_backend::handlers::crm_customer_handler::{
    get_customer as enhanced_get_customer, list_contacts, list_customers as enhanced_list_customers,
};
use bingxi_backend::handlers::customer_handler::{
    create_customer, get_customer, list_customers, update_customer,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const OWNER: i32 = 50;
const ADMIN: i32 = 70;
const OWNER_LOGIN: &str = "sales_a";
const ADMIN_LOGIN: &str = "admin_user";

const C_PHONE: &str = "13812348888";
const C_TEL: &str = "03191234567";
const C_EMAIL: &str = "alice@example.com";
const C_ADDRESS: &str = "河北省邢台市某某路 1 号";
const MASKED_PHONE: &str = "138****8888";
const MASKED_TEL: &str = "031****4567";
const MASKED_EMAIL: &str = "a***@example.com";
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
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

async fn base_state(permissions: Option<&str>) -> AppState {
    // 路线一：连已迁移 PostgreSQL 并清空业务表（TRUNCATE … RESTART IDENTITY，
    // 自增 ID 从 1 起）；表结构唯一来源 = backend/migration，本文件不再自建 DDL。
    let db = test_common::setup_test_db().await;
    // users：归属人/admin 父行（裁定 R1 自种子；department_id=1 对齐迁移种子部门，
    // 供 trg_*_dept 触发器回填冗余列）。布尔列按 PG 真类型写 TRUE/FALSE
    // （sqlite 时代的 1/0 在 PG 上是类型错误）。roles 不再插：属迁移种子参照表
    // （id=1 code='admin'、id=2 manager 已由 m0001 播种且不参与清空）。
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'admin_user','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // 归属 OWNER 的线索行（增强页列表/详情出参行），携带四列明文 PII。
    // department_id 不手写：迁移触发器 trg_crm_lead_dept 按 owner 的
    // users.department_id 自动维护（种子/应用侧写入会被覆盖）。
    exec(
        &db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,mobile_phone,tel_phone,email,address,owner_id,owner_name,
             priority,created_at,updated_at) VALUES
             (1,'LD001','website','new','甲公司','张三','{C_PHONE}','{C_TEL}',
              '{C_EMAIL}','{C_ADDRESS}',{OWNER},'销售甲','low',
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    // 归属 OWNER 的客户主数据行（标准客户入口出参行）。逐列对齐真表：
    // credit_limit 为 DECIMAL(12,2)（sqlite 时代误写成 TEXT '0'）、payment_terms
    // 为 INTEGER（模型非 Option，必须给值）、status/customer_type 为权威词表 token。
    exec(
        &db,
        &format!(
            "INSERT INTO customers (id,customer_code,customer_name,contact_person,
             contact_phone,contact_email,address,credit_limit,payment_terms,status,
             customer_type,owner_id,created_at,updated_at) VALUES
             (1,'CUS-0001','甲客户','张三','{C_PHONE}','{C_EMAIL}','{C_ADDRESS}',
              0,30,'active','retail',{OWNER},
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    // 客户 1 的联系人行（联系人列表出参行；is_primary 为 BOOLEAN）
    exec(
        &db,
        &format!(
            "INSERT INTO customer_contacts (id,customer_id,name,phone,email,is_primary,
             created_at,updated_at) VALUES
             (1,1,'联系人甲','{CONTACT_PHONE}','{CONTACT_EMAIL}',TRUE,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    // 显式 id 种子后对齐自增序列：本文件 POST /crm/customers 经 SERIAL 建行，
    // RESTART IDENTITY 已把序列复位到 1，不对齐会撞上种子行 id=1（PK 冲突假红）。
    exec(
        &db,
        "SELECT setval(pg_get_serial_sequence('customers','id'),
                       (SELECT COALESCE(MAX(id),1) FROM customers))",
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

/// 增强页三读出口按真实挂载路径（routes/crm.rs::crm_customers()）建路由
fn build_enhanced_read_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/customers/enhanced", get(enhanced_list_customers))
        .route(
            "/erp/crm/customers/enhanced/{id}",
            get(enhanced_get_customer),
        )
        .route("/erp/crm/customers/{id}/contacts", get(list_contacts))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

/// 标准客户入口四出口按真实挂载路径（routes/crm.rs::customers()）建路由
fn build_standard_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route(
            "/erp/crm/customers",
            get(list_customers).post(create_customer),
        )
        .route(
            "/erp/crm/customers/{id}",
            get(get_customer).put(update_customer),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn request_json(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    let req = match body {
        Some(v) => builder.body(Body::from(v.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
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

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    request_json(app, Method::GET, uri, None).await
}

async fn post_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    request_json(app, Method::POST, uri, Some(body)).await
}

async fn put_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    request_json(app, Method::PUT, uri, Some(body)).await
}

/// 出参字符串不得出现任何原文 PII（无论键名、无论嵌套）
fn assert_no_raw_pii(value: &Value, where_label: &str) {
    let raw = value.to_string();
    for banned in [
        C_PHONE,
        C_TEL,
        C_EMAIL,
        C_ADDRESS,
        CONTACT_PHONE,
        CONTACT_EMAIL,
    ] {
        assert!(
            !raw.contains(banned),
            "{where_label}：响应体含未脱敏个人信息 {banned}（原文直通旁路）\n出参: {raw}"
        );
    }
}

/// 增强页线索行默认脱敏断言：mobile_phone/tel_phone/email **掩码保留键**（非移除），
/// address 整键移除——客户侧默认脱敏（mask_customer_pii_defaults）的既有掩码契约
fn assert_lead_shape_masked(lead: &Value, where_label: &str) {
    assert_eq!(
        lead["mobile_phone"],
        json!(MASKED_PHONE),
        "{where_label}：mobile_phone 应为掩码值（掩码保留键契约，非整键移除）"
    );
    assert_eq!(
        lead["tel_phone"],
        json!(MASKED_TEL),
        "{where_label}：tel_phone 应为掩码值"
    );
    assert_eq!(
        lead["email"],
        json!(MASKED_EMAIL),
        "{where_label}：email 应为掩码值"
    );
    assert!(
        lead.get("address").is_none(),
        "{where_label}：address 应被整键移除: {lead}"
    );
    assert_no_raw_pii(lead, where_label);
}

fn assert_lead_shape_raw(lead: &Value, where_label: &str) {
    assert_eq!(
        lead["mobile_phone"],
        json!(C_PHONE),
        "{where_label}：admin 原值契约"
    );
    assert_eq!(
        lead["tel_phone"],
        json!(C_TEL),
        "{where_label}：admin 原值契约"
    );
    assert_eq!(
        lead["email"],
        json!(C_EMAIL),
        "{where_label}：admin 原值契约"
    );
    assert_eq!(
        lead["address"],
        json!(C_ADDRESS),
        "{where_label}：admin address 原值契约（不得移除）"
    );
}

// ---------------------------------------------------------------------------
// 1 · 增强页列表（GET /crm/customers/enhanced）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_admin_enhanced_list_is_masked_not_raw() {
    let app = build_enhanced_read_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = get_json(&app, "/erp/crm/customers/enhanced?page=1&page_size=10").await;
    assert_eq!(status, StatusCode::OK, "self 用户看自己线索应成功: {v}");
    let rows = v["data"]["data"]
        .as_array()
        .expect("list_leads 分页出参 data 数组缺失");
    assert!(!rows.is_empty(), "种子线索应对归属人可见");
    assert_lead_shape_masked(&rows[0], "GET /crm/customers/enhanced 列表行");
    assert_no_raw_pii(&v["data"], "GET /crm/customers/enhanced 整体响应");
}

#[tokio::test]
async fn admin_enhanced_list_keeps_raw_pii() {
    let app = build_enhanced_read_app(
        base_state(None).await,
        make_auth(ADMIN, ADMIN_LOGIN, Some(1), "all"),
    );
    let (status, v) = get_json(&app, "/erp/crm/customers/enhanced?page=1&page_size=10").await;
    assert_eq!(status, StatusCode::OK, "admin 列表应成功: {v}");
    let rows = v["data"]["data"].as_array().expect("data 数组缺失");
    assert_lead_shape_raw(&rows[0], "admin GET /crm/customers/enhanced");
}

// ---------------------------------------------------------------------------
// 2 · 增强页详情（GET /crm/customers/enhanced/:id）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_admin_enhanced_detail_is_masked_and_role_id_missing_fail_closed() {
    let app = build_enhanced_read_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = get_json(&app, "/erp/crm/customers/enhanced/1").await;
    assert_eq!(status, StatusCode::OK, "归属人详情应成功: {v}");
    assert_lead_shape_masked(&v["data"], "GET /crm/customers/enhanced/:id 详情");

    // role_id 缺失 → fail-closed 默认脱敏（与本页列表/更新/联系人同一口径）
    let app_none = build_enhanced_read_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, None, "self"),
    );
    let (status, v) = get_json(&app_none, "/erp/crm/customers/enhanced/1").await;
    assert_eq!(status, StatusCode::OK, "role_id 缺失不影响详情本身: {v}");
    assert_lead_shape_masked(&v["data"], "role_id 缺失（fail-closed）详情出参");
}

#[tokio::test]
async fn admin_enhanced_detail_keeps_raw_pii() {
    let app = build_enhanced_read_app(
        base_state(None).await,
        make_auth(ADMIN, ADMIN_LOGIN, Some(1), "all"),
    );
    let (status, v) = get_json(&app, "/erp/crm/customers/enhanced/1").await;
    assert_eq!(status, StatusCode::OK, "admin 详情应成功: {v}");
    assert_lead_shape_raw(&v["data"], "admin GET /crm/customers/enhanced/:id");
}

// ---------------------------------------------------------------------------
// 3 · 判定源锁定（主编排裁定口径）：增强页读出口只吃 resource_type=customer 的
//     权限行，不引用线索域 crm_lead 行配置——两域权限判定语义不同，不强行统一。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn enhanced_reads_are_driven_by_customer_rows_not_crm_lead_rows() {
    // 配 crm_lead 行（hidden=mobile_phone）：按裁定增强页**不**读取该配置，
    // mobile_phone 应保持"掩码保留键"的默认脱敏形态（若被整键移除即判定源漂移回潮）
    let lead_row = r#"INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
        allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
        VALUES (1,2,'crm_lead','SELF',NULL,'["mobile_phone"]',TRUE,
        '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#;
    let app = build_enhanced_read_app(
        base_state(Some(lead_row)).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = get_json(&app, "/erp/crm/customers/enhanced/1").await;
    assert_eq!(status, StatusCode::OK, "详情应成功: {v}");
    assert_eq!(
        v["data"]["mobile_phone"],
        json!(MASKED_PHONE),
        "增强页不得引用 crm_lead 权限行（裁定口径）：mobile_phone 应维持掩码保留键而非被该行移除"
    );

    // 配 customer 行（hidden=phone）：联系人读出口整键移除 phone（同一 filter_fields_batch）
    let customer_row = r#"INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
        allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
        VALUES (2,2,'customer','SELF',NULL,'["phone"]',TRUE,
        '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#;
    let app2 = build_enhanced_read_app(
        base_state(Some(customer_row)).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = get_json(&app2, "/erp/crm/customers/1/contacts").await;
    assert_eq!(status, StatusCode::OK, "联系人列表应成功: {v}");
    let rows = v["data"].as_array().expect("数组缺失");
    assert!(
        rows[0].get("phone").is_none(),
        "customer 权限行 hidden 必须在联系人读出口生效（同一判定源/filter_fields_batch）: {}",
        rows[0]
    );
    // 叠加语义：hidden 只负责删掉 phone 键，未被覆盖的 email 仍是**掩码值**——
    // 本域默认掩码对非 admin 恒定执行（改造前标准入口 customer_handler.rs 即无条件掩码），
    // 配了权限行不得把原文放回来，否则两个入口对同一角色结果不一致即旁路。
    assert_eq!(
        rows[0]["email"],
        json!(MASKED_CONTACT_EMAIL),
        "默认掩码叠加在权限行过滤之上：hidden 仅删键，不得还原原文"
    );
}

// ---------------------------------------------------------------------------
// 4 · 联系人列表（GET /crm/customers/:id/contacts）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_admin_contacts_list_is_masked() {
    let app = build_enhanced_read_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    let (status, v) = get_json(&app, "/erp/crm/customers/1/contacts").await;
    assert_eq!(status, StatusCode::OK, "联系人列表应成功: {v}");
    let rows = v["data"].as_array().expect("联系人列表出参应为数组");
    assert!(!rows.is_empty(), "种子联系人应返回");
    assert_eq!(
        rows[0]["phone"],
        json!(MASKED_CONTACT_PHONE),
        "联系人 phone 应为掩码值（掩码保留键契约）"
    );
    assert_eq!(
        rows[0]["email"],
        json!(MASKED_CONTACT_EMAIL),
        "联系人 email 应为掩码值"
    );
    assert_eq!(rows[0]["name"], json!("联系人甲"), "非 PII 列不受影响");
    assert_no_raw_pii(&v["data"], "GET /crm/customers/:id/contacts");
}

#[tokio::test]
async fn admin_contacts_list_keeps_raw_pii() {
    let app = build_enhanced_read_app(
        base_state(None).await,
        make_auth(ADMIN, ADMIN_LOGIN, Some(1), "all"),
    );
    let (status, v) = get_json(&app, "/erp/crm/customers/1/contacts").await;
    assert_eq!(status, StatusCode::OK, "admin 联系人列表应成功: {v}");
    let rows = v["data"].as_array().expect("数组缺失");
    assert_eq!(rows[0]["phone"], json!(CONTACT_PHONE), "admin 原值契约");
    assert_eq!(rows[0]["email"], json!(CONTACT_EMAIL), "admin 原值契约");
}

// ---------------------------------------------------------------------------
// 5 · 标准客户入口（customer_handler）：读写出口同一客户域唯一实现
// ---------------------------------------------------------------------------

#[tokio::test]
async fn standard_customer_read_and_update_exits_no_raw_pii() {
    let app = build_standard_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );

    // 详情：无权限行角色在查询层按默认隐藏列下推（contact_phone 可能整列缺失），
    // 出参若仍带该键则必须为掩码值；两种形态都不得出现原文
    let (status, v) = get_json(&app, "/erp/crm/customers/1").await;
    assert_eq!(status, StatusCode::OK, "归属人详情应成功: {v}");
    match v["data"].get("contact_phone") {
        None | Some(Value::Null) => {}
        Some(phone) => assert_eq!(
            phone,
            &json!(MASKED_PHONE),
            "GET /crm/customers/:id contact_phone 出现即须为掩码值（掩码保留键契约）"
        ),
    }
    assert_no_raw_pii(&v["data"], "GET /crm/customers/:id 详情");

    // 列表：同一实现批量分支，响应体不得含原文
    let (status, v) = get_json(&app, "/erp/crm/customers?page=1&page_size=10").await;
    assert_eq!(status, StatusCode::OK, "self 列表应成功: {v}");
    assert_no_raw_pii(&v["data"], "GET /crm/customers 列表");

    // 更新写响应：客户侧唯一实现默认分支=掩码保留键 + 非 admin 移除 address
    let (status, v) = put_json(
        &app,
        "/erp/crm/customers/1",
        json!({"customer_name": "甲客户标准改名"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "归属人更新应成功: {v}");
    let data = &v["data"];
    assert_eq!(
        data["customer_name"],
        json!("甲客户标准改名"),
        "更新须真实生效"
    );
    assert_eq!(
        data["contact_phone"],
        json!(MASKED_PHONE),
        "PUT /crm/customers/:id 写响应 contact_phone 应为掩码值"
    );
    assert_eq!(
        data["contact_email"],
        json!(MASKED_EMAIL),
        "PUT /crm/customers/:id 写响应 contact_email 应为掩码值"
    );
    assert!(
        data.get("address").is_none(),
        "PUT /crm/customers/:id 写响应 address 应整键移除: {data}"
    );
    assert_no_raw_pii(data, "PUT /crm/customers/:id 写响应");
}

#[tokio::test]
async fn standard_customer_create_response_is_masked_not_raw() {
    let app = build_standard_app(
        base_state(None).await,
        make_auth(OWNER, OWNER_LOGIN, Some(2), "self"),
    );
    // 显式 customer_code 跳过 PG 取号锁（与既有线索/增强建档测试同一做法）
    let (status, v) = post_json(
        &app,
        "/erp/crm/customers",
        json!({
            "customer_code": "CUS-HTTP-9101",
            "customer_name": "新建契约客户",
            "contact_person": "李四",
            "contact_phone": C_PHONE,
            "contact_email": C_EMAIL,
            "address": C_ADDRESS,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "创建客户应成功: {v}");
    let data = &v["data"];
    assert_eq!(
        data["customer_name"],
        json!("新建契约客户"),
        "创建须真实落库回传"
    );
    assert_eq!(
        data["contact_phone"],
        json!(MASKED_PHONE),
        "POST /crm/customers 写响应 contact_phone 应为掩码值"
    );
    assert!(
        data.get("address").is_none(),
        "POST /crm/customers 写响应 address 应整键移除: {data}"
    );
    assert_no_raw_pii(data, "POST /crm/customers 写响应");
}

#[tokio::test]
async fn admin_standard_customer_read_and_write_keep_raw() {
    let app = build_standard_app(
        base_state(None).await,
        make_auth(ADMIN, ADMIN_LOGIN, Some(1), "all"),
    );
    let (status, v) = get_json(&app, "/erp/crm/customers/1").await;
    assert_eq!(status, StatusCode::OK, "admin 详情应成功: {v}");
    assert_eq!(
        v["data"]["contact_phone"],
        json!(C_PHONE),
        "admin 读出口原值契约"
    );

    let (status, v) = put_json(
        &app,
        "/erp/crm/customers/1",
        json!({"customer_name": "甲客户admin改"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin 更新应成功: {v}");
    assert_eq!(
        v["data"]["contact_phone"],
        json!(C_PHONE),
        "admin 写响应原值契约"
    );
    assert_eq!(
        v["data"]["address"],
        json!(C_ADDRESS),
        "admin 写响应 address 原值契约（不得移除）"
    );
}

// ---------------------------------------------------------------------------
// 6 · 源码扫描棘轮：本波收口出口不得退回整行原文直出
// ---------------------------------------------------------------------------

fn split_body(src: &str, name: &str) -> String {
    src.split(&format!("pub async fn {name}"))
        .nth(1)
        .unwrap_or_else(|| panic!("{name} 定义缺失"))
        .split("pub async fn ")
        .next()
        .unwrap_or_else(|| panic!("{name} 函数体边界缺失"))
        .to_string()
}

#[test]
fn enhanced_read_and_standard_customer_exits_ratchet() {
    let enhanced = include_str!("../src/handlers/crm_customer_handler.rs");

    // 增强页三读出口全部挂客户域唯一实现（同页判定源一致，裁定口径）
    for name in ["list_customers", "get_customer", "list_contacts"] {
        let body = split_body(enhanced, name);
        assert!(
            body.contains("apply_customer_field_permission("),
            "回潮棘轮：增强页 {name} 读出口又整行原文直出（未挂客户域唯一实现）"
        );
    }
    let contacts_body = split_body(enhanced, "list_contacts");
    assert!(
        contacts_body.contains("tracing::error!"),
        "list_contacts 形状漂移（非数组）必须显式报错，不得静默跳过字段权限"
    );

    // 标准客户入口（终局口径：客户域读写出口统一权限版）：四出口全部挂客户域唯一实现；
    // 默认脱敏分支掩码等价旧内联 mask_contact_fields_for_role（保留键），另加非 admin
    // address 整键移除=只更严不放松；掩码列集合全仓唯一=utils/field_mask
    let standard = include_str!("../src/handlers/customer_handler.rs");
    for name in [
        "create_customer",
        "update_customer",
        "list_customers",
        "get_customer",
    ] {
        let body = split_body(standard, name);
        assert!(
            body.contains("apply_customer_field_permission("),
            "回潮棘轮：customer_handler::{name} 未挂客户域字段级权限唯一实现（整行原文直出/双口径分叉）"
        );
    }
    // 增强页列表定位复用共享数组定位函数（data/list 双键 + 定位失败显式 error）
    let enhanced_list_body = split_body(enhanced, "list_customers");
    assert!(
        enhanced_list_body.contains("paginated_list_array_mut("),
        "增强页列表必须复用 paginated_list_array_mut（data/list 双键兼容 + 不静默）"
    );
}

#[test]
fn convert_exits_output_summary_fields_only() {
    // convert 出口形状锁：服务层只回传摘要键（id+编号+名称），出参侧禁止整行原文直出
    // （也不在出参侧挂字段权限函数——摘要形状本身即收口）
    let lead_service = include_str!("../src/services/crm/lead.rs");
    let convert_body = split_body(lead_service, "convert_lead_to_customer");
    for key in ["customer_id", "customer_code", "customer_name"] {
        assert!(
            convert_body.contains(&format!("\"{key}\"")),
            "convert_lead 出参必须包含摘要键 {key}（前端转化流程消费）"
        );
    }
    for banned in [
        "contact_phone",
        "contact_email",
        "address",
        "mobile_phone",
        "tel_phone",
        "email",
    ] {
        assert!(
            !convert_body.contains(&format!("\"{banned}\"")),
            "convert_lead 出参禁止出现掩码列 {banned}（整行原文直出旁路回潮）"
        );
    }

    let opp_service = include_str!("../src/services/crm/opp.rs");
    let convert_opp_body = split_body(opp_service, "convert_opportunity_to_order");
    assert!(
        convert_opp_body.contains("\"order_id\"") && convert_opp_body.contains("\"order_no\""),
        "商机转单出参必须是 id+编号摘要"
    );
    for banned in [
        "contact_phone",
        "contact_person",
        "shipping_address",
        "billing_address",
    ] {
        assert!(
            !convert_opp_body.contains(&format!("\"{banned}\"")),
            "商机转单出参禁止出现订单行掩码/地址列 {banned}（整行直出旁路回潮）"
        );
    }

    // handler 侧：convert 两出口不挂字段权限函数（拍板口径），形状由服务层摘要保证
    let handler = include_str!("../src/handlers/crm_handler.rs");
    for name in ["convert_lead", "convert_opportunity_to_order"] {
        let body = split_body(handler, name);
        assert!(
            !body.contains("apply_customer_field_permission(")
                && !body.contains("apply_lead_field_permission("),
            "convert 出口 {name} 不挂字段权限函数（拍板口径：形状收口在服务层摘要）"
        );
    }
}
