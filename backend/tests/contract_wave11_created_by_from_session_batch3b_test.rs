//! 后端安全「缸号状态机操作人只认服务端会话」行为级活体证明（批 3 B 组）
//!
//! 锁定的契约：以下两条留痕型端点的操作人身份列 `operator_id` 唯一来源是
//! `AuthContext.user_id`（服务端会话），请求 DTO 不再承载该身份字段；
//! 请求体里伪造的 `operator_id` 会被 serde 忽略，绝不落库。
//! - `POST /production/dye-batch-lifecycle-logs`（缸号生命周期流转日志，
//!   `services/dye_batch_state_machine_ops/lifecycle_log.rs`）；
//! - `POST /production/dye-batch-operations`（缸号操作记录 merge/split/…，
//!   `services/dye_batch_state_machine_ops/operation.rs`）。
//!
//! 证明形态（夹具范式同 contract_wave11_created_by_from_session_batch1）：
//! ① 会话注入用户 A（`from_fn_with_state(make_auth(A), inject_auth)`）；
//! ② 请求体故意携带伪造用户 B 的 `operator_id`；③ 动作必须成功（该键非必填）；
//! ④ 直读实体行断言 `row.operator_id == Some(SESSION_A)` 且 `!= Some(FORGED_B)`；
//! ⑤ 另断业务列不受身份改造牵连。
//!
//! 另含两条形态锁 + 一条端点地板锁：
//! - 两个请求 DTO 结构体不再含 `operator_id` 入参字段；
//! - 两个 service 写入点必须是 `operator_id: Set(Some(operator_id))`（会话形参），
//!   且不再出现 `Set(req.operator_id)`；两个 handler 必须把 `auth.user_id` 传给 service；
//! - 真实路由表 `src/routes/production.rs` 里这两个端点必须解析到（检测力地板：
//!   解析到的端点数 < 改动站点数即当场判红，禁止空集合当通过）。

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
use bingxi_backend::handlers::dye_batch_state_machine_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{dye_batch_lifecycle_log, dye_batch_operation};
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话用户 A：AuthContext.user_id 注入的合法操作人（私有 id 段 951x，
/// 与同族锁 contract_wave11_created_by_from_session_batch1 保持同一对常量）
const SESSION_A: i32 = 9511;
/// 伪造用户 B：只出现在请求体里，永远不允许落进任何 operator_id 留痕列
const FORGED_B: i32 = 9472;
/// 本组实际改动站点数（两条行为锁 + 端点地板锁的判据基线）
const CHANGED_SITES: usize = 2;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w11_b3b_createdby_{user_id}"),
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

/// 以 SESSION_A 为会话用户组装带 auth 注入层的 Router（路由按相对路径注册，同真实 routes 形态）
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

/// 请求体统一掺入伪造身份键（handler 不认的键 serde 直接忽略——正是要证明的「发了也无效」）
fn with_forged_identity(mut obj: serde_json::Map<String, Value>) -> String {
    obj.insert("operator_id".to_string(), Value::from(FORGED_B));
    obj.insert("created_by".to_string(), Value::from(FORGED_B));
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

/// 断言直读行的 operator_id 归会话用户 A、绝归伪造用户 B
fn assert_operator_is_session(row_operator_id: Option<i32>, what: &str) {
    assert_eq!(
        row_operator_id,
        Some(SESSION_A),
        "直读实体：{what} 操作人必须等于会话用户 A，实际 {row_operator_id:?}"
    );
    assert_ne!(
        row_operator_id,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进 {what} 的操作人列"
    );
    assert!(
        row_operator_id.is_some(),
        "直读实体：{what} 操作人列不许为空（会话必落库）"
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
// 域 1：缸号生命周期流转日志（POST /dye-batch-lifecycle-logs）
// =========================================================

#[tokio::test]
async fn lifecycle_log_operator_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/dye-batch-lifecycle-logs",
            post(dye_batch_state_machine_handler::record_transition),
        )],
    );

    // pending_schedule → scheduled（transition_code=schedule）是内置规则表里的合法流转，
    // 保证动作一定成功（不因业务门被拒而把「未落库」误当成「身份来源正确」）。
    let batch_no = format!("B3B-LL-{}", std::process::id());
    let mut body = serde_json::Map::new();
    body.insert("batch_id".to_string(), json!(9850001));
    body.insert("batch_no".to_string(), json!(batch_no));
    body.insert("from_status".to_string(), json!("pending_schedule"));
    body.insert("to_status".to_string(), json!("scheduled"));
    body.insert("transition_code".to_string(), json!("schedule"));
    body.insert("transition_name".to_string(), json!("排缸"));
    body.insert("work_shift".to_string(), json!("morning"));
    let body = with_forged_identity(body);

    let (status, v) = call_post(&app, "/dye-batch-lifecycle-logs", &body).await;
    let id = created_id(status, &v, "缸号生命周期流转日志记录");

    let row = read_row!(dye_batch_lifecycle_log, id, &read_db, "缸号生命周期日志");
    assert_operator_is_session(row.operator_id, "缸号生命周期日志");
    assert_eq!(row.batch_no, batch_no, "业务列（缸号）不受身份改造牵连");
    assert_eq!(row.to_status, "scheduled", "业务列（流转后状态）不受牵连");
    assert_eq!(
        row.transition_code, "schedule",
        "业务列（流转代码）不受牵连"
    );
    assert_eq!(
        row.work_shift.as_deref(),
        Some("morning"),
        "业务列（班次）不受牵连"
    );
}

// =========================================================
// 域 2：缸号操作记录（POST /dye-batch-operations）
// =========================================================

#[tokio::test]
async fn dye_batch_operation_operator_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/dye-batch-operations",
            post(dye_batch_state_machine_handler::create_operation),
        )],
    );

    let batch_no = format!("B3B-OP-{}", std::process::id());
    let mut body = serde_json::Map::new();
    body.insert("operation_type".to_string(), json!("merge"));
    body.insert("operation_name".to_string(), json!("合缸"));
    body.insert("target_batch_id".to_string(), json!(9850002));
    body.insert("target_batch_no".to_string(), json!(batch_no));
    body.insert("source_batch_ids".to_string(), json!([9850003, 9850004]));
    let body = with_forged_identity(body);

    let (status, v) = call_post(&app, "/dye-batch-operations", &body).await;
    let id = created_id(status, &v, "缸号操作记录创建");

    let row = read_row!(dye_batch_operation, id, &read_db, "缸号操作记录");
    assert_operator_is_session(row.operator_id, "缸号操作记录");
    assert_eq!(
        row.operation_type, "merge",
        "业务列（操作类型）不受身份改造牵连"
    );
    assert_eq!(row.operation_name, "合缸", "业务列（操作名称）不受牵连");
    assert_eq!(row.target_batch_no, batch_no, "业务列（目标缸号）不受牵连");
    assert_eq!(
        row.target_batch_id, 9850002,
        "业务列（目标缸号 ID）不受牵连"
    );
}

// =========================================================
// 形态锁 A：两个请求 DTO 不再承载 operator_id 入参字段
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

fn read_src(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", path.display()))
}

#[test]
fn batch3b_request_dtos_have_no_operator_id_field() {
    // (DTO 源码相对路径, 结构体名) 与本组改动站点一一对应
    let targets: [(&str, &str); 2] = [
        ("src/models/dto/dye_batch_dto.rs", "CreateTransitionRequest"),
        ("src/models/dto/dye_batch_dto.rs", "CreateOperationRequest"),
    ];
    let mut parsed = 0usize;
    for (file, dto) in targets.iter() {
        let src = read_src(file);
        let body = struct_body(&src, dto)
            .unwrap_or_else(|| panic!("形态锁取不到结构体 {dto}（检测力地板，禁止空集合=绿）"));
        parsed += 1;
        assert!(
            !body.contains("operator_id"),
            "请求 DTO {dto}（{file}）不应再承载 operator_id 入参字段，实际体: {body}"
        );
    }
    assert_eq!(
        parsed,
        targets.len(),
        "两个缸号状态机建单 DTO 必须全部被解析到（检测力地板），实际 {parsed}"
    );
}

// =========================================================
// 形态锁 B：service 写入点与 handler 取值点必须来自会话形参
// =========================================================

#[test]
fn batch3b_write_and_handler_sites_take_session_operator() {
    // (源码相对路径, 必须出现的会话写法, 必须消失的请求体写法)
    // 禁串一律带 service 变量名限定，避免与同 handler 内其它 `create(req)`（状态规则/回修单）误判。
    let sites: [(&str, &str, &str); 4] = [
        (
            "src/services/dye_batch_state_machine_ops/lifecycle_log.rs",
            "operator_id: Set(Some(operator_id))",
            "operator_id: Set(req.operator_id)",
        ),
        (
            "src/services/dye_batch_state_machine_ops/operation.rs",
            "operator_id: Set(Some(operator_id))",
            "operator_id: Set(req.operator_id)",
        ),
        (
            "src/handlers/dye_batch_state_machine_handler.rs",
            ".record_transition(req, auth.user_id)",
            "lifecycle_log_service(&state).record_transition(req)",
        ),
        (
            "src/handlers/dye_batch_state_machine_handler.rs",
            "operation_service(&state).create(req, auth.user_id)",
            "operation_service(&state).create(req)",
        ),
    ];
    let mut parsed = 0usize;
    for (file, required, forbidden) in sites.iter() {
        let src = read_src(file);
        parsed += 1;
        assert!(
            src.contains(required),
            "{file} 必须含会话写法 `{required}`（操作人唯一来源=服务端会话）"
        );
        assert!(
            !src.contains(forbidden),
            "{file} 不许再出现请求体写法 `{forbidden}`（操作人由 body 决定的根因形态）"
        );
    }
    assert_eq!(
        parsed,
        sites.len(),
        "四个写入/取值站点必须全部被解析到（检测力地板），实际 {parsed}"
    );
}

// =========================================================
// 端点地板锁：真实路由表里这两个端点必须存在（防空集合假绿）
// =========================================================

#[test]
fn batch3b_target_endpoints_are_mounted() {
    let src = read_src("src/routes/production.rs");
    // (端点相对路径片段, handler 名)
    let endpoints: [&str; 2] = [
        "dye_batch_state_machine_handler::record_transition",
        "dye_batch_state_machine_handler::create_operation",
    ];
    let mut resolved = 0usize;
    for ep in endpoints.iter() {
        assert!(
            src.contains(ep),
            "真实路由表必须注册端点 `{ep}`，否则本组行为锁验证的是未挂载的假端点"
        );
        resolved += 1;
    }
    assert!(
        resolved >= CHANGED_SITES,
        "解析到的端点数（{resolved}）不得少于本组改动站点数（{CHANGED_SITES}）——检测力地板"
    );
}
