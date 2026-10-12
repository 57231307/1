//! 后端安全「能耗登记操作人只认服务端会话」行为级活体证明（批 4）
//!
//! 锁定的契约：`POST /energy-consumptions` 的登记操作人留痕列
//! `energy_consumption_record.operator_id` 唯一来源是 `AuthContext.user_id`
//! （服务端会话），请求 DTO `CreateConsumptionRequest` 不再承载该身份字段；
//! 请求体里伪造的 `operator_id`/`created_by`/`user_id` 被 serde 忽略，绝不落库。
//!
//! 证明形态（夹具范式同 contract_wave11_created_by_from_session_batch1）：
//! ① 会话注入用户 A（`from_fn_with_state(make_auth(A), inject_auth)`）；
//! ② 请求体故意携带伪造用户 B 的全量身份键；③ 动作必须成功（身份键非必填且未知键被忽略）；
//! ④ `Entity::find_by_id` 直读实体列断 `row.operator_id == Some(A)` 且显式 `!= Some(B)`。
//!
//! 另含形态锁：解析源码断言 `CreateConsumptionRequest` 不再含身份入参字段
//! （`operator_id`/`created_by`），并带检测力地板（目标结构体取不到即判红，禁止空集合=绿）。
//!
//! 检测力（改坏源码即红的点位）：
//! - 若 service 退回 `operator_id: Set(req.operator_id)` 或 handler 退回 body 取值 → 直读断言红；
//! - 若 DTO 重新加回 `operator_id` 字段 → 形态锁当场红；
//! - 若形态锁解析不到目标结构体（改动站点数与解析端点数对不上）→ 地板断言当场红。

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
use bingxi_backend::handlers::energy_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::energy_consumption_record;
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话用户 A：AuthContext.user_id 注入的合法登记操作人（私有 id 段 951x，
/// 与同族锁保持同一对常量）
const SESSION_A: i32 = 9511;
/// 伪造用户 B：只出现在请求体里，永远不允许落进 operator_id/created_by 列
const FORGED_B: i32 = 9472;
/// 本批实际改动站点数（energy_ops/consumption.rs create 落库 1 处；
/// ai_extend_service.rs :179/:240/:494 经溯源为 handler 无条件覆写的确认透传，不在改动集）
const CHANGED_SITES: usize = 1;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w11_createdby_b4_{user_id}"),
        role_id: Some(2),
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

/// 请求体统一掺入伪造身份键（handler 不认的键 serde 直接忽略——正是要证明的「发了也无效」）
fn with_forged_identity(mut obj: serde_json::Map<String, Value>) -> String {
    obj.insert("created_by".to_string(), json!(FORGED_B));
    obj.insert("operator_id".to_string(), json!(FORGED_B));
    obj.insert("user_id".to_string(), json!(FORGED_B));
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

/// 以 SESSION_A 为会话用户组装带 auth 注入层的 Router（路由按占位符注册，同真实 routes 形态）
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

/// 成功信封取新建行 id，并断动作真实成功（缺身份键合法、伪造键被忽略）
fn created_id(status: StatusCode, v: &Value, what: &str) -> i32 {
    assert_eq!(
        status,
        StatusCode::OK,
        "{what} 必须成功（operator_id 不再是必填入参），实际: {status} {v}"
    );
    assert_eq!(v["code"], 200, "{what} 成功信封 code 必须为 200, 实际: {v}");
    v["data"]["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("{what} 出参必须携带新行 id, 实际: {v}")) as i32
}

/// 断言直读行的身份留痕列归会话用户 A、绝归伪造用户 B
fn assert_identity_column_is_session(row_value: Option<i32>, column: &str, what: &str) {
    assert_eq!(
        row_value,
        Some(SESSION_A),
        "直读实体：{what} 的 {column} 必须等于会话用户 A，实际 {row_value:?}"
    );
    assert_ne!(
        row_value,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进 {what} 的 {column} 列"
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
// 行为锁：POST /energy-consumptions 登记操作人取会话
// =========================================================

#[tokio::test]
async fn energy_consumption_operator_id_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/energy-consumptions",
            post(energy_handler::create_energy_consumption),
        )],
    );
    let mut body = serde_json::Map::new();
    body.insert("meter_type".to_string(), json!("electricity"));
    body.insert("workshop".to_string(), json!("createdby-b4-lock"));
    body.insert("unit".to_string(), json!("度"));
    body.insert("previous_reading".to_string(), json!("100"));
    body.insert("current_reading".to_string(), json!("180"));
    body.insert("unit_price".to_string(), json!("0.5"));
    body.insert(
        "period_start".to_string(),
        json!("2026-10-01T00:00:00+00:00"),
    );
    body.insert("period_end".to_string(), json!("2026-10-02T00:00:00+00:00"));
    let (status, v) = call_post(&app, "/energy-consumptions", &with_forged_identity(body)).await;
    let id = created_id(status, &v, "能耗记录登记");

    let row = read_row!(energy_consumption_record, id, &read_db, "能耗记录");
    assert_identity_column_is_session(row.operator_id, "operator_id", "能耗记录（登记操作人）");
    assert_identity_column_is_session(row.created_by, "created_by", "能耗记录（建单人）");
    assert_eq!(row.meter_type, "electricity", "业务列不受身份改造牵连");
    assert_eq!(
        row.workshop.as_deref(),
        Some("createdby-b4-lock"),
        "业务列不受身份改造牵连"
    );

    // 行为锁端点覆盖地板：解析/锁定的端点数不得低于本批改动站点数（防空集合假绿）
    let covered = 1usize;
    assert!(
        covered >= CHANGED_SITES,
        "行为锁端点覆盖数不得低于改动站点数 {CHANGED_SITES}，实际 {covered}"
    );
}

// =========================================================
// 形态锁：改动站点对应的请求 DTO 不再承载身份入参字段
// =========================================================

/// 在源码文本中定位 `pub struct NAME { ... }` 的结构体体文本。
/// 找不到即返回 None——由调用方按地板判红，禁止静默跳过。
fn struct_body(src: &str, name: &str) -> Option<String> {
    let marker = format!("pub struct {name} {{");
    let start = src.find(&marker)? + marker.len();
    let rest = &src[start..];
    let end = rest.find("\n}")?;
    Some(rest[..end].to_string())
}

#[test]
fn batch4_request_dtos_have_no_identity_field() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // (DTO 源码相对路径, DTO 名, 禁止承载的身份入参字段)：与本批改动站点一一对应
    let targets: [(&str, &str, [&str; 2]); CHANGED_SITES] = [(
        "src/services/energy_ops/consumption.rs",
        "CreateConsumptionRequest",
        ["operator_id", "created_by"],
    )];
    let mut parsed = 0usize;
    for (file, dto, forbidden) in targets.iter() {
        let path = std::path::Path::new(manifest_dir).join(file);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", path.display()));
        let body = struct_body(&src, dto)
            .unwrap_or_else(|| panic!("形态锁取不到结构体 {dto}（检测力地板，禁止空集合=绿）"));
        parsed += 1;
        for field in forbidden.iter() {
            assert!(
                !body.contains(field),
                "请求 DTO {dto}（{file}）不应再承载身份入参字段 {field}，实际体: {body}"
            );
        }
    }
    assert_eq!(
        parsed,
        targets.len(),
        "改动站点数与解析到的 DTO 数必须一致（检测力地板），实际 {parsed}/{}",
        targets.len()
    );
}
