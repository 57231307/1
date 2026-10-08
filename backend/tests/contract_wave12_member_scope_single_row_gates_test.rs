//! 跨域共用的单行归属门活体锁：证明 `check_resource_owner_by_member_scope`
//! 在 Dept 范围下真的按「归属人 ∈ 可见部门成员集合」区分放行/拒绝，
//! 而不是退化成恒拒或放宽成 All。
//!
//! 功能：以定制订单取消端点（`DELETE /custom-orders/{id}`，无 `department_id` 列，
//! 写前经单行归属门判定归属）为唯一 HTTP 载体，钉死三种数据范围（All/Dept/Self_）
//! 对「本人行 / 同可见部门成员行 / 非成员行 / NULL 归属历史行」的操作结果；并用纯函数
//! 真值表直接断言该判据的全部输入组合，防止有人把它改回恒拒（Dept 一律 403）或
//! 放宽成恒放行（Dept/Self_ 变成等价 All）。
//! 调用方：`cargo test` 集成层（真已迁移 PostgreSQL，TEST_DATABASE_URL；CI 以
//! `--test-threads=1` 串行、独占一只 service 容器库）。
//! 入参：经 `test_common::setup_test_db()` 连接并清空业务表后，按私有 id 段播种
//! 三个用户、一条客户、一条产品与四条定制订单（归属人分别为本人 / 同可见部门成员 /
//! 非成员 / NULL）；再以不同 `AuthContext`（data_scope 与 dept_member_user_ids 逐用例
//! 指定）经 `axum::middleware::from_fn_with_state` 注入，走真 HTTP 装配。
//! 传给谁：`handlers::custom_order_handler::cancel_custom_order` 及其写前归属门。
//! 存什么：放行时 `status` 由 draft 如实迁到 cancelled 且 `notes` 追加取消原因；
//! 拒绝时归属门在 service 落库点之前 return 403，`status`/`notes` 零漂移。
//! 存哪里：`custom_orders` 表（回读真库逐字段比对，不信任响应体）。
//!
//! 判据（逐条对应下方用例）：
//! - Dept：归属人=本人 ⇒ 2xx 且状态/字段如实变更；
//! - Dept：归属人∈可见成员集合 ⇒ 按新判据放行（Dept 的本职）——该行为在旧「资源部门
//!   列」判据（无部门列只能传 None ⇒ Dept 恒拒）下必红，是判据切换的核心可证伪点；
//! - Dept：归属人∉可见成员集合 ⇒ 403 + 机器码 FORBIDDEN + 固定脱敏文案「无权限」
//!   （断言文案不含记录 ID），且回读状态零漂移；
//! - Dept 且成员集合为空（退化仅本人）：本人行放行、他人行拒绝；
//! - Self_：本人放行、他人 403；
//! - NULL 归属历史行：All 可操作，Dept 与 Self_ 一律拒绝（不放宽成「无主即可读写」）；
//! - All：任意归属行（含跨成员、含 NULL 归属）均可操作，钉死读侧同宽度量。
//!
//! 真值表用例（纯函数级）反向自证：直接对 `check_resource_owner_by_member_scope`
//! 断言全部 (scope × owner) 组合成真值表——
//!   若函数退化成恒 false：上述所有「应放行」的 HTTP 用例（Dept 本人、Dept 成员、
//!     Dept 空集本人、Self_ 本人、All 任意、All/NULL）与真值表全部 true 行同时变红；
//!   若函数退化成恒 true（Dept/Self_ 被放宽成 All）：所有「应拒绝」用例（Dept 非成员
//!     403、Dept 空集他人 403、Self_ 他人 403、Dept/NULL 403、Self_/NULL 403）与真值表
//!     全部 false 行同时变红。
//!   因此本锁对「改回恒拒」与「改宽成 All」两个方向都敏感，任一回潮必红。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::delete,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::custom_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{custom_order, customer, product, user};
use bingxi_backend::utils::data_scope::{
    DataScope, DataScopeContext, check_resource_owner_by_member_scope,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// 私有 id 段（941x 用户 / 947x 主数据 / 951x 定制订单），与并行域用例的 id 区间不重叠。
const ACTOR: i32 = 9410; // 发起操作的用户（Dept/Self_ 的本人）
const MEMBER: i32 = 9411; // 与 ACTOR 同属可见部门的另一成员
const OUTSIDER: i32 = 9412; // 不在 ACTOR 可见成员集合内的用户
const DEPT_CSV: &str = "9460"; // dept 用户的可见部门串（本判据不使用部门列，仅占位）

const CUSTOMER_ID: i32 = 9470;
const PRODUCT_ID: i32 = 9471;

const CO_SELF: i64 = 9510; // 归属人 = ACTOR（本人行）
const CO_MEMBER: i64 = 9511; // 归属人 = MEMBER（同可见部门成员行）
const CO_OUTSIDER: i64 = 9512; // 归属人 = OUTSIDER（非成员行）
const CO_NULL: i64 = 9513; // 归属人 = NULL（历史无主行）

// ------------------------------------------------------------------
// AuthContext 构造助手（data_scope 与 dept_member_user_ids 逐用例注入）
// ------------------------------------------------------------------

fn members_csv(members: &[i32]) -> Arc<String> {
    Arc::new(
        members
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
    )
}

/// Dept 范围：可见成员集合由 `members` 指定（不含 ACTOR 时由 to_data_scope_context 兜底并入本人）。
/// 传空切片 ⇒ 集合退化为仅本人。
fn auth_dept(actor: i32, members: &[i32]) -> AuthContext {
    AuthContext {
        user_id: actor,
        username: format!("ms_actor_{actor}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("dept".to_string()),
        dept_ids: Some(Arc::new(DEPT_CSV.to_string())),
        dept_member_user_ids: Some(members_csv(members)),
    }
}

/// Self_ 范围：仅本人。
fn auth_self(actor: i32) -> AuthContext {
    AuthContext {
        user_id: actor,
        username: format!("ms_actor_{actor}"),
        role_id: Some(3),
        department_id: Some(1),
        data_scope: Some("self".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

/// All 范围：任意行。
fn auth_all(actor: i32) -> AuthContext {
    AuthContext {
        user_id: actor,
        username: format!("ms_actor_{actor}"),
        role_id: Some(1),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
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

async fn cancel(app: &Router, uri: &str, reason: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::DELETE)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(json!({ "reason": reason }).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "取消响应非 JSON（{uri}）: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

async fn seed(db: &sea_orm::DatabaseConnection) {
    for uid in [ACTOR, MEMBER, OUTSIDER] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("ms_actor_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(1)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    customer::ActiveModel {
        id: Set(CUSTOMER_ID),
        customer_code: Set("CUS-MS-LOCK".to_string()),
        customer_name: Set("单行归属门锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(ACTOR),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    product::ActiveModel {
        id: Set(PRODUCT_ID),
        code: Set("PROD-MS-LOCK".to_string()),
        name: Set("单行归属门锁产品".to_string()),
        unit: Set("m".to_string()),
        product_type: Set("fabric".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    // 四条草稿态定制订单，归属人分别为 本人 / 成员 / 非成员 / NULL。
    for (co_id, owner) in [
        (CO_SELF, Some(ACTOR as i64)),
        (CO_MEMBER, Some(MEMBER as i64)),
        (CO_OUTSIDER, Some(OUTSIDER as i64)),
        (CO_NULL, None),
    ] {
        custom_order::ActiveModel {
            id: Set(co_id),
            order_no: Set(format!("CO-MS-{co_id}")),
            customer_id: Set(i64::from(CUSTOMER_ID)),
            product_id: Set(i64::from(PRODUCT_ID)),
            color_id: Set(None),
            spec: Set("draft-spec".to_string()),
            quantity: Set(Decimal::ONE),
            unit: Set("m".to_string()),
            custom_requirements: Set(json!({})),
            status: Set("draft".to_string()),
            currency: Set("CNY".to_string()),
            created_by: Set(owner),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
}

fn build_app(auth: AuthContext, db: Arc<sea_orm::DatabaseConnection>) -> Router {
    let state = AppState {
        db,
        ..Default::default()
    };
    Router::new()
        .route(
            "/custom-orders/{id}",
            delete(custom_order_handler::cancel_custom_order),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn seeded_app(auth: AuthContext) -> (Router, Arc<sea_orm::DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let app = build_app(auth, db.clone());
    (app, db)
}

/// 回读真库：返回 (status, notes)。用例以此验证写侧如实变更 / 越权零漂移，绝不信任响应体。
async fn readback(db: &sea_orm::DatabaseConnection, id: i64) -> (String, Option<String>) {
    let row = custom_order::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("回读前提：定制订单 {id} 应存在"));
    (row.status, row.notes)
}

// ==================================================================
// 1. Dept + 归属人=本人 ⇒ 2xx，状态/字段如实变更（回读真库）
// ==================================================================
#[tokio::test]
async fn dept_own_row_allowed_and_persists() {
    let (app, db) = seeded_app(auth_dept(ACTOR, &[ACTOR, MEMBER])).await;
    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_SELF}"), "本人取消").await;
    assert_eq!(status, StatusCode::OK, "Dept 本人行取消应 2xx: {v}");

    let (st, notes) = readback(&db, CO_SELF).await;
    assert_eq!(st, "cancelled", "写侧状态应如实迁到 cancelled，实得: {st}");
    assert!(
        notes.as_deref().is_some_and(|n| n.contains("本人取消")),
        "取消原因应落库到 notes，实得: {notes:?}"
    );
}

// ==================================================================
// 2. Dept + 归属人∈可见成员集合 ⇒ 放行（Dept 本职；旧「资源部门列」判据下必红）
// ==================================================================
#[tokio::test]
async fn dept_visible_member_row_allowed_and_persists() {
    let (app, db) = seeded_app(auth_dept(ACTOR, &[ACTOR, MEMBER])).await;
    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_MEMBER}"), "代成员取消").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "Dept 归属人∈可见成员集合应放行（区别于恒拒）: {v}"
    );

    let (st, notes) = readback(&db, CO_MEMBER).await;
    assert_eq!(st, "cancelled", "成员行被放行后状态应变更，实得: {st}");
    assert!(
        notes.as_deref().is_some_and(|n| n.contains("代成员取消")),
        "成员行取消原因应落库，实得: {notes:?}"
    );
}

// ==================================================================
// 3. Dept + 归属人∉可见成员集合 ⇒ 403 + FORBIDDEN + 脱敏文案（不含 ID），零漂移
// ==================================================================
#[tokio::test]
async fn dept_non_member_row_denied_403_and_zero_drift() {
    let (app, db) = seeded_app(auth_dept(ACTOR, &[ACTOR, MEMBER])).await;
    let (before_st, before_notes) = readback(&db, CO_OUTSIDER).await;
    assert_eq!(before_st, "draft", "回读前提：非成员行初始为 draft");

    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_OUTSIDER}"), "越权取消").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "Dept 非成员行必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN", "机器码应为 FORBIDDEN: {v}");

    let msg = v["message"]
        .as_str()
        .unwrap_or_else(|| panic!("403 响应应含 message 字段: {v}"));
    assert_eq!(
        msg, "无权限",
        "越权文案应为固定脱敏常量「无权限」，实得: {msg}"
    );
    assert!(
        !msg.contains(&CO_OUTSIDER.to_string()),
        "脱敏文案严禁泄露记录 ID（{CO_OUTSIDER}）: {msg}"
    );

    let (after_st, after_notes) = readback(&db, CO_OUTSIDER).await;
    assert_eq!(after_st, before_st, "越权被拒后状态不得漂移");
    assert_eq!(after_notes, before_notes, "越权被拒后 notes 不得漂移");
}

// ==================================================================
// 4. Dept + 成员集合为空（退化仅本人）：本人放行、他人拒绝
// ==================================================================
#[tokio::test]
async fn dept_empty_member_set_own_allowed() {
    // 传空成员集合 ⇒ to_data_scope_context 兜底并入本人 ⇒ 集合仅 [ACTOR]，退化为 Self 语义。
    let (app, db) = seeded_app(auth_dept(ACTOR, &[])).await;
    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_SELF}"), "空集本人取消").await;
    assert_eq!(status, StatusCode::OK, "Dept 空集下本人行应放行: {v}");
    let (st, _) = readback(&db, CO_SELF).await;
    assert_eq!(st, "cancelled", "空集本人行放行后状态应变更，实得: {st}");
}

#[tokio::test]
async fn dept_empty_member_set_member_row_denied() {
    // 同一空集场景：MEMBER 行不在退化的仅本人集合内 ⇒ 必须拒绝（证明退化到 Self，不是恒放行）。
    let (app, db) = seeded_app(auth_dept(ACTOR, &[])).await;
    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_MEMBER}"), "空集他人取消").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Dept 空集下他人行必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    let (st, _) = readback(&db, CO_MEMBER).await;
    assert_eq!(st, "draft", "空集他人行被拒后状态不得漂移，实得: {st}");
}

// ==================================================================
// 5. Self_：本人放行、他人 403
// ==================================================================
#[tokio::test]
async fn self_own_row_allowed() {
    let (app, db) = seeded_app(auth_self(ACTOR)).await;
    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_SELF}"), "self 本人取消").await;
    assert_eq!(status, StatusCode::OK, "Self_ 本人行应放行: {v}");
    let (st, _) = readback(&db, CO_SELF).await;
    assert_eq!(st, "cancelled", "Self_ 本人行放行后状态应变更，实得: {st}");
}

#[tokio::test]
async fn self_member_row_denied() {
    let (app, db) = seeded_app(auth_self(ACTOR)).await;
    let (status, v) = cancel(
        &app,
        &format!("/custom-orders/{CO_MEMBER}"),
        "self 他人取消",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "Self_ 他人行必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    let (st, _) = readback(&db, CO_MEMBER).await;
    assert_eq!(st, "draft", "Self_ 他人行被拒后状态不得漂移，实得: {st}");
}

// ==================================================================
// 6. NULL 归属历史行：All 可操作，Dept 与 Self_ 一律拒绝（不放宽）
// ==================================================================
#[tokio::test]
async fn null_owner_row_all_allowed() {
    let (app, db) = seeded_app(auth_all(ACTOR)).await;
    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_NULL}"), "all 无主取消").await;
    assert_eq!(status, StatusCode::OK, "All 对 NULL 归属行应可操作: {v}");
    let (st, _) = readback(&db, CO_NULL).await;
    assert_eq!(st, "cancelled", "All 无主行放行后状态应变更，实得: {st}");
}

#[tokio::test]
async fn null_owner_row_dept_denied() {
    let (app, db) = seeded_app(auth_dept(ACTOR, &[ACTOR, MEMBER])).await;
    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_NULL}"), "dept 无主取消").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Dept 对 NULL 归属行必须拒绝: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    let (st, _) = readback(&db, CO_NULL).await;
    assert_eq!(st, "draft", "Dept 无主行被拒后状态不得漂移，实得: {st}");
}

#[tokio::test]
async fn null_owner_row_self_denied() {
    let (app, db) = seeded_app(auth_self(ACTOR)).await;
    let (status, v) = cancel(&app, &format!("/custom-orders/{CO_NULL}"), "self 无主取消").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Self_ 对 NULL 归属行必须拒绝: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    let (st, _) = readback(&db, CO_NULL).await;
    assert_eq!(st, "draft", "Self_ 无主行被拒后状态不得漂移，实得: {st}");
}

// ==================================================================
// All 宽度锚点：任意归属行（含跨成员、非成员）均可操作，钉死判据未被改窄/改宽语义漂移
// ==================================================================
#[tokio::test]
async fn all_scope_can_act_on_any_owner_row() {
    let (app, db) = seeded_app(auth_all(ACTOR)).await;
    let (status, v) = cancel(
        &app,
        &format!("/custom-orders/{CO_OUTSIDER}"),
        "all 跨主取消",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "All 对非成员归属行应放行: {v}");
    let (st, _) = readback(&db, CO_OUTSIDER).await;
    assert_eq!(st, "cancelled", "All 跨主行放行后状态应变更，实得: {st}");
}

// ==================================================================
// 7. 真值表反向自证（纯函数级）：直接钉死 check_resource_owner_by_member_scope 全部输入组合
// ==================================================================
fn ctx(scope: DataScope, user_id: i32, members: Vec<i32>) -> DataScopeContext {
    DataScopeContext {
        scope,
        user_id,
        department_id: Some(1),
        dept_ids: vec![1],
        dept_member_user_ids: members,
    }
}

#[test]
fn member_scope_predicate_truth_table() {
    // All：任意归属（含 NULL）恒放行。
    assert!(check_resource_owner_by_member_scope(
        &ctx(DataScope::All, ACTOR, vec![]),
        None
    ));
    assert!(check_resource_owner_by_member_scope(
        &ctx(DataScope::All, ACTOR, vec![ACTOR]),
        Some(MEMBER),
    ));

    // Dept + 非空成员集合 [ACTOR, MEMBER]：本人/成员放行，非成员与 NULL 拒绝。
    let dept_members = ctx(DataScope::Dept, ACTOR, vec![ACTOR, MEMBER]);
    assert!(check_resource_owner_by_member_scope(
        &dept_members,
        Some(ACTOR)
    ));
    assert!(check_resource_owner_by_member_scope(
        &dept_members,
        Some(MEMBER)
    ));
    assert!(!check_resource_owner_by_member_scope(
        &dept_members,
        Some(OUTSIDER)
    ));
    assert!(!check_resource_owner_by_member_scope(&dept_members, None));

    // Dept + 空成员集合：退化仅本人（守卫 owner==user_id），他人/NULL 拒绝，绝不恒放行。
    let dept_empty = ctx(DataScope::Dept, ACTOR, vec![]);
    assert!(check_resource_owner_by_member_scope(
        &dept_empty,
        Some(ACTOR)
    ));
    assert!(!check_resource_owner_by_member_scope(
        &dept_empty,
        Some(MEMBER)
    ));
    assert!(!check_resource_owner_by_member_scope(
        &dept_empty,
        Some(OUTSIDER)
    ));
    assert!(!check_resource_owner_by_member_scope(&dept_empty, None));

    // Self_：仅本人，他人/NULL 拒绝。
    let self_ctx = ctx(DataScope::Self_, ACTOR, vec![]);
    assert!(check_resource_owner_by_member_scope(&self_ctx, Some(ACTOR)));
    assert!(!check_resource_owner_by_member_scope(
        &self_ctx,
        Some(MEMBER)
    ));
    assert!(!check_resource_owner_by_member_scope(&self_ctx, None));
}
