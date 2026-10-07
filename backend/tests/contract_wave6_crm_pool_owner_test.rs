//! 契约波次 6 · CRM 公海写端点行级归属门禁（越权回收/越权领取）
//!
//! 根因（修复前实证）：`crm_pool_handler.rs` 的 `claim_from_pool` 与 `recycle_to_pool`
//! 均用 `service.get_lead(lead_id, None)`。`services/crm/lead.rs` 的
//! `get_lead` 把行级归属校验 `check_resource_owner` 包在 `if let Some(ctx)` 内，
//! 传 None 即整体跳过 —— 后果是**任意用户可把他人私海线索回收进公海**
//! （recycle 只写 lead_status=POOL，不校验归属），领取端点同理可对任意 lead_id 落写。
//!
//! 修复口径与权限依据（全部为既有机制，未新增权限键、未放宽任何判定）：
//! - RBAC 层：`/api/v1/erp/crm/pool/recycle` 与 `/pool/claim` 由 URL 段推导出
//!   **同一个键** `pool:create`（`middleware/permission.rs:259` `extract_resource_info`；
//!   `crm` 是模块前缀 `utils/path_utils.rs:75`，`resolve_module_prefixed_resource`
//!   默认臂保留 `pool`；POST → action=create，`recycle`/`claim` 均不在
//!   `PATH_ACTION_KEYWORDS`），admin 角色在 `permission.rs:536`
//!   `admin_checker::is_admin_role`（roles.code='admin'）处整体放行。
//!   ⇒ RBAC 区分不了 claim/recycle，"谁能写哪一行"只能由行级数据权限决定。
//! - recycle：与 `list_leads`/`get_lead`/`update_lead`/`delete_lead` 正常路径同口径注入
//!   `auth.to_data_scope_context()`，复用 `utils/data_scope.rs:149-171`
//!   `check_resource_owner`：`Self_` 仅原归属人本人、`Dept` 需资源 department_id ∈
//!   可见部门集合、`All`（admin/总经理 role.data_scope='all'）可越界；不满足 → 403。
//! - claim：公海行的业务语义是"无归属人、对有权进公海者开放"，**不套私海归属门**
//!   （否则 self 销售领取他人公海行会被打死，本文件用例 4 就是这条可用性锁）；
//!   命中非公海行时才回落到同一个 `check_resource_owner`（用例 2）。
//!
//! 与 2026-10-02 用户裁定**方案 A**（读可 All、写须 owner 或显式 `crm/cross_owner_write`
//! 代表键+留痕）的关系：领取/回收属**公海分支**，维持现状不套跨 owner 写门
//! （`utils/data_scope.rs::check_resource_owner` 文档与 `crm_write_guard` 同口径）；
//! 本文件用例 5 的 admin 回收 200 依据是 admin 既有通道，非"凡 All 皆可代写"
//! （私海行的跨 owner 代写门见 `contract_wave7_crm_read_vs_write_gate_test.rs`
//! 与合并/分配族各测）。
//!
//! 覆盖（真 PostgreSQL 真跑 + 真 HTTP 装配，无硬编码 JSON 假断言；行状态一律用
//! `crm_lead::Entity::find_by_id` 回读真库比对，证明"零漂移"。通道为路线一：
//! `test_common::setup_test_db()`，表结构唯一来源 = backend/migration，
//! 不再自建 DDL）：
//! 1. B（self）recycle A 的私海行 → 403 且该行 lead_status/owner_id 逐字段零漂移；
//! 2. B（self）claim A 的私海行 → 403（不再是"该客户不在公海中"的 400 业务码）且零漂移；
//! 3. B recycle 本人私海行 → 200，lead_status→pool（未被误收紧）；
//! 4. B claim A 的公海行 → 200，lead_status→new（领取可用性未被归属门打死）；
//! 5. admin（data_scope=all）recycle A 的私海行 → 200（既有可越界通道未收紧）；
//! 6. 两条领取路径归属语义已统一（附带项收口）：批量路径
//!    `/pool/{id}/claim`（`services/crm/pool.rs` build_claimed_active）与单条路径
//!    `/pool/claim`（claim_from_pool → `claim_lead_ownership`）都真写 owner_id=领取人，
//!    并把 owner_name 写成**真实操作人名**（AuthContext.username），不再落
//!    `format!("用户{id}")` 造出来的假展示名；
//! 7. 单条 claim 转移归属的回归锁（原为 #[ignore] 的目标契约，源码已落地，
//!    取消 ignore 后即为常跑回归：领取后 owner_id 必须是领取人，
//!    否则 self 销售在自己的数据范围列表里看不到刚领取的行）。
//! 8. 源码扫描锁（shrink-only 棘轮）：recycle 不得再省略 ctx；claim 省略 ctx 的
//!    那一行必须紧随 `check_resource_owner` 回落；claim 必须经统一的
//!    `claim_lead_ownership` 落归属，不得退回"只改状态不写 owner"的旧写法。
//!
//! 覆盖边界（诚实声明，按 2026-10-02 拍板后口径更新）：
//! - 单条领取路径**已经与批量路径同一套公海规则校验**（保护期/领取上限/最大持有数，
//!   判据 = 领取事件列 `last_claimed_at`/`last_claimed_by`，见
//!   `services/crm/pool.rs` 文件头与 `tests/services_crm_pool_claim_rules_test.rs`）。
//!   本文件种子行的 `last_claimed_at` 均为 NULL（存量语义=无保护期），
//!   且领取次数远低于默认上限（5/日、50 持有），既有断言不受校验收紧影响——
//!   校验行为本身不在本文件重复断言。
//! - 本文件不依赖 PG 专有特性（claim/recycle 写路径无取号咨询锁、无 lock_exclusive），
//!   故不另开活库用例；`utils/data_scope.rs` Self/Dept 分支的公海放行
//!   （`PoolVisibility::Open`，拍板 ②）已由 `contract_wave6_crm_pool_mask_test.rs`
//!   用例 5/6 常跑锁定，本文件不重复。

mod test_common;

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
use bingxi_backend::handlers::crm_pool_handler::{
    claim_from_pool, claim_specific, recycle_to_pool,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::crm_lead;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 脚手架（与 contract_wave6_crm_*_mask_test.rs 同款：每用例同构种子，
// 规避 ADMIN_ROLE_CACHE 进程级缓存在任意执行顺序下串味）
// ---------------------------------------------------------------------------

const USER_A: i32 = 50;
const USER_B: i32 = 60;
const USER_ADMIN: i32 = 70;

fn make_auth(user_id: i32, role_id: i32, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave6_owner_user_{user_id}"),
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
/// - id=1：A 的公海行（updated_at 远在保护期之外，批量领取路径可用）；
/// - id=2：A 的私海行（越权目标）；
/// - id=3：B 的私海行（合法回收目标）。
/// users 50/60/70 为自种子父行（裁定 R1：update_lead →
/// AuditLogService::update_with_audit 取操作人用户名需真实 users/audit_logs 表，
/// 二者均由迁移提供；trg_crm_lead_dept 触发器按 owner 的 users.department_id
/// 回填冗余部门列，布尔列按 PG 写 TRUE/FALSE）。
/// roles 不再插：id=1 code='admin'（is_admin_role 判定源，admin_checker.rs:86-87）
/// 为迁移种子参照行。customer_pool_rules 为迁移真实表（v15 建表），批量领取路径
/// 的规则校验读到空表 → 走默认值兜底，无需建表也无需播种。
/// department_id 不手写：由触发器维护。
async fn seeded_db() -> Arc<sea_orm::DatabaseConnection> {
    let db = test_common::setup_test_db().await;

    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (60,'sales_b','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'admin_user','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(
        &db,
        "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
         contact_name,mobile_phone,email,owner_id,owner_name,
         created_at,updated_at) VALUES
         (1,'LD001','website','pool','甲公司','张三','13812348888','alice@example.com',
          50,'销售甲','2026-01-01T00:00:00Z','2020-01-01T00:00:00Z'),
         (2,'LD002','ad','new','乙公司','李四','13700001111','carol@example.com',
          50,'销售甲','2026-01-02T00:00:00Z','2020-01-01T00:00:00Z'),
         (3,'LD003','referral','new','丙公司','王五','13711112222','bob@example.com',
          60,'销售乙','2026-01-03T00:00:00Z','2020-01-01T00:00:00Z')",
    )
    .await;

    Arc::new(db)
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    // 装配补全（族I Disconnected 根因）：handler 走两条独立的数据库把手——写路径用
    // state.db（CrmService::new(state.db)，见 crm_pool_handler.rs:197/266），
    // 出参字段级权限走 state.data_permission_service 内嵌连接
    // （crm_handler.rs:57 resolve_role_data_permission）。AppState::default() 用
    // DatabaseConnection::default()（= sea-orm Disconnected，container/mod.rs:334）
    // 构造 data_permission_service，只覆盖 db 字段会让后者仍持 Disconnected，
    // mask_lead_write_response → get_role_data_permission 直接 panic（Disconnected）。
    // 与生产装配链同形（container/mod.rs:221）：用同一真实测试库重建后注入。
    let state = AppState {
        db: db.clone(),
        data_permission_service: Arc::new(DataPermissionService::new(db.clone())),
        ..Default::default()
    };
    Router::new()
        .route("/erp/crm/pool/claim", post(claim_from_pool))
        .route("/erp/crm/pool/recycle", post(recycle_to_pool))
        .route("/erp/crm/pool/{customer_id}/claim", post(claim_specific))
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

/// 回读真库的 (lead_status, owner_id)——"零漂移"唯一可信证据
async fn row_state(db: &sea_orm::DatabaseConnection, id: i32) -> (String, i32) {
    let (status, owner_id, _) = row_state_with_owner_name(db, id).await;
    (status, owner_id)
}

/// 回读真库的 (lead_status, owner_id, owner_name)：领取路径的归属语义既要看
/// owner_id 是否转移，也要看 owner_name 是否为真实操作人（而非 `用户{id}` 假名）
async fn row_state_with_owner_name(
    db: &sea_orm::DatabaseConnection,
    id: i32,
) -> (String, i32, String) {
    let lead = crm_lead::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("线索 {id} 不应被删除"));
    (
        lead.lead_status.unwrap_or_default(),
        lead.owner_id,
        lead.owner_name,
    )
}

// ---------------------------------------------------------------------------
// 1) self 用户 recycle 他人私海行 → 403，且该行零漂移（核心越权写）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_recycle_others_private_lead_is_403_and_row_untouched() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));
    let before = row_state(&db, 2).await;
    assert_eq!(before, ("new".to_string(), USER_A));

    let (status, v) = post_json(&app, "/erp/crm/pool/recycle", json!({"lead_id": 2})).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "非归属人回收他人私海行必须 403（修复前为 200，越权写）: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN", "失败信封 code 契约: {v}");

    let after = row_state(&db, 2).await;
    assert_eq!(
        after, before,
        "越权回收被拒后 lead_status/owner_id 必须零漂移（不得先写再拒）"
    );
}

// ---------------------------------------------------------------------------
// 2) self 用户 claim 他人私海行 → 403（领取端点不得成为第二个越权写入口）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_claim_others_private_lead_is_403_not_business_400() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));
    let before = row_state(&db, 2).await;

    let (status, v) = post_json(&app, "/erp/crm/pool/claim", json!({"lead_id": 2})).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "非公海行且非归属人 → 应按行级归属判 403，而不是仅给「不在公海中」的业务 400: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");

    let after = row_state(&db, 2).await;
    assert_eq!(after, before, "越权领取被拒后该行必须零漂移");
}

// ---------------------------------------------------------------------------
// 3) self 用户回收本人私海行 → 仍可用（未被误收紧）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_recycle_own_lead_still_works() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (status, v) = post_json(&app, "/erp/crm/pool/recycle", json!({"lead_id": 3})).await;
    assert_eq!(status, StatusCode::OK, "本人回收应成功: {v}");
    let after = row_state(&db, 3).await;
    assert_eq!(after.0, "pool", "回收后 lead_status 应为 pool");
    // 回收只改状态不改 owner_id 是既有语义（本批不扩大范围，见交付报告）
    assert_eq!(after.1, USER_B);
}

// ---------------------------------------------------------------------------
// 4) self 用户领取他人公海行 → 仍可用（不得把私海归属门套到领取上）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_claim_others_pool_lead_still_works() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (status, v) = post_json(&app, "/erp/crm/pool/claim", json!({"lead_id": 1})).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "领取的合法主体是「有权进公海的人」，公海行不得被套私海归属门打死: {v}"
    );
    assert_eq!(v["data"]["lead_status"], json!("new"));
    let after = row_state(&db, 1).await;
    assert_eq!(after.0, "new", "领取后 lead_status 应落库为 new");
}

// ---------------------------------------------------------------------------
// 5) admin（role.code='admin' + data_scope=all）回收他人私海行 → 仍可越界
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_can_recycle_others_private_lead() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_ADMIN, 1, "all"));

    let (status, v) = post_json(&app, "/erp/crm/pool/recycle", json!({"lead_id": 2})).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "DataScope::All（admin）的既有越界通道不得被收紧: {v}"
    );
    assert_eq!(row_state(&db, 2).await.0, "pool");
}

// ---------------------------------------------------------------------------
// 6) 两条领取路径归属语义统一：都写 owner_id=领取人，且 owner_name 为真实操作人
//    （收口前实证：批量路径真写 owner_id，单条路径只改 lead_status → 线索仍挂在
//    回收前的原归属人名下，属两条路径实现漂移；本用例把"两条路径同语义"锁成等式）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn both_claim_paths_write_owner_id_with_real_operator_name() {
    // 批量路径 `/pool/{id}/claim` → claim_pool_customers → build_claimed_active
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));
    let (status, v) = post_json(&app, "/erp/crm/pool/1/claim", json!(null)).await;
    // claim_specific 无请求体，用空 JSON 仅作 body 占位
    assert_eq!(status, StatusCode::OK, "批量领取路径应成功: {v}");
    let batch = row_state_with_owner_name(&db, 1).await;
    assert_eq!(batch.0, "new", "批量领取后 lead_status 应为 new");
    assert_eq!(batch.1, USER_B, "批量领取路径应把 owner_id 写成领取人");
    assert_eq!(
        batch.2, "wave6_owner_user_60",
        "owner_name 必须是真实操作人名（落库口径 = AuthContext.username）"
    );

    // 单条路径 `/pool/claim` → claim_from_pool → claim_lead_ownership
    // → **同一个** build_claimed_active（单一归属实现）
    let db2 = seeded_db().await;
    let app2 = build_app(&db2, make_auth(USER_B, 2, "self"));
    let (status, v) = post_json(&app2, "/erp/crm/pool/claim", json!({"lead_id": 1})).await;
    assert_eq!(status, StatusCode::OK, "单条领取应成功: {v}");
    let single = row_state_with_owner_name(&db2, 1).await;
    assert_eq!(single.0, "new", "单条领取后 lead_status 应落库为 new");
    assert_eq!(
        (single.0.clone(), single.1, single.2.clone()),
        batch,
        "两条领取路径的归属结果必须逐字段一致（单条路径回潮 = 领取人 self 列表看不到刚领取的行）"
    );
    // 负例：禁止回退到 `format!("用户{id}")` 造假展示名（本仓硬规则）
    assert!(
        !single.2.starts_with("用户"),
        "owner_name 回潮为伪造展示名（pool.rs 不得用 format! 造名）: {}",
        single.2
    );
    assert_eq!(single.2, "wave6_owner_user_60");
}

// ---------------------------------------------------------------------------
// 7) 单条领取转移归属的回归锁（原 #[ignore] 目标契约，源码已落地 → 常跑）
//    修复前的 ignore 理由是"行为未实现且需改契约"；本批按"复用 build_claimed_active"
//    落地（未引入公海规则校验，理由见文件头"覆盖边界"），故取消 ignore 成为回归锁。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn single_claim_should_transfer_ownership_to_claimer() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));
    let (status, v) = post_json(&app, "/erp/crm/pool/claim", json!({"lead_id": 1})).await;
    assert_eq!(status, StatusCode::OK, "领取应成功: {v}");

    let (status_after, owner_after) = row_state(&db, 1).await;
    assert_eq!(status_after, "new");
    assert_eq!(
        owner_after, USER_B,
        "目标契约：领取人即归属人，单条与批量两条路径必须同语义"
    );
}

// ---------------------------------------------------------------------------
// 8) 源码扫描锁（shrink-only 棘轮）
// ---------------------------------------------------------------------------

#[test]
fn pool_write_handlers_must_not_skip_data_scope() {
    let src = include_str!("../src/handlers/crm_pool_handler.rs");

    let recycle = src
        .split("pub async fn recycle_to_pool")
        .nth(1)
        .expect("recycle_to_pool 定义缺失")
        .split("pub async fn claim_specific")
        .next()
        .expect("recycle_to_pool 函数体边界缺失");
    assert!(
        !recycle.contains("get_lead(req.lead_id, None)"),
        "回潮棘轮：回收端点再次省略行级 scope（任意用户可回收他人私海行）"
    );
    assert!(
        recycle.contains("get_lead(req.lead_id, Some(&data_scope_ctx))"),
        "回收端点必须与 get_lead/update_lead 正常路径同口径注入 ctx"
    );

    let claim = src
        .split("pub async fn claim_from_pool")
        .nth(1)
        .expect("claim_from_pool 定义缺失")
        .split("pub async fn recycle_to_pool")
        .next()
        .expect("claim_from_pool 函数体边界缺失");
    assert!(
        claim.contains("check_resource_owner(&data_scope_ctx"),
        "领取端点缺非公海行的归属回落（可借领取改写他人私海行）"
    );
    // 附带项收口棘轮：单条领取必须经统一的归属实现
    assert!(
        claim.contains("claim_lead_ownership(lead, auth.user_id, &auth.username)"),
        "回潮棘轮：单条领取端点不再经 claim_lead_ownership 落归属（两条领取路径重新分叉）"
    );
    assert!(
        !claim.contains("lead_status: Some(lead_status::NEW.to_string())"),
        "回潮棘轮：单条领取端点退回\"只把 lead_status 置 new、不写 owner\"的旧写法"
    );

    // 写响应原文旁路棘轮：claim/recycle 的成功出参必须过统一的字段级权限实现
    for (name, body) in [
        ("claim_from_pool", claim),
        (
            "recycle_to_pool",
            src.split("pub async fn recycle_to_pool")
                .nth(1)
                .expect("recycle_to_pool 定义缺失")
                .split("pub async fn claim_specific")
                .next()
                .expect("recycle_to_pool 函数体边界缺失"),
        ),
    ] {
        assert!(
            body.contains("mask_lead_write_response(&state, auth.role_id, &updated_lead)"),
            "回潮棘轮：{name} 成功响应未走 mask_lead_write_response（整行原文回传 PII）"
        );
        assert!(
            !body.contains("serde_json::to_value(updated_lead)?"),
            "回潮棘轮：{name} 再次整行原文 serde_json::to_value(updated_lead) 回传"
        );
    }

    // 归属实现单一化棘轮：服务层不得再用 format! 造展示名
    let pool_service = include_str!("../src/services/crm/pool.rs");
    assert!(
        !pool_service.contains("format!(\"用户{}\""),
        "回潮棘轮：services/crm/pool.rs 用 format! 伪造 owner_name 展示名"
    );
    assert!(
        pool_service.contains("lead_active.owner_name = Set(operator_name.to_string())"),
        "build_claimed_active 必须落真实操作人名（claim_pool_customers 的 operator_name 形参不得再被忽略）"
    );
    assert!(
        !pool_service.contains("_operator_name"),
        "回潮棘轮：claim_pool_customers 的 operator_name 形参又被忽略"
    );
}
