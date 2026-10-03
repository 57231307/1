//! 契约波次 6 · 任务 #200：CRM 线索默认字段脱敏读错键防回潮锁
//!
//! 根因（修复前实证）：crm_handler.rs 的 P1-08-5「默认脱敏」块（无数据权限行 +
//! 非 admin 分支）读写的键是 `contact_phone`，但 crm_lead 出参由
//! `serde_json::to_value(crm_lead::Model)` 生成，手机号真实列名是
//! `mobile_phone`（models/crm_lead.rs:40），`contact_phone` 键根本不存在
//! （只存在于 customer / customer_address / sales_order / supplier 模型）——
//! 手机号脱敏恒不生效，非 admin 拿到完全未打码的手机号；同块的 email/address 是生效的。
//!
//! 覆盖（全部真 PostgreSQL 真跑 + 真 HTTP 装配，无伪装断言）：
//! 1. 无数据权限行的非 admin（role_id=2）→ 列表与详情的 `mobile_phone` 为
//!    mask_phone 形态（前 3 后 4，不含原文中段数字）、`email` 掩码、`address` 移除；
//! 2. 有数据权限行（hidden_fields）→ hidden_fields 生效、不走默认打码（既有语义不回归）；
//! 3. admin（role_id=1，roles.code='admin'）→ get_role_data_permission 返回
//!    Ok(Some(ALL, None, None))，filter_fields_batch 为空操作且默认打码分支不可达
//!    （role_id != 1 为假）→ 出参保持原值（这是既有契约，依据即该两行代码路径）；
//! 4. 源码扫描锁：默认脱敏已收敛为唯一实现（#208 收口），锁的两条断言是
//!    「权威 PII 列集合覆盖 crm_lead 真实电话列 mobile_phone/tel_phone」与
//!    「四个出口只调用共用函数、不再有内联掩码分支」；`get("contact_phone")`
//!    这一读错键写法仍被禁止（回潮即红）。
//!
//! 通道（路线一，#4669 判责）：用例经 `test_common::setup_test_db()` 连已迁移
//! PostgreSQL 真跑；表结构唯一来源 = backend/migration，不再自建 DDL；roles 为
//! 迁移种子参照表（id=1 code='admin' 即 is_admin_role 判定源），不再插种子。

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
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

/// 种子：
/// - users：owner_id=50 的归属人父行（裁定 R1 自种子；trg_crm_lead_dept 触发器
///   按其 users.department_id 回填线索冗余部门列）；
/// - crm_lead：一条 owner_id=50 的线索，携带原始手机号/邮箱/地址。
/// roles 不再插：属迁移种子参照表，id=1 code='admin'（is_admin_role 判定源，
/// admin_checker.rs:86）与 id=2（data_permissions 外键父行）已由迁移播种。
async fn seeded_state(hidden_fields: Option<&[&str]>) -> AppState {
    let db = test_common::setup_test_db().await;

    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
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
                 VALUES (1,2,'crm_lead','SELF',NULL,'{hidden_json}',TRUE,
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
// 4) 源码扫描锁：默认脱敏收敛到唯一实现，且不得再按不存在的键打码
//    contact_phone 属 customer/customer_address/sales_order/supplier 模型，
//    crm_lead 出参不存在该键——读错键 = 脱敏恒不生效。
//    #208 遗留项收口后，掩码列集合不再写在 crm_handler.rs 的内联分支里，
//    而是四个出口（列表/详情/公海列表/公海写响应）共用
//    `CrmService::mask_lead_pii_defaults` → `utils::field_mask::mask_contact_fields_for_role`
//    （本仓 PII 列集合权威定义）。因此本锁从"handler 里有 get("mobile_phone")"
//    改为两条更实的断言：
//    a) 权威列集合确实覆盖 crm_lead 的真实电话列（读错键的同一根因在唯一实现处再查一遍）；
//    b) 内联分支已消失，出口只剩共用函数调用（回潮即红）。
// ---------------------------------------------------------------------------

#[test]
fn lead_default_mask_is_single_implementation_on_real_columns() {
    let handler = include_str!("../src/handlers/crm_handler.rs");
    let pool_handler = include_str!("../src/handlers/crm_pool_handler.rs");
    let service = include_str!("../src/services/crm/lead.rs");
    let field_mask = include_str!("../src/utils/field_mask.rs");

    // a) 禁止回潮的错误读键（本文件根因）
    assert!(
        !handler.contains(r#"get("contact_phone")"#),
        "crm_handler.rs 回潮：对 crm_lead 出参读不存在的 contact_phone 键打码"
    );
    // 权威列集合必须含 crm_lead 真实列（models/crm_lead.rs:40/:43）与 email；
    // 漏任一列 = 该列原文直通（本波次实证到的 tel_phone 漏码即此类）
    let phone_keys = field_mask
        .split("pub fn mask_contact_fields_for_role")
        .nth(1)
        .expect("mask_contact_fields_for_role 定义缺失")
        .split("let email_keys")
        .next()
        .expect("电话列集合边界缺失");
    for key in ["mobile_phone", "tel_phone"] {
        assert!(
            phone_keys.contains(&format!("\"{key}\"")),
            "utils/field_mask.rs 电话列集合缺 crm_lead 真实列 {key}（默认脱敏对该列恒不生效）"
        );
    }
    assert!(
        field_mask
            .split("let email_keys")
            .nth(1)
            .expect("邮箱列集合边界缺失")
            .contains("\"email\""),
        "utils/field_mask.rs 邮箱列集合缺 crm_lead 真实列 email"
    );

    // b) 唯一实现 + 四个出口都挂在它上面（不得再各写一份内联掩码分支）
    assert!(
        service.contains("pub fn mask_lead_pii_defaults"),
        "线索默认脱敏唯一实现缺失（列表/详情/公海写响应必须共用同一函数）"
    );
    assert!(
        service.contains("mask_contact_fields_for_role"),
        "唯一实现必须复用 utils/field_mask 的权威 PII 列集合，不得另造第二套列名清单"
    );
    assert!(
        handler.contains("apply_lead_field_permission(&state, auth.role_id, list)"),
        "list_leads 未接共用字段级权限实现"
    );
    assert!(
        handler.contains("std::slice::from_mut(&mut value)"),
        "get_lead 未把单条出参接入共用的字段级权限实现（详情与列表必须同源）"
    );
    // 内联掩码分支已收敛：handler 里不再对 crm_lead 出参手写 address 移除
    assert!(
        !handler.contains(r#"obj.remove("address")"#),
        "回潮棘轮：crm_handler.rs 又出现内联 address 移除分支（应走唯一实现）"
    );
    assert!(
        !pool_handler.contains("field_mask::mask_phone"),
        "回潮棘轮：公海 handler 又出现内联掩码实现（应与列表同源）"
    );
}
