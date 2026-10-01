//! 契约波次 6 · 任务 #202：CRM 公海列表行级 scope + phone/email 掩码锁
//!
//! 根因（修复前实证）：crm_pool_handler.rs 的 list_pool 调
//! `service.list_leads(query, None)` —— services/crm/lead.rs:143-151 中
//! 行级数据权限过滤 `apply_department_scope_with_pool` 仅在
//! `data_scope = Some(ctx)` 时应用，传 None 即整体跳过行级过滤（跨行泄露）；
//! 且出参构造对 mobile_phone/email 原样直通，无 crm_handler::list_leads
//! （#200 定稿）的字段级掩码分支——同一份线索数据，走公海入口就不打码。
//!
//! 修复口径：行级 scope 与字段级掩码均按 crm_handler::list_leads 同构接入
//! （`auth.to_data_scope_context()` + get_role_data_permission 判定源 +
//! utils/field_mask::mask_phone/mask_email），公海侧不另造更松的规则。
//!
//! 覆盖（sqlite 真跑 + 真 HTTP 装配，无伪装断言）：
//! 1. 非 admin（role_id=2、无数据权限行）取公海列表 → mobile_phone 为
//!    mask_phone 形态（前 3 后 4、不含原文中段数字）、email 为 mask_email 形态；
//! 2. 行级 scope 生效：B（user 60，self）看不到 A（owner 50）的公海/私海线索
//!    （修复前 None 绕过行级过滤，A 的公海线索对 B 可见——本用例修复前必红）；
//! 3. admin（role_id=1、roles.code='admin'，判定源 admin_checker.rs:87）→
//!    get_role_data_permission 返回 Ok(Some{allowed:None,hidden:None}) →
//!    filter_fields_batch 空操作且默认打码分支（role_id != 1）不可达 → 原值契约；
//! 4. 源码扫描锁（shrink-only 棘轮）：crm_pool_handler.rs 中
//!    `list_leads(query, None` 命中数必须为 0。
//!
//! 覆盖边界（声明）：服务层/utils 在本任务禁改清单内。
//! `apply_department_scope_with_pool`（utils/data_scope.rs:218-236）的公海放行
//! 仅对 Dept 分支生效（pool 条件 AND 进 dept 条件；公海入口自带
//! lead_status='pool' 谓词，故 Dept 用例结果集不受影响），而 Self 分支无任何
//! 公海放行——self 销售在公海列表只能看到自己回收进公海的线索，看不到他人
//! 公海线索，与 RLS 策略"公海行放行"意图（data_scope.rs:212-214 注释）不一致。
//! 该口径需改 utils/data_scope.rs 方能量化落地，用例 5 以 #[ignore] 记录目标
//! 契约，不伪装成已通过端到端。

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
use bingxi_backend::handlers::crm_pool_handler::list_pool;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 脚手架（与 contract_wave6_crm_lead_mask_test.rs 同款：种子同构、缓存规避）
// ---------------------------------------------------------------------------

const RAW_PHONE: &str = "13812348888";
const RAW_EMAIL: &str = "alice@example.com";

fn make_auth(user_id: i32, role_id: i32, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave6_pool_user_{user_id}"),
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

/// 与 models/crm_lead.rs::Model 逐列对应（表 crm_lead，含 department_id）
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

/// 与 models/role.rs::Model 对应（is_admin_role 判定源，admin_checker.rs:86-87）
const CREATE_ROLES: &str = r#"CREATE TABLE roles (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, code TEXT NOT NULL,
    description TEXT, permissions TEXT, is_system INTEGER NOT NULL,
    data_scope TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

/// 与 models/data_permission.rs::Model 对应（表 data_permissions；
/// 公海掩码用例不配置权限行——判定走 get_role_data_permission → Ok(None) 分支）
const CREATE_DATA_PERMISSIONS: &str = r#"CREATE TABLE data_permissions (
    id INTEGER PRIMARY KEY, role_id INTEGER NOT NULL,
    resource_type TEXT NOT NULL, scope_type TEXT NOT NULL,
    custom_condition TEXT, allowed_fields TEXT, hidden_fields TEXT,
    is_enabled INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

/// 建表 + 种子：
/// - roles：1=admin、2=sales——每个用例同构种子，保证 admin_checker 全局缓存
///   （ADMIN_ROLE_CACHE 按 role_id 进程级缓存）在任何执行顺序下取值一致；
/// - crm_lead：
///   id=1 A（owner 50）的公海线索，携带原始手机号/邮箱/地址；
///   id=2 A（owner 50）的私海线索（status='new'）；
///   id=3 B（owner 60）的公海线索。
async fn seeded_state() -> AppState {
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
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,mobile_phone,email,address,owner_id,owner_name,department_id,
             created_at,updated_at) VALUES
             (1,'LD001','website','pool','甲公司','张三','{RAW_PHONE}','{RAW_EMAIL}',
              '河北省邢台市某某路 1 号',50,'销售甲',1,'2026-09-01T00:00:00Z','2026-09-01T00:00:00Z'),
             (2,'LD002','ad','new','乙公司','李四','13711112222','carol@example.com',
              '地址乙',50,'销售甲',1,'2026-09-01T00:00:00Z','2026-09-01T00:00:00Z'),
             (3,'LD003','referral','pool','丙公司','王五','13998887777','bob@example.com',
              '地址丙',60,'销售乙',1,'2026-09-02T00:00:00Z','2026-09-02T00:00:00Z')"
        ),
    )
    .await;

    let mut state = AppState::default();
    state.db = Arc::new(db);
    // data_permission_service 内部持有独立 db 引用，必须与 state.db 同源重建，
    // 否则 get_role_data_permission 落在 Disconnected 连接上恒 Err（假绿）。
    state.data_permission_service = Arc::new(DataPermissionService::new(state.db.clone()));
    state
}

fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/pool", get(list_pool))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
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

/// 信封锚点：ApiResponse::success → {code,data}；list_pool 的 data 内层为
/// {items:[...],total,page,page_size}（handlers/crm_pool_handler.rs 响应构造）。
fn pool_items(v: &Value) -> &Vec<Value> {
    v["data"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("公海列表形状漂移（期望 data.items 数组）: {v}"))
}

fn ids_of(items: &[Value]) -> Vec<i64> {
    items
        .iter()
        .map(|i| i["id"].as_i64().expect("items 缺 id 键"))
        .collect()
}

// ---------------------------------------------------------------------------
// 1) 非 admin 无数据权限行：公海入口 phone/email 必须与正常入口同源掩码
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_admin_pool_list_masks_mobile_phone_and_email() {
    let app = build_app(seeded_state().await, make_auth(50, 2, "self"));
    let (status, v) = get_json(&app, "/erp/crm/pool?page=1&page_size=20").await;
    assert_eq!(status, StatusCode::OK, "公海列表 200 契约: {v}");
    let items = pool_items(&v);
    assert_eq!(items.len(), 1, "self 范围应仅命中本人公海线索: {v}");
    let lead = &items[0];
    assert_eq!(lead["id"], json!(1));

    // 核心回归点：修复前公海入口无任何掩码分支 → 原值直通
    assert_eq!(
        lead["mobile_phone"],
        json!("138****8888"),
        "mask_phone（前3后4，utils/field_mask.rs:5）未生效——公海入口手机号泄露回潮"
    );
    let phone = lead["mobile_phone"].as_str().unwrap();
    assert!(!phone.contains("1234"), "掩码后仍含原文中段数字: {phone}");
    assert_ne!(phone, RAW_PHONE);
    assert_eq!(
        lead["email"],
        json!("a***@example.com"),
        "mask_email（首字母+***，utils/field_mask.rs:16）未生效"
    );
}

// ---------------------------------------------------------------------------
// 2) 行级 scope 生效：B 不得看到 A 的任何线索（公海入口曾传 None 绕过过滤）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pool_list_honors_row_level_scope_against_other_owner_leads() {
    let app = build_app(seeded_state().await, make_auth(60, 2, "self"));
    let (status, v) = get_json(&app, "/erp/crm/pool?page=1&page_size=20").await;
    assert_eq!(status, StatusCode::OK, "公海列表 200 契约: {v}");
    let ids = ids_of(pool_items(&v));

    // 修复前实证泄露点：list_leads(…, None) 跳过行级过滤，A 的公海线索 id=1 可见
    assert!(
        !ids.contains(&1),
        "行级 scope 未接入：B 看到了 A 的公海线索: {v}"
    );
    // 私海线索（status='new'）不得出现在公海入口（谓词+scope 双保险）
    assert!(!ids.contains(&2), "A 的私海线索出现在公海列表: {v}");
    assert!(ids.contains(&3), "B 本人公海线索应可见: {v}");
    assert_eq!(ids.len(), 1, "self 范围公海列表应仅含本人行: {v}");

    // B 的行同样被掩码（掩码判定与行级过滤双缺陷同源入口，各自独立锁）
    let lead_b = &pool_items(&v)[0];
    assert_eq!(lead_b["mobile_phone"], json!("139****7777"));
    assert_eq!(lead_b["email"], json!("b***@example.com"));
}

// ---------------------------------------------------------------------------
// 3) admin（role_id=1、roles.code='admin'）：保持原值（既有 RBAC 行为契约不改）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_pool_list_keeps_raw_values() {
    let app = build_app(seeded_state().await, make_auth(50, 1, "all"));
    let (status, v) = get_json(&app, "/erp/crm/pool?page=1&page_size=20").await;
    assert_eq!(status, StatusCode::OK, "admin 公海列表 200 契约: {v}");
    let items = pool_items(&v);
    let ids = ids_of(items);
    assert!(
        ids.contains(&1) && ids.contains(&3),
        "all 范围应见全部公海行: {v}"
    );

    let lead = items
        .iter()
        .find(|i| i["id"] == json!(1))
        .expect("id=1 应在 admin 公海列表");
    assert_eq!(lead["mobile_phone"], json!(RAW_PHONE), "admin 原值契约");
    assert_eq!(lead["email"], json!(RAW_EMAIL), "admin 原值契约");
    assert_eq!(
        lead["address"],
        Value::Null,
        "公海出参为挑选字段构造，本就不含 address 键"
    );
}

// ---------------------------------------------------------------------------
// 4) 源码扫描锁（shrink-only 棘轮）：公海 handler 禁止再以省略 scope 调 list_leads
// ---------------------------------------------------------------------------

#[test]
fn pool_handler_never_calls_list_leads_without_scope() {
    let src = include_str!("../src/handlers/crm_pool_handler.rs");
    let hits = src.matches("list_leads(query, None").count();
    assert_eq!(
        hits, 0,
        "回潮棘轮：crm_pool_handler.rs 出现 list_leads(…, None)，行级数据权限被绕过"
    );
    assert!(
        src.contains("list_leads(query, Some(&data_scope_ctx))"),
        "公海列表必须以 Some(ctx) 接入行级 scope（与 crm_handler::list_leads 同口径）"
    );
}

// ---------------------------------------------------------------------------
// 5) 覆盖边界（#[ignore]）：Self 分支的公海放行需 utils/data_scope.rs 配合
//    现状：build_department_scope_condition 的 Self 分支仅 owner_id=user_id，
//    无「lead_status='pool' 放行」OR 支（utils/data_scope.rs:140-141）；
//    与 RLS 策略意图（data_scope.rs:212-214 注释）不一致——self 销售在公海页
//    看不到他人公海线索，"领取"业务受损。本用例记录目标契约（修复后取消
//    ignore 即成为回归锁），当前不通过，不伪装成端到端。
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "覆盖边界：Self 分支公海放行需先改 utils/data_scope.rs（本任务禁改清单），落地后取消 ignore"]
async fn self_scope_user_should_see_others_pool_leads() {
    let app = build_app(seeded_state().await, make_auth(60, 2, "self"));
    let (_, v) = get_json(&app, "/erp/crm/pool?page=1&page_size=20").await;
    let ids = ids_of(pool_items(&v));
    assert!(
        ids.contains(&1),
        "目标契约：公海行对所有可见用户放行（RLS 同口径），B 应看到 A 的公海线索: {v}"
    );
    assert!(
        !ids.contains(&2),
        "私海线索仍必须被行级 scope + 状态谓词挡住: {v}"
    );
}
