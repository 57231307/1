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
//! 覆盖（真 PostgreSQL 真跑 + 真 HTTP 装配，无伪装断言）：
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
//! 覆盖边界（已落地，用户 2026-10-02 拍板 ②）：
//! `apply_department_scope_with_pool`（utils/data_scope.rs）对 crm_lead 采用
//! `PoolVisibility::Open`——公海行可见性与归属条件相互独立（OR），与 DB 层
//! RLS USING 的独立公海 OR 支（rls_dept/mod.rs:195-207）同形态。
//! 修复前 Dept 分支为 `(self ∪ dept) AND 公海条件`、Self 分支无公海放行，
//! self 销售在公海列表看不到他人公海线索。用例 1/2 按新语义断言可见集，
//! 用例 5（原 #[ignore] 记录该边界的目标契约）已取消 ignore 成为常跑回归锁；
//! 私海行（lead_status≠'pool'）仍严格受行级 scope 约束（用例 2 锁）。
//!
//! 通道（路线一，#4669 判责）：用例经 `test_common::setup_test_db()` 连已迁移
//! PostgreSQL 真跑；表结构唯一来源 = backend/migration，不再自建 DDL。
//! users 50/60 为自种子父行（trg_crm_lead_dept 触发器按 owner 回填冗余部门列，
//! Dept 用例的 department_id=1 覆盖由此成立）；roles 为迁移种子参照表，不再插。

mod test_common;

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
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

/// 种子：
/// - users 50/60：两个归属人父行（裁定 R1 自种子；布尔列按 PG 写 TRUE/FALSE）；
/// - crm_lead：
///   id=1 A（owner 50）的公海线索，携带原始手机号/邮箱/地址；
///   id=2 A（owner 50）的私海线索（status='new'）；
///   id=3 B（owner 60）的公海线索。
/// roles 不再插：id=1 code='admin'（is_admin_role 判定源，admin_checker.rs:86-87）
/// 与 id=2 均为迁移种子参照行，不参与清空；每个用例重建同构线索种子即可保证
/// ADMIN_ROLE_CACHE（按 role_id 进程级缓存）在任何执行顺序下取值一致。
/// department_id 不手写：迁移触发器 trg_crm_lead_dept 按 owner 的
/// users.department_id 自动维护。
async fn seeded_state() -> AppState {
    let db = test_common::setup_test_db().await;

    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (60,'sales_b','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;

    exec(
        &db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,mobile_phone,email,address,owner_id,owner_name,
             created_at,updated_at) VALUES
             (1,'LD001','website','pool','甲公司','张三','{RAW_PHONE}','{RAW_EMAIL}',
              '河北省邢台市某某路 1 号',50,'销售甲','2026-09-01T00:00:00Z','2026-09-01T00:00:00Z'),
             (2,'LD002','ad','new','乙公司','李四','13711112222','carol@example.com',
              '地址乙',50,'销售甲','2026-09-01T00:00:00Z','2026-09-01T00:00:00Z'),
             (3,'LD003','referral','pool','丙公司','王五','13998887777','bob@example.com',
              '地址丙',60,'销售乙','2026-09-02T00:00:00Z','2026-09-02T00:00:00Z')"
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
    // PoolVisibility::Open：公海行对所有有权进公海者放行（RLS 同口径），
    // self 用户 A 在公海页看到全部 2 条公海行（本人 id=1 + 他人 id=3）
    assert_eq!(items.len(), 2, "公海入口应见全部公海行: {v}");
    // 断言不依赖 created_at DESC 排序（可见集是集合语义）
    let mut got = ids_of(items.as_slice());
    got.sort();
    assert_eq!(got, vec![1, 3], "公海可见集漂移: {v}");
    let lead = items
        .iter()
        .find(|i| i["id"] == json!(1))
        .expect("id=1 应在公海列表");

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
    // Open 放行的是"行可见性"，不是"字段原文"：他人公海行（id=3）同样必须掩码
    let lead_other = items
        .iter()
        .find(|i| i["id"] == json!(3))
        .expect("id=3 应在公海列表");
    assert_eq!(
        lead_other["mobile_phone"],
        json!("139****7777"),
        "公海放行不得连带放宽字段级掩码（他人行原文外泄）"
    );
}

// ---------------------------------------------------------------------------
// 2) 行级 scope 对**私海行**仍生效：B 不得看到 A 的私海线索（公海入口曾传
//    None 绕过过滤）。公海行按 PoolVisibility::Open 全量放行属拍板语义（用例 5），
//    本用例锁"放开仅限公海行、私海行不渗"。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pool_list_honors_row_level_scope_against_other_owner_leads() {
    let app = build_app(seeded_state().await, make_auth(60, 2, "self"));
    let (status, v) = get_json(&app, "/erp/crm/pool?page=1&page_size=20").await;
    assert_eq!(status, StatusCode::OK, "公海列表 200 契约: {v}");
    let ids = ids_of(pool_items(&v));

    // 私海线索（status='new'）不得出现在公海入口（状态谓词 + scope 双保险）——
    // 修复前 list_leads(…, None) 绕过行级过滤时 id=2 同样不可见（谓词挡住），
    // 本断言与用例 5 一起把"公海入口只可能出公海行"钉死
    assert!(!ids.contains(&2), "A 的私海线索出现在公海列表: {v}");
    // Open 语义：本人公海行 id=3 与他人公海行 id=1 均可见
    assert!(ids.contains(&3), "B 本人公海线索应可见: {v}");
    assert_eq!(ids.len(), 2, "公海入口可见集应恰为两条公海行: {v}");

    // 全部可见行都被掩码（掩码判定与行级过滤双缺陷同源入口，各自独立锁）
    for lead in pool_items(&v) {
        let phone = lead["mobile_phone"].as_str().unwrap_or_default();
        let email = lead["email"].as_str().unwrap_or_default();
        assert!(
            phone.contains("****") && email.contains("***@"),
            "公海列表存在未掩码行: {lead}"
        );
    }
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
// 5) 常跑回归锁（原 #[ignore] 目标契约，用户 2026-10-02 拍板 ② 落地）：
//    Self 分支的公海放行——公海行可见性与归属条件相互独立（OR），
//    self 销售在公海页能看到他人公海线索，与 RLS USING 的独立公海 OR 支
//    （rls_dept/mod.rs:195-207）同口径；私海行仍被行级 scope + 状态谓词挡住。
// ---------------------------------------------------------------------------

#[tokio::test]
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

// ---------------------------------------------------------------------------
// 6) Dept 档可见集（拍板 ② 的另一半）：Dept 用户公海页见全部公海行；
//    修复前 Dept 为 `(self ∪ dept) AND pool`——他人部门/无部门公海行被遮蔽，
//    且**领取动作把 lead_status 从 pool 改回 new 后该行反而跌出领取人可见集**
//    （复审实证）。OR 组合下两类漂移同时消除。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dept_scope_user_sees_all_pool_leads_on_pool_page() {
    let auth = AuthContext {
        user_id: 55,
        username: "wave6_pool_dept_55".to_string(),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("dept".to_string()),
        // dept_ids=可见部门集合；dept_member_user_ids=部门成员（与 RLS 同口径）
        dept_ids: Some(std::sync::Arc::new("1".to_string())),
        dept_member_user_ids: Some(std::sync::Arc::new("50,60".to_string())),
    };
    let app = build_app(seeded_state().await, auth);
    let (status, v) = get_json(&app, "/erp/crm/pool?page=1&page_size=20").await;
    assert_eq!(status, StatusCode::OK, "dept 公海列表 200 契约: {v}");
    let ids = ids_of(pool_items(&v));
    assert_eq!(
        ids.len(),
        2,
        "Dept 用户在公海入口应见全部公海行（id=1,3），不多不少: {v}"
    );
    assert!(ids.contains(&1) && ids.contains(&3), "公海可见集漂移: {v}");
    assert!(!ids.contains(&2), "私海行不得进入公海入口: {v}");
}
