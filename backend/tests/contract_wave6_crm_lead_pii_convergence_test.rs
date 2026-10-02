//! 契约波次 6 · 任务 #204/#208 遗留项：CRM 线索 PII 出口收敛（公海写响应 + tel_phone）
//!
//! 根因（修复前实证，两条同源缺陷）：
//! 1. **写响应回传原文 PII**：`handlers/crm_pool_handler.rs` 的 `claim_from_pool`
//!    与 `recycle_to_pool` 直接 `serde_json::to_value(updated_lead)?`，即整行
//!    `crm_lead::Model` 原文（含 `mobile_phone`/`tel_phone`/`email`/`address`）；
//!    而同一个非 admin 角色走 `GET /crm/leads/:id` 或列表时是被打码的。
//!    ⇒ "列表打码、写响应原文"旁路：点一次"领取/回收"即可换取整行联系方式原文。
//! 2. **默认脱敏漏座机 `tel_phone`**：列表/详情的 P1-08-5 内联分支只打码
//!    `mobile_phone` 与 `email`，而本仓 PII 列集合的权威定义
//!    （`utils/field_mask.rs::mask_contact_fields_for_role`）把 `tel_phone` 也算电话类，
//!    已提交的导出分支 `EXPORT_PII_PHONE_COLUMNS` 亦含它。
//!    ⇒ 同一角色"导出被打码、列表/详情原文"。
//!
//! 修复口径（不新增权限键、不放宽判定、只更严或等价）：
//! - 默认脱敏收敛为**唯一实现** `CrmService::mask_lead_pii_defaults`：电话/邮箱列集合
//!   直接复用权威定义 `mask_contact_fields_for_role`（不另造第二套列名清单），
//!   再加 `address` 整键移除；
//! - 两层字段级权限（有数据权限行 → `filter_fields_batch`；无权限行且非 admin →
//!   默认脱敏）收敛为**唯一入口** `crm_handler::apply_lead_field_permission`，
//!   四个出口共用：`list_leads`（列表）、`get_lead`（详情）、`list_pool`（公海列表）、
//!   `claim_from_pool`/`recycle_to_pool`（写响应，经 `mask_lead_write_response`）；
//! - admin（roles.code='admin'）与"配了 allowed_fields 的角色"保持既有原文通道；
//! - `role_id` 缺失按非 admin 处理（fail-closed，与已提交的 `export_leads` 同口径）。
//!
//! 覆盖边界（诚实声明）：
//! - 权限拒绝文案不在本文件断言原文（只断 `status` + 信封 `code`）；
//! - 公海写响应新增的掩码分支不影响状态码：越权仍由行级 `check_resource_owner` 判 403，
//!   该锁在 `contract_wave6_crm_pool_owner_test.rs` 用例 1/2。
//!
//! 通道（路线一，#4669 判责）：用例经 `test_common::setup_test_db()` 连已迁移
//! PostgreSQL 真跑；表结构唯一来源 = backend/migration，不再自建 DDL。领取/回收
//! 写路径经 AuditLogService::update_with_audit 落真实 audit_logs 表（迁移提供），
//! 操作人 users 行按裁定 R1 自种子。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler::{get_lead, list_leads};
use bingxi_backend::handlers::crm_pool_handler::{claim_from_pool, recycle_to_pool};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 脚手架（与 contract_wave6_crm_lead_mask_test.rs / _pool_owner_test.rs 同款：
// 每用例同构种子，规避 ADMIN_ROLE_CACHE 进程级缓存在任意执行顺序下的串味）
// ---------------------------------------------------------------------------

const USER_A: i32 = 50;
const USER_ADMIN: i32 = 70;
const A_PHONE: &str = "13812348888";
const A_TEL: &str = "03191234567";
const A_EMAIL: &str = "alice@example.com";
const A_ADDRESS: &str = "河北省邢台市某某路 1 号";
/// 掩码后期望值（与 utils/field_mask.rs::mask_phone/mask_email 逐字符一致）
const MASKED_PHONE: &str = "138****8888";
const MASKED_TEL: &str = "031****4567";
const MASKED_EMAIL: &str = "a***@example.com";

fn make_auth(user_id: i32, role_id: i32, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave6_conv_user_{user_id}"),
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

/// 种子（owner 全为 USER_A，self 用户即归属人本人，四个出口都能命中同一行）：
/// - id=1：pool 行（单条领取目标）；
/// - id=2：new 行（回收目标）。
/// 两行都携带原始手机号/座机/邮箱/地址。
/// users 50/70 为自种子父行（裁定 R1：领取/回收写路径经
/// AuditLogService::update_with_audit 取操作人用户名，且 trg_crm_lead_dept
/// 触发器按 owner 的 users.department_id 回填冗余列）。roles 不再插：
/// id=1 code='admin'（is_admin_role 判定源）与 id=2（data_permissions 外键父行）
/// 均为迁移种子参照行，不参与清空。
async fn seeded_state(permissions: Option<&str>) -> AppState {
    let db = test_common::setup_test_db().await;

    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'admin_user','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(
        &db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,mobile_phone,tel_phone,email,address,owner_id,owner_name,
             priority,created_at,updated_at) VALUES
             (1,'LD001','website','pool','甲公司','张三','{A_PHONE}','{A_TEL}','{A_EMAIL}',
              '{A_ADDRESS}',{USER_A},'销售甲','low','2026-01-01T00:00:00Z','2020-01-01T00:00:00Z'),
             (2,'LD002','ad','new','乙公司','李四','13700001111','01088886666','carol@example.com',
              '地址乙',{USER_A},'销售甲','high','2026-01-02T00:00:00Z','2020-01-01T00:00:00Z')"
        ),
    )
    .await;

    if let Some(insert_sql) = permissions {
        exec(&db, insert_sql).await;
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
        .route("/erp/crm/pool/claim", post(claim_from_pool))
        .route("/erp/crm/pool/recycle", post(recycle_to_pool))
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

/// 默认脱敏四件套断言：mobile_phone/tel_phone/email 掩码 + address 整键移除。
/// `where_label` 只用于失败信息定位是哪个出口泄露（四个出口共用一个实现，
/// 任一处回潮都应在本函数报错）。
fn assert_default_masked(lead: &Value, where_label: &str) {
    assert_eq!(
        lead["mobile_phone"],
        json!(MASKED_PHONE),
        "{where_label}：mobile_phone 未掩码（mask_phone 未生效）"
    );
    assert_eq!(
        lead["tel_phone"],
        json!(MASKED_TEL),
        "{where_label}：tel_phone 未掩码（本波次根因：内联分支漏座机列）"
    );
    assert_eq!(
        lead["email"],
        json!(MASKED_EMAIL),
        "{where_label}：email 未掩码"
    );
    assert!(
        lead.get("address").is_none(),
        "{where_label}：address 应被整键移除（P1-08-5 既有语义）: {lead}"
    );
    // 泄露面锁死：出参任何字符串值都不得含原文 PII
    let raw = lead.to_string();
    for banned in [A_PHONE, A_TEL, A_EMAIL, A_ADDRESS] {
        assert!(
            !raw.contains(banned),
            "{where_label}：出参含未脱敏个人信息 {banned}"
        );
    }
}

/// 原文四件套断言（admin / allowed_fields 受控通道）
fn assert_raw_values(lead: &Value, where_label: &str) {
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
        "{where_label}：admin 原值契约（address 不得被移除）"
    );
}

// ---------------------------------------------------------------------------
// 1) 读路径（列表 / 详情）：默认脱敏必须覆盖 tel_phone
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_and_detail_default_mask_covers_tel_phone_and_drops_address() {
    let app = build_app(seeded_state(None).await, make_auth(USER_A, 2, "self"));

    let (status, v) = get_json(&app, "/erp/crm/leads?page=1&page_size=10").await;
    assert_eq!(status, StatusCode::OK, "列表 200 契约: {v}");
    let items = v["data"]["data"]
        .as_array()
        .unwrap_or_else(|| panic!("分页列表形状漂移（期望 data.data 数组）: {v}"));
    assert_eq!(items.len(), 2, "self 用户应见本人两行: {v}");
    let pool_row = items
        .iter()
        .find(|i| i["id"] == json!(1))
        .expect("id=1 应在列表");
    assert_default_masked(pool_row, "GET /crm/leads 列表");

    let (_, v) = get_json(&app, "/erp/crm/leads/1").await;
    assert_default_masked(&v["data"], "GET /crm/leads/:id 详情");
}

// ---------------------------------------------------------------------------
// 2) #204 遗留项核心：公海写响应（领取 / 回收）不得回传原文 PII
// ---------------------------------------------------------------------------

#[tokio::test]
async fn claim_response_does_not_leak_raw_pii() {
    let app = build_app(seeded_state(None).await, make_auth(USER_A, 2, "self"));
    let (status, v) = post_json(&app, "/erp/crm/pool/claim", json!({"lead_id": 1})).await;
    assert_eq!(status, StatusCode::OK, "本人领取公海行应成功: {v}");
    // 状态流转不回潮（领取仍写 new，且 owner 已转移 → 见 pool_owner 锁）
    assert_eq!(v["data"]["lead_status"], json!("new"));
    assert_eq!(v["data"]["owner_id"], json!(USER_A));
    assert_default_masked(&v["data"], "POST /crm/pool/claim 写响应");
}

#[tokio::test]
async fn recycle_response_does_not_leak_raw_pii() {
    let app = build_app(seeded_state(None).await, make_auth(USER_A, 2, "self"));
    let (status, v) = post_json(
        &app,
        "/erp/crm/pool/recycle",
        json!({"lead_id": 2, "reason": "契约用例回收"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人回收应成功: {v}");
    assert_eq!(v["data"]["lead_status"], json!("pool"));
    assert_default_masked(&v["data"], "POST /crm/pool/recycle 写响应");
}

// ---------------------------------------------------------------------------
// 3) admin（roles.code='admin'）：四个出口都保持原文（修复只收紧非 admin，不伤既有通道）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_keeps_raw_pii_on_read_and_write_responses() {
    let app = build_app(seeded_state(None).await, make_auth(USER_ADMIN, 1, "all"));

    let (_, v) = get_json(&app, "/erp/crm/leads/1").await;
    assert_raw_values(&v["data"], "admin GET /crm/leads/:id");

    let (status, v) = post_json(&app, "/erp/crm/pool/claim", json!({"lead_id": 1})).await;
    assert_eq!(status, StatusCode::OK, "admin 领取应成功: {v}");
    assert_raw_values(&v["data"], "admin POST /crm/pool/claim");
}

// ---------------------------------------------------------------------------
// 4) 配了数据权限行的角色：写响应同样走 filter_fields_batch（不叠加默认打码）
//    - hidden_fields=["tel_phone"] → 该键被移除，其余列保持原值（既有白名单语义）；
//    - allowed_fields 含 tel_phone → 放行原文（受控通道，导出侧已同语义）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn claim_response_with_hidden_fields_removes_that_key_only() {
    let insert = r#"INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
        allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
        VALUES (1,2,'crm_lead','SELF',NULL,'["tel_phone"]',TRUE,
        '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#;
    let app = build_app(
        seeded_state(Some(insert)).await,
        make_auth(USER_A, 2, "self"),
    );
    let (status, v) = post_json(&app, "/erp/crm/pool/claim", json!({"lead_id": 1})).await;
    assert_eq!(status, StatusCode::OK, "领取应成功: {v}");
    let lead = &v["data"];
    assert!(
        lead.get("tel_phone").is_none(),
        "hidden_fields 应直接移除该键（非打码）: {lead}"
    );
    // 该分支不叠加默认打码：mobile_phone 保持原值（与列表既有行为锁一致）
    assert_eq!(lead["mobile_phone"], json!(A_PHONE));
    assert_eq!(lead["address"], json!(A_ADDRESS));
}

#[tokio::test]
async fn list_with_allowed_fields_keeps_tel_phone_raw() {
    let allowed = r#"["id","lead_no","lead_source","lead_status","company_name","contact_name",
        "mobile_phone","tel_phone","email","address","owner_id","owner_name","priority",
        "created_at","updated_at"]"#;
    let insert = format!(
        "INSERT INTO data_permissions (id,role_id,resource_type,scope_type,\
         allowed_fields,hidden_fields,is_enabled,created_at,updated_at) \
         VALUES (1,2,'crm_lead','SELF','{allowed}',NULL,TRUE,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
    );
    let app = build_app(
        seeded_state(Some(&insert)).await,
        make_auth(USER_A, 2, "self"),
    );
    let (_, v) = get_json(&app, "/erp/crm/leads?page=1&page_size=10").await;
    let items = v["data"]["data"].as_array().expect("列表数组");
    let row = items
        .iter()
        .find(|i| i["id"] == json!(1))
        .expect("id=1 应在列表");
    assert_eq!(
        row["tel_phone"],
        json!(A_TEL),
        "allowed_fields 含 tel_phone 时应放行原文（受控通道，与导出侧 #207 同语义）"
    );
}

// ---------------------------------------------------------------------------
// 5) 源码扫描锁（shrink-only 棘轮）：四出口不得再退回内联掩码 / 原文直出
// ---------------------------------------------------------------------------

#[test]
fn all_lead_pii_exits_share_one_masking_implementation() {
    let pool_handler = include_str!("../src/handlers/crm_pool_handler.rs");
    let handler = include_str!("../src/handlers/crm_handler.rs");

    // 公海写响应：领取与回收都必须过共用实现，且不再出现"整行原文 to_value"
    for name in ["claim_from_pool", "recycle_to_pool"] {
        let body = pool_handler
            .split(&format!("pub async fn {name}"))
            .nth(1)
            .unwrap_or_else(|| panic!("{name} 定义缺失"))
            .split("pub async fn ")
            .next()
            .unwrap_or_else(|| panic!("{name} 函数体边界缺失"));
        assert!(
            body.contains("mask_lead_write_response("),
            "回潮棘轮：{name} 成功响应未走统一字段级权限实现"
        );
    }
    assert_eq!(
        pool_handler
            .matches("serde_json::to_value(updated_lead)?")
            .count(),
        0,
        "回潮棘轮：公海 handler 出现整行原文回传（PII 写响应旁路）"
    );

    // 列表/详情：不得再各写一份内联掩码分支（唯一实现在服务层）
    assert!(
        !handler.contains(r#""mobile_phone".to_string()"#),
        "回潮棘轮：crm_handler.rs 又出现内联掩码写回分支（应与 mask_lead_pii_defaults 同源）"
    );
    assert!(
        !handler.contains(r#"obj.remove("address")"#),
        "回潮棘轮：crm_handler.rs 又出现内联 address 移除分支"
    );
    assert!(
        handler.contains("pub(crate) async fn apply_lead_field_permission"),
        "线索字段级权限唯一实现缺失"
    );
    // fail-closed：role_id 缺失必须仍走默认脱敏，不得原文放行
    assert!(
        handler.contains("if let Some(rid) = role_id"),
        "apply_lead_field_permission 的 role_id 缺失分支缺失（无角色 = 原文放行旁路）"
    );
}
