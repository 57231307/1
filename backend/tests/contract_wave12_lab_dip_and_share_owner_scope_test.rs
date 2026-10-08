//! 化验室打样（lab_dip）与客户团队协作/共享（customer_team_share）行级归属门契约锁。
//!
//! IR 六要素：
//! - 目的（功能）：钉死"每个 lab_dip 写端点、每个团队/共享读端点在落库/出参前必须经
//!   `data_scope` 判定源，越权即 403+FORBIDDEN、绝不降级成空列表或静默 None"。
//! - 判据：本人 ⇒ 2xx 且回读如实（写族断言字段/状态真实变更）；他人 ⇒ 403+FORBIDDEN
//!   且写族回读断言零写入 / 状态零漂移；身份自报族另加"query 传他人 user_id 不得越权"。
//! - 依据：打样通知单/小样与各写端点按 `created_by` 走 `check_resource_owner(ctx, owner, None)`；
//!   团队成员/共享的客户维度读门借 customers 的真实 `owner_id`/`department_id` 同函数判定，
//!   身份自报主体则按 `to_data_scope_context` 的可见成员集合（`dept_member_user_ids`）判定。
//!   本锁以 self 范围会话取证"本人可、他人 403+FORBIDDEN"；无部门列的表在 Dept 范围下会被
//!   `check_resource_owner` 恒拒，该口径待裁，本文件不为 Dept 立断言。
//! - 夹具：真 PostgreSQL（`test_common::setup_test_db()`），禁 sqlite/mock/`#[ignore]`。
//! - 覆盖：PUT/DELETE 打样通知单、start-sampling/submit/approve/reject/restart/complete
//!   状态流转、PUT/DELETE 打样小样、团队成员 by-customer/by-user/check、共享
//!   by-customer/by-user/check 共 16 个站点（不含到期清理维护作业）。
//! - 边界：到期清理 `expire-overdue` 为全局维护作业、无归属主体，不在本锁范围。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{delete, get, post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::customer_team_share_handler::{
    check_share_permission, is_team_member, list_customer_shares, list_team_members,
    list_user_shares, list_user_teams,
};
use bingxi_backend::handlers::lab_dip_handler::{
    approve_ok_sample, complete_request, delete_request, delete_sample, reject_and_redo,
    restart_sampling, start_sampling, submit_to_customer, update_request, update_sample,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::lab_dip_request as req_status;
use bingxi_backend::models::status::lab_dip_sample as sample_status;
use bingxi_backend::models::{
    customer, customer_share, customer_team_member, lab_dip_request, lab_dip_sample, user,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const USER_A: i32 = 7101;
const USER_B: i32 = 7102;
// 部门 ID 取迁移种子已存在的 1（departments 为参照种子表，不自建；users/customers
// 的 department_id 外键指向它，用 7xxx 会触发 FK 违例）。
const DEPT_ID: i32 = 1;

const CUST_A: i32 = 7201;
const CUST_B: i32 = 7202;

const R_A_PENDING: i32 = 7301;
const R_A_SAMPLING: i32 = 7302;
const R_A_SUBMITTED: i32 = 7303;
const R_A_SUBMITTED_REJECT: i32 = 7304;
const R_A_REJECTED: i32 = 7305;
const R_A_APPROVED: i32 = 7306;
const R_B_PENDING: i32 = 7311;

const S_A_UPD: i32 = 7401;
const S_A_DEL: i32 = 7402;
const S_A_UNDER_SAMPLING: i32 = 7403;
const S_A_UNDER_SUBMITTED: i32 = 7404;
const S_B_PENDING: i32 = 7411;

const TEAM_A: i64 = 7501;
const TEAM_B: i64 = 7502;
const SHARE_A: i64 = 7601;
const SHARE_B: i64 = 7602;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("owner_scope_{user_id}"),
        role_id: Some(2),
        department_id: Some(DEPT_ID),
        // self 范围：跨归属一律拒；本人行放行。这是最紧的水平越权判据。
        data_scope: Some("self".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    req.extensions_mut().insert(auth.clone());
    next.run(req).await
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(v) => {
            let bytes = serde_json::to_vec(&v).unwrap();
            builder
                .header("content-type", "application/json")
                .body(Body::from(bytes))
                .unwrap()
        }
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
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

fn assert_403_forbidden(status: StatusCode, v: &Value, label: &str) {
    assert_eq!(status, StatusCode::FORBIDDEN, "{label} 必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN", "{label} 信封 code 契约: {v}");
    assert!(
        v["message"].is_string() && v["trace_id"].is_string(),
        "{label} 失败信封键齐全: {v}"
    );
}

async fn seed(db: &sea_orm::DatabaseConnection) {
    for uid in [USER_A, USER_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("owner_scope_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(DEPT_ID)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (cid, owner) in [(CUST_A, USER_A), (CUST_B, USER_B)] {
        customer::ActiveModel {
            id: Set(cid),
            customer_code: Set(format!("CUS-{cid}")),
            customer_name: Set(format!("归属门测试客户-{cid}")),
            credit_limit: Set(rust_decimal::Decimal::ZERO),
            payment_terms: Set(30),
            status: Set("active".to_string()),
            customer_type: Set("retail".to_string()),
            owner_id: Set(owner),
            department_id: Set(Some(DEPT_ID)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // (id, created_by, status, customer_id)
    for (rid, owner, status, cust) in [
        (R_A_PENDING, USER_A, req_status::PENDING, CUST_A),
        (R_A_SAMPLING, USER_A, req_status::SAMPLING, CUST_A),
        (R_A_SUBMITTED, USER_A, req_status::SUBMITTED, CUST_A),
        (R_A_SUBMITTED_REJECT, USER_A, req_status::SUBMITTED, CUST_A),
        (R_A_REJECTED, USER_A, req_status::REJECTED, CUST_A),
        (R_A_APPROVED, USER_A, req_status::APPROVED, CUST_A),
        (R_B_PENDING, USER_B, req_status::PENDING, CUST_B),
    ] {
        lab_dip_request::ActiveModel {
            id: Set(rid),
            request_no: Set(format!("LD-SEED-{rid}")),
            customer_id: Set(Some(cust)),
            light_source: Set("D65".to_string()),
            sample_versions: Set(4),
            required_date: Set(Utc::now().date_naive()),
            status: Set(status.to_string()),
            is_deleted: Set(false),
            created_by: Set(Some(owner)),
            created_at: Set(Utc::now().fixed_offset()),
            updated_at: Set(Utc::now().fixed_offset()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // (id, request_id, created_by, matching_result)
    for (sid, rid, owner, matching) in [
        (S_A_UPD, R_A_PENDING, USER_A, sample_status::PENDING),
        (S_A_DEL, R_A_PENDING, USER_A, sample_status::PENDING),
        (
            S_A_UNDER_SAMPLING,
            R_A_SAMPLING,
            USER_A,
            sample_status::PENDING,
        ),
        (
            S_A_UNDER_SUBMITTED,
            R_A_SUBMITTED,
            USER_A,
            sample_status::PENDING,
        ),
        (S_B_PENDING, R_B_PENDING, USER_B, sample_status::PENDING),
    ] {
        lab_dip_sample::ActiveModel {
            id: Set(sid),
            request_id: Set(rid),
            version_label: Set(format!("V{sid}")),
            version_seq: Set(1),
            matching_result: Set(matching.to_string()),
            resample_status: Set(Some("none".to_string())),
            is_deleted: Set(false),
            created_by: Set(Some(owner)),
            created_at: Set(Utc::now().fixed_offset()),
            updated_at: Set(Utc::now().fixed_offset()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 团队成员（表无 Default 派生，逐列 Set）
    for (tid, cid, uid, role) in [
        (TEAM_A, CUST_A, USER_A, "primary"),
        (TEAM_B, CUST_B, USER_B, "member"),
    ] {
        customer_team_member::ActiveModel {
            id: Set(tid),
            customer_id: Set(cid),
            user_id: Set(uid),
            user_name: Set(Some(format!("owner_scope_{uid}"))),
            team_role: Set(role.to_string()),
            is_active: Set(true),
            joined_at: Set(Utc::now()),
            left_at: Set(None),
            notes: Set(None),
            created_by: Set(Some(uid)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 共享记录（表无 Default 派生，逐列 Set）：A 收到 B 的共享、B 收到 A 的共享
    for (shid, cid, by_u, to_u, perm) in [
        (SHARE_A, CUST_A, USER_B, USER_A, "view"),
        (SHARE_B, CUST_B, USER_A, USER_B, "full"),
    ] {
        customer_share::ActiveModel {
            id: Set(shid),
            customer_id: Set(cid),
            shared_by_user_id: Set(by_u),
            shared_by_user_name: Set(Some(format!("owner_scope_{by_u}"))),
            shared_to_user_id: Set(to_u),
            shared_to_user_name: Set(Some(format!("owner_scope_{to_u}"))),
            permission: Set(perm.to_string()),
            status: Set(customer_share::SHARE_STATUS_ACTIVE.to_string()),
            shared_at: Set(Utc::now()),
            expire_at: Set(None),
            revoked_at: Set(None),
            revoked_by: Set(None),
            revoke_reason: Set(None),
            share_reason: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
        .insert(db)
        .await
        .unwrap();
    }
}

async fn build_app(auth: AuthContext) -> (Router, Arc<sea_orm::DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = Router::new()
        .route("/lab-dip/requests/{id}", put(update_request))
        .route("/lab-dip/requests/{id}", delete(delete_request))
        .route(
            "/lab-dip/requests/{id}/start-sampling",
            post(start_sampling),
        )
        .route("/lab-dip/requests/{id}/submit", post(submit_to_customer))
        .route("/lab-dip/requests/{id}/approve", post(approve_ok_sample))
        .route("/lab-dip/requests/{id}/reject", post(reject_and_redo))
        .route("/lab-dip/requests/{id}/restart", post(restart_sampling))
        .route("/lab-dip/requests/{id}/complete", post(complete_request))
        .route("/lab-dip/samples/{id}", put(update_sample))
        .route("/lab-dip/samples/{id}", delete(delete_sample))
        .route(
            "/customer-team-members/by-customer/{customer_id}",
            get(list_team_members),
        )
        .route("/customer-team-members/by-user", get(list_user_teams))
        .route("/customer-team-members/check", get(is_team_member))
        .route("/customer-shares/by-customer", get(list_customer_shares))
        .route("/customer-shares/by-user", get(list_user_shares))
        .route("/customer-shares/check", get(check_share_permission))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth));
    (app, db)
}

async fn reload_request(db: &sea_orm::DatabaseConnection, id: i32) -> lab_dip_request::Model {
    lab_dip_request::Entity::find_by_id(id)
        .filter(lab_dip_request::Column::IsDeleted.eq(false))
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("回读打样通知单 {id} 失败"))
}

async fn reload_sample(db: &sea_orm::DatabaseConnection, id: i32) -> lab_dip_sample::Model {
    lab_dip_sample::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("回读打样小样 {id} 失败"))
}

// ============================================================================
// lab_dip：更新打样通知单（PUT /requests/{id}）
// ============================================================================
// 修复前必红：handler 把会话写成 `_auth: AuthContext`，提取后丢弃，门不存在——
// 任意持 PUT /lab-dip/requests 权限键者对他人 R_B_PENDING 更新必返回 200 且落库，
// "他人 ⇒ 403" 与 "零漂移" 两条断言同时落空。

#[tokio::test]
async fn lab_dip_update_request_own_is_2xx_and_persists() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/lab-dip/requests/{R_A_PENDING}"),
        Some(json!({ "light_source": "OWNMARK" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 更新应 2xx: {v}");
    let row = reload_request(&db, R_A_PENDING).await;
    assert_eq!(row.light_source, "OWNMARK", "own 更新必须真实落库");
}

#[tokio::test]
async fn lab_dip_update_request_other_is_403_and_no_drift() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let before = reload_request(&db, R_B_PENDING).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/lab-dip/requests/{R_B_PENDING}"),
        Some(json!({ "light_source": "HACKMARK" })),
    )
    .await;
    assert_403_forbidden(status, &v, "他人更新打样通知单");
    let after = reload_request(&db, R_B_PENDING).await;
    assert_eq!(
        after.light_source, before.light_source,
        "403 后他人行不得被写入"
    );
    assert_eq!(after.status, before.status, "403 后他人行状态不得漂移");
}

// ============================================================================
// lab_dip：软删除打样通知单（DELETE /requests/{id}，仅 pending 可删）
// ============================================================================
// 修复前必红：`_auth` 丢弃 ⇒ 无门 ⇒ 他人 pending 单被软删（is_deleted=true）→ 200，
// 断言 "他人 ⇒ 403 + is_deleted 仍为 false" 必红。

#[tokio::test]
async fn lab_dip_delete_request_own_is_2xx_and_soft_deleted() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/lab-dip/requests/{R_A_PENDING}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 删除应 2xx: {v}");
    // 软删除后 get_by_id 过滤 is_deleted=false 查不到；直读全行确认已置位
    let row = lab_dip_request::Entity::find_by_id(R_A_PENDING)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert!(row.is_deleted, "own 删除必须落库 is_deleted=true");
}

#[tokio::test]
async fn lab_dip_delete_request_other_is_403_and_not_deleted() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/lab-dip/requests/{R_B_PENDING}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "他人删除打样通知单");
    let after = lab_dip_request::Entity::find_by_id(R_B_PENDING)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert!(!after.is_deleted, "403 后他人行不得被软删");
}

// ============================================================================
// lab_dip：状态流转 start-sampling / submit / approve / reject / restart / complete
// ============================================================================
// 修复前必红：`_auth` 丢弃 ⇒ 无门 ⇒ 他人 R_B_PENDING（pending 态）被任意流转，
// "他人 ⇒ 403 + 状态零漂移" 必红。

#[tokio::test]
async fn lab_dip_start_sampling_own_is_2xx_and_status_moves() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_A_PENDING}/start-sampling"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 开始打样应 2xx: {v}");
    assert_eq!(
        reload_request(&db, R_A_PENDING).await.status,
        req_status::SAMPLING,
        "own 流转必须真实推进状态"
    );
}

#[tokio::test]
async fn lab_dip_start_sampling_other_is_403_and_status_frozen() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_B_PENDING}/start-sampling"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "他人流转 start-sampling");
    assert_eq!(
        reload_request(&db, R_B_PENDING).await.status,
        req_status::PENDING,
        "403 后他人单状态不得漂移"
    );
}

#[tokio::test]
async fn lab_dip_submit_own_is_2xx_and_status_moves() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_A_SAMPLING}/submit"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own submit 应 2xx: {v}");
    assert_eq!(
        reload_request(&db, R_A_SAMPLING).await.status,
        req_status::SUBMITTED,
        "own submit 必须推进到 submitted"
    );
}

#[tokio::test]
async fn lab_dip_submit_other_is_403_and_status_frozen() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_B_PENDING}/submit"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "他人 submit 越权");
    assert_eq!(
        reload_request(&db, R_B_PENDING).await.status,
        req_status::PENDING,
        "403 后他人单状态不得漂移"
    );
}

#[tokio::test]
async fn lab_dip_approve_own_is_2xx_and_status_moves() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_A_SUBMITTED}/approve"),
        Some(json!({ "sample_id": S_A_UNDER_SUBMITTED, "comment": "OK" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own approve 应 2xx: {v}");
    assert_eq!(
        reload_request(&db, R_A_SUBMITTED).await.status,
        req_status::APPROVED,
        "own approve 必须推进到 approved"
    );
    assert_eq!(
        reload_sample(&db, S_A_UNDER_SUBMITTED)
            .await
            .matching_result,
        sample_status::SELECTED,
        "选中 OK 样必须置 selected"
    );
}

#[tokio::test]
async fn lab_dip_approve_other_is_403_and_status_frozen() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_B_PENDING}/approve"),
        Some(json!({ "sample_id": S_B_PENDING, "comment": "hack" })),
    )
    .await;
    assert_403_forbidden(status, &v, "他人 approve 越权");
    assert_eq!(
        reload_request(&db, R_B_PENDING).await.status,
        req_status::PENDING,
        "403 后他人单状态不得漂移"
    );
}

#[tokio::test]
async fn lab_dip_reject_own_is_2xx_and_status_moves() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_A_SUBMITTED_REJECT}/reject"),
        Some(json!({ "comment": "重打" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own reject 应 2xx: {v}");
    assert_eq!(
        reload_request(&db, R_A_SUBMITTED_REJECT).await.status,
        req_status::REJECTED,
        "own reject 必须推进到 rejected"
    );
}

#[tokio::test]
async fn lab_dip_reject_other_is_403_and_status_frozen() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_B_PENDING}/reject"),
        Some(json!({ "comment": "hack" })),
    )
    .await;
    assert_403_forbidden(status, &v, "他人 reject 越权");
    assert_eq!(
        reload_request(&db, R_B_PENDING).await.status,
        req_status::PENDING,
        "403 后他人单状态不得漂移"
    );
}

#[tokio::test]
async fn lab_dip_restart_own_is_2xx_and_status_moves() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_A_REJECTED}/restart"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own restart 应 2xx: {v}");
    assert_eq!(
        reload_request(&db, R_A_REJECTED).await.status,
        req_status::SAMPLING,
        "own restart 必须回到 sampling"
    );
}

#[tokio::test]
async fn lab_dip_restart_other_is_403_and_status_frozen() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_B_PENDING}/restart"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "他人 restart 越权");
    assert_eq!(
        reload_request(&db, R_B_PENDING).await.status,
        req_status::PENDING,
        "403 后他人单状态不得漂移"
    );
}

#[tokio::test]
async fn lab_dip_complete_own_is_2xx_and_status_moves() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_A_APPROVED}/complete"),
        Some(json!({ "production_recipe_id": 8801 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own complete 应 2xx: {v}");
    let row = reload_request(&db, R_A_APPROVED).await;
    assert_eq!(
        row.status,
        req_status::COMPLETED,
        "own complete 必须建库完成"
    );
    assert_eq!(
        row.production_recipe_id,
        Some(8801),
        "own complete 必须写入 production_recipe_id"
    );
}

#[tokio::test]
async fn lab_dip_complete_other_is_403_and_status_frozen() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/lab-dip/requests/{R_B_PENDING}/complete"),
        Some(json!({ "production_recipe_id": 9999 })),
    )
    .await;
    assert_403_forbidden(status, &v, "他人 complete 越权");
    let after = reload_request(&db, R_B_PENDING).await;
    assert_eq!(
        after.status,
        req_status::PENDING,
        "403 后他人单状态不得漂移"
    );
    assert_eq!(
        after.production_recipe_id, None,
        "403 后他人单不得被写入配方"
    );
}

// ============================================================================
// lab_dip：打样小样 PUT/DELETE（归属依据 lab_dip_sample.created_by）
// ============================================================================
// 修复前必红：`_auth` 丢弃 ⇒ 无门 ⇒ 他人小样被改/删，断言 "他人 ⇒ 403 + 零漂移" 必红。

#[tokio::test]
async fn lab_dip_update_sample_own_is_2xx_and_persists() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/lab-dip/samples/{S_A_UPD}"),
        Some(json!({ "remarks": "OWNMARK" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 更新小样应 2xx: {v}");
    assert_eq!(
        reload_sample(&db, S_A_UPD).await.remarks.as_deref(),
        Some("OWNMARK"),
        "own 更新必须真实落库"
    );
}

#[tokio::test]
async fn lab_dip_update_sample_other_is_403_and_no_drift() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let before = reload_sample(&db, S_B_PENDING).await;
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/lab-dip/samples/{S_B_PENDING}"),
        Some(json!({ "remarks": "HACKMARK" })),
    )
    .await;
    assert_403_forbidden(status, &v, "他人更新小样");
    let after = reload_sample(&db, S_B_PENDING).await;
    assert_eq!(
        after.remarks, before.remarks,
        "403 后他人小样不得被写入 remarks"
    );
    assert_eq!(
        after.matching_result, before.matching_result,
        "403 后他人小样对色状态不得漂移"
    );
}

#[tokio::test]
async fn lab_dip_delete_sample_own_is_2xx_and_soft_deleted() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/lab-dip/samples/{S_A_DEL}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 删除小样应 2xx: {v}");
    assert!(
        reload_sample(&db, S_A_DEL).await.is_deleted,
        "own 删除必须落库 is_deleted=true"
    );
}

#[tokio::test]
async fn lab_dip_delete_sample_other_is_403_and_not_deleted() {
    let (app, db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/lab-dip/samples/{S_B_PENDING}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "他人删除小样");
    assert!(
        !reload_sample(&db, S_B_PENDING).await.is_deleted,
        "403 后他人小样不得被软删"
    );
}

// ============================================================================
// customer_team_share：by-customer（客户维度读门）
// ============================================================================
// 修复前必红：`_auth` 丢弃 ⇒ 无客户归属门 ⇒ 任意 customer_id 枚举他人团队成员/共享，
// 断言 "他人客户 ⇒ 403" 必红，本人客户 "2xx 且非空" 也因原先无门而无对照锁定。

#[tokio::test]
async fn share_list_team_members_own_customer_is_2xx_and_nonempty() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-team-members/by-customer/{CUST_A}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人客户团队成员应 2xx: {v}");
    assert!(
        !v["data"].as_array().unwrap().is_empty(),
        "本人客户应能列出自己的团队成员行"
    );
}

#[tokio::test]
async fn share_list_team_members_other_customer_is_403() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-team-members/by-customer/{CUST_B}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "他人客户团队成员");
    // 关键：越权不得降级成看着正常的空列表
    assert_ne!(v["code"], "OK", "越权读必须显式拒绝而非返回空 data");
}

#[tokio::test]
async fn share_list_customer_shares_own_is_2xx() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-shares/by-customer?customer_id={CUST_A}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人客户共享应 2xx: {v}");
}

#[tokio::test]
async fn share_list_customer_shares_other_is_403() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-shares/by-customer?customer_id={CUST_B}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "他人客户共享列表");
}

// ============================================================================
// customer_team_share：by-user / check（身份自报主体可见性门）
// ============================================================================
// 修复前必红：`_auth` 丢弃 + `q.user_id` 被当作身份来源 ⇒ 自报他人 user_id 即可
// 枚举他人团队/共享拓扑。断言 "query 传他人 user_id ⇒ 403（不得越权、不得返回他人数据）" 必红。

#[tokio::test]
async fn share_list_user_teams_self_is_2xx() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-team-members/by-user?user_id={USER_A}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人团队列表应 2xx: {v}");
    assert!(
        !v["data"].as_array().unwrap().is_empty(),
        "本人应能列出自己参与的团队行"
    );
}

#[tokio::test]
async fn share_list_user_teams_other_user_id_is_403() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-team-members/by-user?user_id={USER_B}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "自报他人 user_id 列团队");
}

#[tokio::test]
async fn share_is_team_member_self_is_2xx() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-team-members/check?customer_id={CUST_A}&user_id={USER_A}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人成员身份检查应 2xx: {v}");
    assert_eq!(v["data"], json!("primary"), "本人主负责人结果如实: {v}");
}

#[tokio::test]
async fn share_is_team_member_other_user_id_is_403() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-team-members/check?customer_id={CUST_B}&user_id={USER_B}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "自报他人 user_id 探测成员拓扑");
}

#[tokio::test]
async fn share_list_user_shares_self_is_2xx() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-shares/by-user?user_id={USER_A}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人收到共享应 2xx: {v}");
    assert!(
        !v["data"].as_array().unwrap().is_empty(),
        "本人应能列出自己收到的共享行"
    );
}

#[tokio::test]
async fn share_list_user_shares_other_user_id_is_403() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-shares/by-user?user_id={USER_B}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "自报他人 user_id 看他人共享列表");
}

#[tokio::test]
async fn share_check_permission_self_is_2xx() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-shares/check?customer_id={CUST_A}&user_id={USER_A}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "本人共享权限检查应 2xx: {v}");
    assert_eq!(v["data"], json!("view"), "本人被共享 view 结果如实: {v}");
}

#[tokio::test]
async fn share_check_permission_other_user_id_is_403() {
    let (app, _db) = build_app(make_auth(USER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/customer-shares/check?customer_id={CUST_B}&user_id={USER_B}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "自报他人 user_id 探测授权拓扑");
}
