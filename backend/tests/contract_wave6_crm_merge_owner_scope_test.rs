//! 契约波次 6 · 线索合并 /leads/merge 行级归属校验（越权写，不可逆）
//!
//! 根因（修复前实证）：`services/crm/lead.rs` 的 `merge_leads` 用
//! `find_by_id(master_lead_id)` 与循环内 `find_by_id(*dup_id)` 取行，
//! 不做任何归属/数据范围校验 → 任何用户可把自己看不到的他人线索标记为 lost
//! 并写入合并原因（不可逆的越权写）。
//!
//! 修复口径（2026-10-02 用户裁定**方案 A** 收口后）：
//! - service 签名带 `data_scope: Option<&DataScopeContext>` + `behalf_granted: bool`；
//! - master 与**每一条** duplicate（存在行）都必须过**写侧归属门**
//!   （`utils/data_scope.rs::check_resource_write_owner`）：本人行恒可合并；
//!   `All` 跨 owner 合并须持有 `crm/cross_owner_write` 代表键（无键 403——裁定原文
//!   "读可用 All 跨 owner；写必须是 owner 本人，或显式持代表键+留痕；读门不得复用为写门"，
//!   **旧口径"凡 All 皆可合并他人行=200" 已按裁定改正**，本文件用例 5 锁该侧）；
//!   `Dept` 限可见部门集合（代管本职）、`Self_` 仅本人行；
//!   admin 经 `check_permission` 内置 `is_admin_role` 放行（裁定"放行路径不变"，用例 4）；
//! - 任一行未过门即整笔拒绝（预校验先判后写，拒绝时零漂移），
//!   禁止"跳过不可见行继续合并其余"的静默降级；
//! - 出参 403 走 AppError::permission_denied 固定脱敏文案 + `FORBIDDEN` 码，
//!   真实原因只进日志；本文件只断 status 与信封 code，不断案文案原文。
//!
//! 覆盖（真 PostgreSQL 真跑 + 真 HTTP 装配；行状态一律 `crm_lead::Entity::find_by_id`
//! 回读真库比对，证明"零漂移"。通道为路线一：`test_common::setup_test_db()`，
//! 表结构唯一来源 = backend/migration，不再自建 DDL；users 归属人行按裁定 R1 自种子）：
//! 1. B（self）合并 A 名下两条 → 403 + code=FORBIDDEN，两行 lead_status/lost_reason 零漂移；
//! 2. B 合并自己名下两条重复 → 200 且真生效（dup→lost、master 不动）；
//! 3. B 混合提交（自有 master + 自有 dup + 他人 dup）→ 403 整笔拒绝，
//!    自有 dup 也**不得**被先行合并（静默降级回潮锁）；
//! 4. admin（roles.code='admin'）合并 A 名下两条 → 200（裁定：admin 靠 is_admin_role
//!    放行路径不变，非"因为 All 所以可写"）；
//! 5. 非 admin 的 `data_scope=all` 角色**不带** crm/cross_owner_write 键合并他人行
//!    → 403（裁定"无键不得代他人写"；读门复用为写门的回潮锁——旧实现此处为 200）；
//! 6. 非 admin 的 `data_scope=all` 角色**带** crm/cross_owner_write 键合并他人行
//!    → 200 且真生效，dup 行 `updated_by` 落操作人（行级可查证据；放行留痕由
//!    `crm_write_guard`/`merge_leads` 的 tracing::info! 承担，用例 7 源码棘轮锁其存在）。

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
use bingxi_backend::handlers::crm_handler::merge_leads;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::crm_lead;
use bingxi_backend::utils::messages::err_msg;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 脚手架（与 contract_wave6_crm_pool_owner_test.rs 同款）
// ---------------------------------------------------------------------------

const USER_B: i32 = 60;
const USER_ADMIN: i32 = 70;
/// 非 admin 的 `data_scope=all` 角色用户（role_id=2 迁移种子非 admin 角色）：
/// 方案 A 两侧断言的当事人——无键不得代他人合并（用例 5），持键可（用例 6）。
const USER_MGR_ALL: i32 = 80;

fn make_auth(user_id: i32, role_id: i32, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave6_merge_user_{user_id}"),
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

/// 种子（两条独立重复对，全部私海 new 行）：
/// - id=1/2：A（owner=50）名下重复对（越权目标）；
/// - id=3/4：B（owner=60）名下重复对（合法合并目标）。
/// users 50/60 为自种子父行（裁定 R1：trg_crm_lead_dept 触发器按 owner 的
/// users.department_id 回填冗余列；handler record_async 审计写真实 audit_logs 表，
/// 该表由迁移提供，无需再建）。
async fn seeded_db() -> Arc<sea_orm::DatabaseConnection> {
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
        "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
         contact_name,mobile_phone,email,owner_id,owner_name,
         created_at,updated_at) VALUES
         (1,'LD-A-001','website','new','甲公司','张三A','13900000001','a1@example.com',
          50,'销售甲','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'LD-A-002','ad','new','甲公司','李四A','13900000002','a2@example.com',
          50,'销售甲','2026-01-02T00:00:00Z','2026-01-02T00:00:00Z'),
         (3,'LD-B-001','referral','new','乙公司','王五B','13800000001','b1@example.com',
          60,'销售乙','2026-01-03T00:00:00Z','2026-01-03T00:00:00Z'),
         (4,'LD-B-002','website','new','乙公司分部','赵六B','13800000002','b2@example.com',
          60,'销售乙','2026-01-04T00:00:00Z','2026-01-04T00:00:00Z')",
    )
    .await;
    Arc::new(db)
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    Router::new()
        .route("/erp/crm/leads/merge", post(merge_leads))
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

/// 回读真库的 (lead_status, lost_reason)——"零漂移/真生效"唯一可信证据
async fn row_state(db: &sea_orm::DatabaseConnection, id: i32) -> (String, Option<String>) {
    let lead = crm_lead::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("线索 {id} 不应被删除"));
    (lead.lead_status.unwrap_or_default(), lead.lost_reason)
}

// ---------------------------------------------------------------------------
// 1) self 用户合并他人名下两条 → 403 + FORBIDDEN，两行零漂移
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_merging_others_leads_is_403_and_rows_untouched() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));
    let before1 = row_state(&db, 1).await;
    let before2 = row_state(&db, 2).await;
    assert_eq!(before1.0, "new");
    assert_eq!(before2.0, "new");

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 1, "duplicate_ids": [2]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "非归属人合并他人线索必须整笔 403（修复前为 200，不可逆越权写）: 信封={}",
        v["code"]
    );
    assert_eq!(v["code"], "FORBIDDEN", "失败信封 code 契约: {}", v["code"]);

    assert_eq!(row_state(&db, 1).await, before1, "master 行必须零漂移");
    assert_eq!(row_state(&db, 2).await, before2, "dup 行必须零漂移");
}

// ---------------------------------------------------------------------------
// 2) self 用户合并自己名下两条重复 → 200 且真生效
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_merging_own_duplicates_succeeds() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 3, "duplicate_ids": [4]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人合并应成功: {v}");
    assert_eq!(v["data"]["merged_count"], json!(1));

    let master = row_state(&db, 3).await;
    assert_eq!(master.0, "new", "master 应保持原状态");
    let dup = row_state(&db, 4).await;
    assert_eq!(dup.0, "lost", "dup 应真落库为 lost");
    assert!(
        dup.1.is_some(),
        "dup 应记录合并原因（lost_reason 非空，不断文案原文）"
    );
}

// ---------------------------------------------------------------------------
// 3) 混合提交（自有 master + 自有 dup + 他人 dup）→ 403 整笔拒绝；
//    自有 dup 也不得被先行合并——"跳过不可见行继续合并其余"的静默降级回潮锁
// ---------------------------------------------------------------------------

#[tokio::test]
async fn partial_invisible_batch_rejects_entire_merge_without_silent_skip() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));
    let before4 = row_state(&db, 4).await;
    let before2 = row_state(&db, 2).await;

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 3, "duplicate_ids": [4, 2]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "批次内任一行不可见必须整笔拒绝而非部分合并: 信封={}",
        v["code"]
    );
    assert_eq!(v["code"], "FORBIDDEN");

    assert_eq!(
        row_state(&db, 4).await,
        before4,
        "静默降级回潮：他人 dup 不可见时，B 自己的 dup 行被先行合并（部分提交）"
    );
    assert_eq!(row_state(&db, 2).await, before2, "他人 dup 行必须零漂移");
}

// ---------------------------------------------------------------------------
// 4) admin（roles.code='admin'）合并他人名下两条 → 200
//    （裁定 2026-10-02：admin 靠 check_permission 内置 is_admin_role 放行，
//    "放行路径不变"——此 200 的依据是"admin 身份"，不是"凡 All 皆可代写"；
//    All 非 admin 的代写须显式代表键，见用例 5/6）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_can_merge_others_leads() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_ADMIN, 1, "all"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 1, "duplicate_ids": [2]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "admin（is_admin_role）代操作合并通道不得被收紧（裁定：放行路径不变）: {v}"
    );
    assert_eq!(row_state(&db, 2).await.0, "lost");
}

// ---------------------------------------------------------------------------
// 5) 非 admin 的 data_scope=all 角色**不带** crm/cross_owner_write 键合并他人行
//    → 403 + FORBIDDEN + 两行零漂移。
//    方案 A 的核心改判：修复前（读门复用为写门）此处为 200——"能看全库"被顺带当成
//    "能改任何人的行"，水平越权正是如此产生。裁定明确"读门不得复用为写门"，
//    All 跨 owner 合并无代表键必拒。本用例是"读门≠写门"在合并入口的回归锁。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_scope_without_behalf_key_merge_is_403_and_rows_untouched() {
    let db = seeded_db().await;
    // role_id=2（迁移种子非 admin 角色），data_scope=all，未授予 crm/cross_owner_write
    let app = build_app(&db, make_auth(USER_MGR_ALL, 2, "all"));
    let before1 = row_state(&db, 1).await;
    let before2 = row_state(&db, 2).await;
    assert_eq!(before1.0, "new");
    assert_eq!(before2.0, "new");

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 1, "duplicate_ids": [2]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "All 范围非本人行无代表键合并必须 403（读门不得复用为写门）: 信封={}",
        v["code"]
    );
    assert_eq!(v["code"], "FORBIDDEN", "失败信封 code 契约: {}", v["code"]);
    // 出参脱敏：断 status+code，不断真实原因原文
    assert_eq!(
        v["message"],
        err_msg::PERMISSION_PUBLIC,
        "403 外显文案必须是固定脱敏常量，不得外显真实拒绝原因"
    );

    assert_eq!(row_state(&db, 1).await, before1, "master 行必须零漂移");
    assert_eq!(row_state(&db, 2).await, before2, "dup 行必须零漂移");
}

// ---------------------------------------------------------------------------
// 6) 非 admin 的 data_scope=all 角色**带** crm/cross_owner_write 代表键合并他人行
//    → 200 且真生效（dup→lost）。放行留痕由 tracing::info! 承担（用例 7 源码棘轮锁
//    其存在），行级可查证据 = dup 行 updated_by 落操作人 USER_MGR_ALL。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_scope_with_behalf_key_merge_succeeds_and_lands_operator() {
    let db = seeded_db().await;
    // 授予 role_id=2 「管理员代操作」键（不在任何迁移播种，仅本用例内显式授予——
    // 印证裁定"该键不播种给任何角色"，默认形态只有 admin 能用）
    exec(
        &db,
        "INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
         VALUES (2,'crm','cross_owner_write',TRUE,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    let app = build_app(&db, make_auth(USER_MGR_ALL, 2, "all"));

    let (status, v) = post_json(
        &app,
        "/erp/crm/leads/merge",
        json!({"primary_id": 1, "duplicate_ids": [2]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "持代表键的 All 范围代他人合并按裁定应 2xx: {v}"
    );
    let dup = crm_lead::Entity::find_by_id(2)
        .one(&*db)
        .await
        .unwrap()
        .expect("dup 行不应被删除");
    assert_eq!(dup.lead_status.unwrap_or_default(), "lost");
    assert_eq!(
        dup.updated_by,
        Some(USER_MGR_ALL),
        "代操作写行级留痕：dup 行 updated_by 必须是真实操作人（非归属人）"
    );
}

// ---------------------------------------------------------------------------
// 7) 源码扫描棘轮（shrink-only）：合并 service 必须用写门 check_resource_write_owner，
//    不得回潮复用读门 check_resource_owner；handler 必须查代表键 + service 内放行留痕。
// ---------------------------------------------------------------------------

#[test]
fn merge_service_uses_write_gate_not_read_gate() {
    let src = include_str!("../src/services/crm/lead.rs");
    let body = src
        .split("pub async fn merge_leads")
        .nth(1)
        .expect("merge_leads 定义缺失")
        .split("pub async fn lead_funnel_report")
        .next()
        .expect("merge_leads 函数体边界缺失");
    assert!(
        body.contains("check_resource_write_owner"),
        "回潮棘轮：合并 service 未走写侧归属门（读门 All 恒真＝越权写）"
    );
    assert!(
        !body.contains("check_resource_owner("),
        "回潮棘轮：合并 service 复用读门 check_resource_owner（裁定明令读门不得复用为写门）"
    );
    assert!(
        body.contains("代操作写放行"),
        "回潮棘轮：合并 service 缺代操作放行 tracing::info! 留痕"
    );

    let handler = include_str!("../src/handlers/crm_handler.rs");
    let merge_fn = handler
        .split("pub async fn merge_leads")
        .nth(1)
        .expect("handler merge_leads 定义缺失")
        .split("pub async fn lead_funnel_report")
        .next()
        .expect("handler merge_leads 边界缺失");
    assert!(
        merge_fn.contains("cross_owner_write_behalf_granted"),
        "回潮棘轮：handler 合并未查代表键（N 行一次）即落写"
    );
}
