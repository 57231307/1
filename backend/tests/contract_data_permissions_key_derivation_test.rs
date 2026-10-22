//! 数据权限族端点权限键派生锁（静态复算 + 真实中间件链，不连库、不起服务）
//!
//! 功能：钉死数据权限七个端点的资源名一律稳定派生为 data-permissions，且该段登记在路径
//! 白名单的直接资源表、而非模块前缀表。登记错层是本族缺陷的假修高发点：只把段加进模块
//! 前缀表虽能过白名单，却把资源名漂到第四段，派生出注册表外的伪键（资源类型与范围类型
//! 两个下拉面漂成 resource-types、scope-types，按角色查询面漂成 roles 域键），
//! 非超管恒 403、已登记授权行同时成死码。
//! 覆盖判据：
//! 1. 各面路径逐条经生产谓词 extract_resource_info 复算：资源名 data-permissions，
//!    行 ID 仅在记录面与按角色面取回，下拉面为 None。
//! 2. 动作按方法派生：GET 得 read、POST 得 create、DELETE 得 delete。
//! 3. 层级锁：is_known_resource_segment 为真且 is_module_prefix 为假，挪回模块前缀表即判红。
//! 4. 注册表锁：PERMISSION_RESOURCES 含 data-permissions；不得含漂段伪名
//!    resource-types、scope-types（收编伪名等于把错派生洗成正式键）。
//! 5. 基线棘轮下调锁：存量基线不得再收录 data-permissions、resource-types、scope-types，
//!    修复后仍留行等于把已修债务藏回基线。
//! 6. 拒绝归因（HTTP 链，断开库）：零授权角色写本族端点的 403 来自权限判定层
//!    （文案为权限不足），不再来自白名单层；未登记对照段仍被白名单层拒绝，
//!    证明未登记即拒绝的 fail-closed 语义未被放松。
//! 调用方：CI 的 Rust 集成测试任务（本地按纪律不跑 cargo test，由 CI 验证行为）。
//! 入参：仅路径字面量与生产谓词，不复制任何名单副本。
//! 传给谁：全部判定走生产函数 extract_resource_info、method_to_action、
//!    permission_middleware 与生产注册表 PERMISSION_RESOURCES。
//! 存什么：不写任何数据，纯读。
//! 存哪里：不适用（无落库）。

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{
    extract_resource_info, method_to_action, permission_middleware,
};
use bingxi_backend::services::init_service::PERMISSION_RESOURCES;
use bingxi_backend::utils::path_utils::{is_known_resource_segment, is_module_prefix};
use tower::ServiceExt;

const BASELINE: &str = include_str!("resource_key_baseline.txt");

/// 负向对照段：不登记在任何表内，用于证明白名单闸门仍是真门。
const CANARY_SEGMENT: &str = "data-permissions-canary-xyz";
/// 对照段的完整路径（路由注册与请求共用同一字面量，防止两处漂移）。
const CANARY_PATH: &str = "/api/v1/erp/data-permissions-canary-xyz";

async fn ok_handler() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"ok": true})))
}

/// AuthContext 注入中间件（与打印任务契约锁同模式）：外层先写入 extensions，
/// 权限中间件后读取。
async fn inject_auth(
    axum::extract::State(auth): axum::extract::State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

fn make_auth(user_id: i32, username: &str, role_id: Option<i32>) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
        role_id,
        department_id: None,
        data_scope: None,
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/erp/data-permissions", post(ok_handler))
        .route(CANARY_PATH, post(ok_handler))
        .layer(from_fn_with_state(state.clone(), permission_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn send(app: Router, method: &str, uri: &str) -> (StatusCode, String) {
    let resp = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4096)
        .await
        .expect("拒绝响应体应可读为字节");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// 判据①，挂载的七个端点逐条钉死：方法、路径、资源名、行 ID 与动作派生全部同源，
/// 且每个派生资源名都必须在 PERMISSION_RESOURCES 在册（在册才可被授予，注册表外即死键）。
#[test]
fn all_faces_derive_stable_resource_name() {
    let cases: [(&str, &str, Option<i32>, &str); 7] = [
        ("GET", "/api/v1/erp/data-permissions", None, "read"),
        ("POST", "/api/v1/erp/data-permissions", None, "create"),
        ("GET", "/api/v1/erp/data-permissions/123", Some(123), "read"),
        (
            "DELETE",
            "/api/v1/erp/data-permissions/42",
            Some(42),
            "delete",
        ),
        (
            "GET",
            "/api/v1/erp/data-permissions/roles/7",
            Some(7),
            "read",
        ),
        (
            "GET",
            "/api/v1/erp/data-permissions/resource-types",
            None,
            "read",
        ),
        (
            "GET",
            "/api/v1/erp/data-permissions/scope-types",
            None,
            "read",
        ),
    ];
    for (method, path, want_id, want_action) in cases {
        let (resource, id) = extract_resource_info(path);
        assert_eq!(
            resource, "data-permissions",
            "{method} {path} 的资源名必须稳定派生为 data-permissions，实得 {resource}"
        );
        assert_eq!(
            id, want_id,
            "{method} {path} 的行 ID 派生不符，期望 {want_id:?} 实得 {id:?}"
        );
        let action = method_to_action(&Method::from_bytes(method.as_bytes()).unwrap());
        assert_eq!(
            action, want_action,
            "{method} {path} 的动作派生不符，期望 {want_action} 实得 {action}"
        );
        assert!(
            PERMISSION_RESOURCES.contains(&resource.as_str()),
            "{path} 的派生资源名 {resource} 必须在 PERMISSION_RESOURCES 在册，否则任何角色都授予不了"
        );
    }
}

/// 判据②，动作由方法派生且与注册表可授予动作同源。
#[test]
fn actions_derive_from_method() {
    assert_eq!(
        method_to_action(&Method::GET),
        "read",
        "GET 面动作应为 read"
    );
    assert_eq!(
        method_to_action(&Method::POST),
        "create",
        "POST 面动作应为 create"
    );
    assert_eq!(
        method_to_action(&Method::DELETE),
        "delete",
        "DELETE 面动作应为 delete"
    );
}

/// 判据③，层级锁，本段必须登记在直接资源表，挪回模块前缀表即判红（登记错层假修的直接防线）。
#[test]
fn registered_in_direct_resource_layer_not_module_prefix() {
    assert!(
        is_known_resource_segment("data-permissions"),
        "data-permissions 必须在路径白名单内，否则 validate_route_whitelist \
         对包括管理员在内的所有角色恒 403（未知的资源路径）"
    );
    assert!(
        !is_module_prefix("data-permissions"),
        "data-permissions 不得登记为模块前缀，其第四段是记录 ID 或同一资源下的查询与下拉维度，\
         不是子资源，登记为模块前缀会让资源名漂到第四段（本锁的设立原因）"
    );
    assert!(
        !is_known_resource_segment(CANARY_SEGMENT),
        "对照段 {CANARY_SEGMENT} 不得在任何白名单表内（fail-closed 证据）"
    );
}

/// 判据④，注册表锁，派生权威名必须可授予，漂段伪名不得被收编。
#[test]
fn authoritative_name_registered_and_drift_names_excluded() {
    assert!(
        PERMISSION_RESOURCES.contains(&"data-permissions"),
        "data-permissions 必须登记在 PERMISSION_RESOURCES，否则运行时键恒不命中任何授权行"
    );
    assert!(
        !PERMISSION_RESOURCES.contains(&"resource-types"),
        "resource-types 只在登记错层时才会被派生出来，属伪键，不得进注册表"
    );
    assert!(
        !PERMISSION_RESOURCES.contains(&"scope-types"),
        "scope-types 同上，不得进注册表"
    );
    assert!(
        !PERMISSION_RESOURCES.contains(&CANARY_SEGMENT),
        "对照段不得登记在 PERMISSION_RESOURCES"
    );
}

/// 判据⑤，基线棘轮已随修复下调，三条名义键不得再滞留在存量基线里。
#[test]
fn baseline_ratchet_reduced_for_fixed_names() {
    for removed in ["data-permissions", "resource-types", "scope-types"] {
        assert!(
            !BASELINE.lines().map(str::trim).any(|line| line == removed),
            "存量基线仍收录 {removed}，修复后派生不再产出该名义键，留行等于把已修债务藏回基线"
        );
    }
}

/// 判据⑥，拒绝归因：零授权角色的 403 必须来自权限判定层（权限不足），
/// 不再是白名单层（未知的资源路径）。断开库下 check_permission 对任何角色 fail-closed，
/// 该断言锁的是拒绝发生在哪一层，不是放行面。
#[tokio::test]
async fn denial_attributed_to_permission_layer_not_whitelist() {
    let app = build_app(
        AppState::default(),
        make_auth(2, "e2e_zero_perm", Some(999)),
    );
    let (status, body) = send(app, "POST", "/api/v1/erp/data-permissions").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "零授权角色写数据权限应 403，实得 {status}"
    );
    assert!(
        body.contains("权限不足"),
        "403 归因必须是权限判定层，实得响应体：{body}"
    );
    assert!(
        !body.contains("未知的资源路径"),
        "白名单层不得再拒绝已登记的 data-permissions 面，实得响应体：{body}"
    );
}

/// 判据⑥负向对照：未登记段仍被白名单层拒绝，证明本次登记没有把闸门改成放行未知段。
#[tokio::test]
async fn unregistered_canary_still_rejected_by_whitelist() {
    let app = build_app(
        AppState::default(),
        make_auth(2, "e2e_zero_perm", Some(999)),
    );
    let (status, body) = send(app, "POST", CANARY_PATH).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "未登记对照段必须仍被白名单拒绝（未登记即拒绝的语义不放松）"
    );
    assert!(
        body.contains("未知的资源路径"),
        "对照段的 403 应来自白名单层，实得响应体：{body}"
    );
}
