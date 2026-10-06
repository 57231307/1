//! 后端安全「建单人只认服务端会话」行为级活体证明 —— 第六批（固定资产 / 出口商检 / 客户建档）
//!
//! 锁定的契约：以下创建型端点的审计归属列 `created_by` 唯一来源是 `AuthContext.user_id`
//! （服务端会话）；请求体不承载该身份字段，body 里伪造的 `created_by`/`user_id`/`operator_id`
//! 被 serde 忽略，绝不落库。
//! - `POST /fixed-assets/depreciation-policy-changes`（`depreciation_policy_changes.created_by`，
//!   DDL `INTEGER NOT NULL`：`backend/migration/src/domain/v15/mod.rs` 的
//!   `depreciation_policy_changes` 建表段）；
//! - `POST /export-inspections`（`export_inspection.created_by`，DDL `INTEGER NOT NULL`：
//!   `backend/migration/src/domain/v15/mod.rs` 的 `export_inspection` 建表段）；
//! - `POST /customers`（`customers.created_by` 可空且有 users 外键，见 `seed_users` 注释；
//!   模型 `backend/src/models/customer.rs` 的 `created_by` 字段为 `Option<i32>`）。
//!
//! 证明形态（夹具范式同 contract_wave11_created_by_from_session_batch1）：
//! ① 会话注入用户 A（`from_fn_with_state(make_auth(A), inject_auth)`）；
//! ② 请求体故意携带伪造用户 B 的 `created_by`/`user_id`/`operator_id`；
//! ③ 动作必须成功（身份键非必填、缺省合法）；④ `Entity::find_by_id` 直读列断言归 A 且显式 `!= B`。
//!
//! 另含形态锁：逐个定位并断言这批入参结构体/前端请求声明不再承载身份字段
//! （`CreateDepreciationPolicyChangeDto`、`CreateInspectionBody`、`CreateCustomerRequest`，
//! 以及前端 `frontend/src/api/asset.ts::batchDepreciateAssets` 的 payload 声明）；
//! 任一取不到即 panic（检测力地板，禁止空集合=绿）。
//!
//! 检测力（改坏即红的点位）：
//! - handler 退回从 body 取身份 → 三条直读断言当场红；
//! - service 退回 `Set(req.user_id)` / `Set(data.created_by)` 且值来自 body → 断言红；
//! - 身份字段被加回任一 DTO / 前端 payload → 形态锁当场红；
//! - 前端批量计提重新上送 `user_id` 并带「取不到本地用户 id 就中止」分支 → `asset.ts` 形态锁红；
//! - 形态锁解析不到目标结构体 → 地板断言当场红（防空集合假绿）。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{customer_handler, export_inspection_handler, fixed_asset_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{customer, depreciation_policy_change, export_inspection};
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话用户 A：AuthContext.user_id 注入的合法建单人（与同族锁同一对常量）
const SESSION_A: i32 = 9511;
/// 伪造用户 B：只出现在请求体里，永远不允许落进任何 created_by 列
const FORGED_B: i32 = 9472;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w11_createdby_b6_{user_id}"),
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

fn pg_stmt(sql: String) -> Statement {
    Statement::from_string(DbBackend::Postgres, sql)
}

async fn exec(db: &DatabaseConnection, sql: String) {
    db.execute_raw(pg_stmt(sql.clone()))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL 原文: {sql}"));
}

/// users 自建（`customers.created_by` / `owner_id` 的外键父行；users 不在迁移种子里）。
/// A、B 都建：B 存在只为了让「伪造值被误当会话值插入」这种退化仍会落到 FK 之后，
/// 由值断言而非约束报错来判红。
async fn seed_users(db: &DatabaseConnection) {
    for id in [SESSION_A, FORGED_B] {
        exec(
            db,
            format!("DELETE FROM users WHERE id={id} OR username='b6_createdby_{id}'"),
        )
        .await;
        exec(
            db,
            format!(
                "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at)
                 VALUES ({id},'b6_createdby_{id}','test-only-not-a-real-hash',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
            ),
        )
        .await;
    }
}

/// 以 SESSION_A 为会话用户组装带 auth 注入层的 Router（路由按真实挂载形状注册）
fn layered_app(
    state: AppState,
    routes: Vec<(&'static str, axum::routing::MethodRouter<AppState>)>,
) -> Router {
    let mut router: Router<AppState> = Router::new();
    for (path, method_route) in routes {
        router = router.route(path, method_route);
    }
    router
        .with_state(state)
        .layer(from_fn_with_state(make_auth(SESSION_A), inject_auth))
}

/// 请求体统一掺入伪造身份键（handler 不认的键 serde 直接忽略——正是要证明「发了也无效」）
fn with_forged_identity(mut obj: serde_json::Map<String, Value>) -> String {
    obj.insert("created_by".to_string(), Value::from(FORGED_B));
    obj.insert("operator_id".to_string(), Value::from(FORGED_B));
    obj.insert("user_id".to_string(), Value::from(FORGED_B));
    serde_json::to_string(&Value::Object(obj)).expect("伪造身份体必须是合法 JSON（夹具自证）")
}

async fn call_post(app: &Router, path: &str, body: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// 成功响应取新建行 id，并断动作真实成功（缺身份键合法、伪造键被忽略）
fn created_id(status: StatusCode, v: &Value, what: &str) -> i32 {
    assert_eq!(
        status,
        StatusCode::OK,
        "{what} 必须成功（created_by 不是必填入参），实际: {status} {v}"
    );
    assert_eq!(v["code"], 200, "{what} 成功信封 code 必须为 200, 实际: {v}");
    v["data"]["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("{what} 出参必须携带新行 id, 实际: {v}")) as i32
}

/// 可空审计列（customers.created_by）：直读值必须归会话 A，绝不归伪造 B
fn assert_nullable_created_by_is_session(row_created_by: Option<i32>, what: &str) {
    assert_eq!(
        row_created_by,
        Some(SESSION_A),
        "直读实体：{what} 建单人必须等于会话用户 A，实际 {row_created_by:?}"
    );
    assert_ne!(
        row_created_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进 {what} 的建单人列"
    );
}

/// NOT NULL 审计列（depreciation_policy_changes / export_inspection）：同上，按真实可空性取 i32
fn assert_created_by_is_session(row_created_by: i32, what: &str) {
    assert_eq!(
        row_created_by, SESSION_A,
        "直读实体：{what} 建单人必须等于会话用户 A，实际 {row_created_by}"
    );
    assert_ne!(
        row_created_by, FORGED_B,
        "直读实体：body 伪造的 B 绝不许落进 {what} 的建单人列"
    );
}

macro_rules! read_row {
    ($mod:ident, $id:expr, $db:expr, $what:literal) => {
        $mod::Entity::find_by_id($id)
            .one($db)
            .await
            .unwrap_or_else(|e| panic!(concat!($what, " 直读失败: {}"), e))
            .unwrap_or_else(|| panic!(concat!($what, " 必须存在")))
    };
}

// =========================================================
// 域 1：固定资产折旧政策变更（fixed_asset_handler）
// =========================================================

#[tokio::test]
async fn depreciation_policy_change_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/fixed-assets/depreciation-policy-changes",
            post(fixed_asset_handler::create_depreciation_policy_change),
        )],
    );
    let mut body = serde_json::Map::new();
    body.insert("asset_id".to_string(), json!(990001));
    body.insert("change_date".to_string(), json!("2026-09-01"));
    body.insert("old_method".to_string(), json!("straight_line"));
    body.insert("new_method".to_string(), json!("double_declining_balance"));
    body.insert("old_useful_life".to_string(), json!(10));
    body.insert("new_useful_life".to_string(), json!(8));
    body.insert("reason".to_string(), json!("建单人会话锁（batch6）"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/fixed-assets/depreciation-policy-changes", &body).await;
    let id = created_id(status, &v, "折旧政策变更建单");

    let row = read_row!(depreciation_policy_change, id, &read_db, "折旧政策变更");
    assert_created_by_is_session(row.created_by, "折旧政策变更");
    assert_eq!(row.status, "pending", "业务状态列不受身份改造牵连");
    assert_eq!(
        row.new_method, "double_declining_balance",
        "业务列不受牵连（变更方法照常落库）"
    );
}

// =========================================================
// 域 2：出口商检建单（export_inspection_handler）
// =========================================================

#[tokio::test]
async fn export_inspection_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/export-inspections",
            post(export_inspection_handler::create_inspection),
        )],
    );
    let inspection_no = format!(
        "EI-W11CB6-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    );
    let mut body = serde_json::Map::new();
    body.insert("inspection_no".to_string(), json!(inspection_no.clone()));
    body.insert("sales_order_id".to_string(), json!(990002));
    body.insert("delivery_id".to_string(), json!(null));
    body.insert("product_name".to_string(), json!("棉制针织衫"));
    body.insert("hs_code".to_string(), json!("6109100021"));
    body.insert("inspection_type".to_string(), json!("pre_export"));
    body.insert("inspection_agency".to_string(), json!("中检集团"));
    body.insert("inspection_date".to_string(), json!("2026-09-01"));
    body.insert("remarks".to_string(), json!("建单人会话锁（batch6）"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/export-inspections", &body).await;
    let id = created_id(status, &v, "出口商检建单");

    let row = read_row!(export_inspection, id, &read_db, "出口商检单");
    assert_created_by_is_session(row.created_by, "出口商检单");
    assert_eq!(
        row.result, "pending",
        "建单仍恒落待检态（结论列不受身份改造牵连）"
    );
    assert_eq!(row.inspection_no, inspection_no, "业务列不受牵连");
}

// =========================================================
// 域 3：客户建档（customer_handler，可空审计列 + users 外键）
// =========================================================

#[tokio::test]
async fn customer_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    seed_users(&db).await;
    let read_db = db.clone();
    let mut state = AppState::default();
    state.db = Arc::new(db);
    state.data_permission_service = Arc::new(DataPermissionService::new(state.db.clone()));
    let app = layered_app(
        state,
        vec![("/customers", post(customer_handler::create_customer))],
    );
    let customer_code = format!("CB6-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap());
    let mut body = serde_json::Map::new();
    body.insert("customer_code".to_string(), json!(customer_code.clone()));
    body.insert("customer_name".to_string(), json!("建单人会话锁客户"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/customers", &body).await;
    let id = created_id(status, &v, "客户建档");

    let row = read_row!(customer, id, &read_db, "客户");
    assert_nullable_created_by_is_session(row.created_by, "客户建档");
    assert_eq!(row.customer_code, customer_code, "业务列不受牵连");
}

// =========================================================
// 形态锁：这批入参结构体 / 前端请求声明不再承载身份字段
// =========================================================

/// 在源码文本中定位 `pub struct NAME { ... }` 的结构体体文本。
/// 找不到即返回 `None`——由调用方按地板判红，禁止静默跳过。
fn struct_body(src: &str, name: &str) -> Option<String> {
    let marker = format!("pub struct {name} {{");
    let start = src.find(&marker)? + marker.len();
    let rest = &src[start..];
    let end = rest.find("\n}")?;
    Some(rest[..end].to_string())
}

/// 在 TS 源码中定位 `export const NAME = (data: { ... }) => request.post(` 的 payload 声明文本。
fn ts_payload(src: &str, name: &str) -> Option<String> {
    let marker = format!("export const {name} = (data: {{");
    let start = src.find(&marker)? + marker.len();
    let rest = &src[start..];
    let end = rest.find("})")?;
    Some(rest[..end].to_string())
}

#[test]
fn batch6_request_dtos_have_no_identity_field() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // (源码相对路径, 结构体/声明名, 禁止出现的身份键)
    let targets: [(&str, &str, &[&str]); 4] = [
        (
            "src/handlers/fixed_asset_handler.rs",
            "CreateDepreciationPolicyChangeDto",
            &["created_by", "user_id", "operator_id"],
        ),
        (
            "src/handlers/export_inspection_handler.rs",
            "CreateInspectionBody",
            &["created_by", "user_id", "operator_id"],
        ),
        (
            "src/handlers/customer_handler.rs",
            "CreateCustomerRequest",
            &["created_by", "user_id", "operator_id"],
        ),
        (
            "../frontend/src/api/asset.ts",
            "batchDepreciateAssets",
            &["user_id", "created_by"],
        ),
    ];
    let mut parsed = 0usize;
    for (file, name, forbidden) in targets.iter() {
        let path = std::path::Path::new(manifest_dir).join(file);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", path.display()));
        let body = if file.ends_with(".ts") {
            ts_payload(&src, name).unwrap_or_else(|| {
                panic!("形态锁取不到前端请求声明 {name}（检测力地板，禁止空集合=绿）")
            })
        } else {
            struct_body(&src, name)
                .unwrap_or_else(|| panic!("形态锁取不到结构体 {name}（检测力地板，禁止空集合=绿）"))
        };
        parsed += 1;
        for key in forbidden.iter() {
            assert!(
                !body.contains(key),
                "{name}（{file}）不应承载身份入参键 `{key}`，实际声明体: {body}"
            );
        }
    }
    assert_eq!(
        parsed,
        targets.len(),
        "本组 {} 个入参声明必须全部被解析到（检测力地板），实际 {parsed}",
        targets.len()
    );
}
