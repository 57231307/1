//! 收货确认权限键的活体门层锁：持 `purchase-receipts:confirm` 才可能过门
//!
//! 功能：在真已迁移 PostgreSQL 上，用真 `permission_middleware` 装配 `POST
//!       /api/v1/erp/purchase/receipts/{id}/confirm`，钉两件事——① 只授该一枚键的角色
//!       不会被门层判 403（放行到业务侧）；② 同结构零授权的角色必被门层 403 +
//!       机器码 `FORBIDDEN`。
//! 调用方：集成测试 crate（CI 里对已跑完迁移链的 PostgreSQL 执行）。
//! 入参：`test_common::setup_test_db()` 注入库连接；角色/操作人由本文件自建自清。
//! 传给谁：判权结果交给 `middleware::permission::permission_middleware`，业务侧
//!       `handlers::purchase_receipt_handler::confirm_receipt` 只作为"已过门"的落点。
//! 存什么：只种两个测试角色（其一持 confirm 授权行）、一个测试操作人；不写业务表。
//! 存哪里：PostgreSQL `roles` / `role_permissions` / `users`（夹具不清这三张参照表，
//!       故用例开头先按角色码收敛起点再重种）。
//!
//! 为什么正向断 404 而不是断 200：本锁的对象是"门层按哪枚键放行"。收货单不存在时业务侧
//! 必然给 NOT_FOUND，故"404 + NOT_FOUND"恰好等价于"权限门已放行、且没有把 500/422 之类
//! 意外也算成放行"。真实状态流转与库存落库的判据由流转门活体锁覆盖，不在本文件重复。
//!
//! 反空操作自证：修复前矩阵授的是 approve（该资源没有 approve 端点），confirm 键无人持有
//! ⇒ 两向都会拿到 403，本文件的正向断言必红；反向断言若被写成"也放行"则立即红。

mod test_common;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::purchase_receipt_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{invalidate_permission_cache, permission_middleware};
use bingxi_backend::models::{role, role_permission, user};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

const GRANTEE_ROLE_CODE: &str = "it_w12_rcpt_confirm_grantee";
const STRANGER_ROLE_CODE: &str = "it_w12_rcpt_confirm_stranger";
const SEED_USER_ID: i32 = 9463;
/// 不存在的收货单 ID：门层放行后必然落到"记录不存在"，绝不会误判成权限放行。
const MISSING_RECEIPT_ID: i32 = 946300;

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// 清掉上一轮可能残留的测试角色与其授权行，保证起点收敛（参照表不被夹具 TRUNCATE）。
async fn reset_fixture_roles(db: &DatabaseConnection) {
    let role_ids: Vec<i32> = role::Entity::find()
        .filter(role::Column::Code.is_in([GRANTEE_ROLE_CODE, STRANGER_ROLE_CODE]))
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    if !role_ids.is_empty() {
        role_permission::Entity::delete_many()
            .filter(role_permission::Column::RoleId.is_in(role_ids.clone()))
            .exec(db)
            .await
            .unwrap();
    }
    role::Entity::delete_many()
        .filter(role::Column::Code.is_in([GRANTEE_ROLE_CODE, STRANGER_ROLE_CODE]))
        .exec(db)
        .await
        .unwrap();
}

/// 建一个 data_scope=self 的测试角色，返回 role_id。
async fn seed_role(db: &DatabaseConnection, code: &str) -> i32 {
    let model = role::ActiveModel {
        name: Set(format!("收货确认门锁角色 {code}")),
        code: Set(code.to_string()),
        is_system: Set(false),
        // 用 all 让业务侧的行级归属门不参与，留下的唯一判据就是权限键本身
        data_scope: Set("all".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子角色 {code} 失败: {e}"));
    model.id
}

/// 角色级 allowed=true 授权行（resource_id=NULL，与矩阵落库形态逐列一致）。
async fn seed_role_level_grant(
    db: &DatabaseConnection,
    role_id: i32,
    resource: &str,
    action: &str,
) {
    role_permission::ActiveModel {
        role_id: Set(role_id),
        resource_type: Set(resource.to_string()),
        resource_id: Set(None),
        action: Set(action.to_string()),
        allowed: Set(true),
        permission_code: Set(None),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子授权行 {resource}:{action} 失败: {e}"));
}

/// 带真实 permission_middleware 的活体 Router：路径含 /api/v1/erp 前缀段与 purchase/receipts
/// 两段，使资源键按生产同一链路派生成 purchase-receipts:confirm。
fn app_with_role(db: Arc<DatabaseConnection>, role_id: i32) -> axum::Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    async fn inject_auth(
        auth: axum::extract::State<AuthContext>,
        mut request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let auth = AuthContext {
        user_id: SEED_USER_ID,
        username: "it_w12_rcpt_confirm".to_string(),
        role_id: Some(role_id),
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    axum::Router::new()
        .route(
            "/api/v1/erp/purchase/receipts/{id}/confirm",
            axum::routing::post(purchase_receipt_handler::confirm_receipt),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
        .with_state(state)
}

/// 以该角色会话 POST 确认端点，返回 HTTP 码与解析后的信封。
async fn post_confirm(app: &axum::Router, id: i32) -> (axum::http::StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(format!("/api/v1/erp/purchase/receipts/{id}/confirm"))
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

/// 活体双向：仅持 confirm 键的角色不被门层拦（落到业务侧），零授权的同结构角色必 403。
#[tokio::test]
async fn confirm_key_is_what_the_gate_checks() {
    let db = Arc::new(test_common::setup_test_db().await);
    reset_fixture_roles(db.as_ref()).await;

    if user::Entity::find_by_id(SEED_USER_ID)
        .one(db.as_ref())
        .await
        .unwrap()
        .is_none()
    {
        user::ActiveModel {
            id: Set(SEED_USER_ID),
            username: Set("it_w12_rcpt_confirm".to_string()),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            real_name: Set(Some("收货确认门锁操作人".to_string())),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("种子操作人失败: {e}"));
    }

    let grantee = seed_role(&db, GRANTEE_ROLE_CODE).await;
    let stranger = seed_role(&db, STRANGER_ROLE_CODE).await;
    // grantee 只有 confirm 一枚键：证明放行的正是本轮补到真实键上的授权
    seed_role_level_grant(&db, grantee, "purchase-receipts", "confirm").await;
    invalidate_permission_cache(grantee);
    invalidate_permission_cache(stranger);

    let (status, body) =
        post_confirm(&app_with_role(db.clone(), grantee), MISSING_RECEIPT_ID).await;
    // 门层放行后必然落到业务侧"记录不存在"（夹具已 TRUNCATE 收货单，且 data_scope=all
    // 让行级归属门不参与）——据此把"权限门放行"与"权限门拒绝"一刀分开，
    // 而不是只断"不是 403"那种把 500/422 也当放行的弱判据。
    assert_eq!(
        status,
        axum::http::StatusCode::NOT_FOUND,
        "持 confirm 键的角色应穿过权限门进到业务侧（缺行必 404）；被门层拦则实得 403，实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("NOT_FOUND"),
        "放行侧的机器码须为 NOT_FOUND（绝不得是 FORBIDDEN），实得 {body}"
    );

    let (status, body) =
        post_confirm(&app_with_role(db.clone(), stranger), MISSING_RECEIPT_ID).await;
    assert_eq!(
        status,
        axum::http::StatusCode::FORBIDDEN,
        "不持 confirm 键的同结构角色必须被门层真实拒绝，实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("FORBIDDEN"),
        "403 必须是统一信封机器码 FORBIDDEN（只断码不断文案），实得 {body}"
    );

    reset_fixture_roles(db.as_ref()).await;
    invalidate_permission_cache(grantee);
    invalidate_permission_cache(stranger);
}
