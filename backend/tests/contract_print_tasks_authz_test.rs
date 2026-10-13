//! 打印任务 RBAC 契约锁：派生资源名、注册表登记、路由白名单可达性与最小授权三处同源。
//!
//! 覆盖：
//! 1. `POST/GET /api/v1/erp/print-tasks` 由 URL seg3 派生为资源名 `print-tasks`（无模块前缀消歧），
//!    且该资源段已在权限中间件白名单 `is_known_resource_segment` 内（否则对全员 fail-closed 403）；
//! 2. 派生权威名 `print-tasks` 已登记在 `PERMISSION_RESOURCES`（负向锁：注册表外的段必然 403）；
//! 3. HTTP 层（真实 `permission_middleware` + tower oneshot）：无 print-tasks 权限码的非超管角色
//!    访问 GET/POST `/print-tasks` 均 403；未登记的资源段（负向对照）同样 403「未知的资源路径」；
//! 4. 授权矩阵最小化：`print-tasks` 只授 `system_admin` 且仅 `read` + `create`（无通配、无 update/
//!    delete、无业务岗），`admin` 经既有通配覆盖不在矩阵里单列。
//!
//! 说明：真实“system_admin 非 403”需连库命中 role_permissions 行，由 e2e 40-system-update-authz 同族
//! 链路覆盖；本测在非连库下以「派生键 == 矩阵授予键」+「白名单登记」证明该岗位在授予后即放行，
//! 并锁定 fail-closed 拒绝面（无权限角色与未登记段恒 403）。

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{
    extract_resource_info, method_to_action, permission_middleware,
};
use bingxi_backend::middleware::public_routes::is_public_path;
use bingxi_backend::services::init_service::PERMISSION_RESOURCES;
use bingxi_backend::utils::path_utils::is_known_resource_segment;
use tower::ServiceExt;

/// 测试用业务处理器：命中则返回 200，用于确认“403 来自中间件而非处理器”。
async fn ok_handler() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"ok": true})))
}

/// AuthContext 注入中间件（与 test_permission_rbac 相同模式）。
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

/// 构建带真实 `permission_middleware` 的测试 Router：注入外层 auth、内层 RBAC。
/// 层序与 test_permission_rbac 一致（inject_auth 先执行写 extensions，permission_middleware 后校验）。
fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route(
            "/api/v1/erp/print-tasks",
            get(ok_handler).post(ok_handler),
        )
        // 负向对照段：不在任何注册表/白名单内，应被 validate_route_whitelist 直接 403。
        .route("/api/v1/erp/print-tasks-unregistered-xyz", get(ok_handler))
        .layer(from_fn_with_state(state.clone(), permission_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn send(app: Router, method: &str, uri: &str) -> Response {
    app.oneshot(
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

/// 判据①：`/api/v1/erp/print-tasks` 派生资源名恰为 `print-tasks`，且动作 GET→read / POST→create。
#[test]
fn print_tasks_url_derives_to_print_tasks_resource() {
    let (resource, id) = extract_resource_info("/api/v1/erp/print-tasks");
    assert_eq!(
        resource, "print-tasks",
        "根级打印任务的资源名应取 URL seg3=print-tasks（无模块前缀消歧），实得 {resource}"
    );
    assert!(id.is_none(), "列表路径不应解析出资源 ID");

    let (res_detail, id_detail) = extract_resource_info("/api/v1/erp/print-tasks/123");
    assert_eq!(
        res_detail, "print-tasks",
        "详情路径的资源名仍应为 print-tasks，实得 {res_detail}"
    );
    assert_eq!(id_detail, Some(123), "详情路径应解析出资源 ID=123");

    assert_eq!(
        method_to_action(&axum::http::Method::GET),
        "read",
        "GET 动作应派生 read"
    );
    assert_eq!(
        method_to_action(&axum::http::Method::POST),
        "create",
        "POST 动作应派生 create"
    );
}

/// 判据②：`print-tasks` 既在中间件白名单（否则对全员 fail-closed 403），又登记在 PERMISSION_RESOURCES。
#[test]
fn print_tasks_is_whitelisted_segment_and_registered_resource() {
    assert!(
        is_known_resource_segment("print-tasks"),
        "print-tasks 必须在 is_known_resource_segment 白名单内——否则 validate_route_whitelist \
         对包括 admin 在内的所有人恒 403（fail-closed 静默失效）"
    );
    assert!(
        PERMISSION_RESOURCES.contains(&"print-tasks"),
        "派生权威名 print-tasks 必须登记在 PERMISSION_RESOURCES，否则运行时键恒不命中任何授权行"
    );
    // 负向：未登记的对照段必须不在白名单，证明白名单是真门而非全放行。
    assert!(
        !is_known_resource_segment("print-tasks-unregistered-xyz"),
        "对照段 print-tasks-unregistered-xyz 不应在白名单内（作为 fail-closed 证据）"
    );
    assert!(
        !PERMISSION_RESOURCES.contains(&"print-tasks-unregistered-xyz"),
        "对照段不得登记在 PERMISSION_RESOURCES"
    );
}

/// 判据④：授权矩阵最小化——print-tasks 仅授 system_admin 的 read+create，无通配/写动作。
#[test]
fn matrix_grants_print_tasks_read_create_to_system_admin_only() {
    let src = matrix_src();
    // 执行体（剔整行注释）里 print-tasks 授予恰为 2 条：read + create。
    let code = code_only(&src);
    let read = "(\"print-tasks\", \"read\")";
    let create = "(\"print-tasks\", \"create\")";
    assert_eq!(
        code.matches(read).count(),
        1,
        "print-tasks:read 在矩阵里应恰 1 处（仅 system_admin），实得 {}",
        code.matches(read).count()
    );
    assert_eq!(
        code.matches(create).count(),
        1,
        "print-tasks:create 在矩阵里应恰 1 处（仅 system_admin），实得 {}",
        code.matches(create).count()
    );
    // 最小授权禁项：不得出现通配或写/删授予。
    for banned in [
        "(\"print-tasks\", \"*\")",
        "(\"print-tasks\", \"update\")",
        "(\"print-tasks\", \"delete\")",
    ] {
        assert!(
            !code.contains(banned),
            "打印任务对系统管理岗只需 read+create，矩阵禁止出现 {banned}"
        );
    }
    // 两条授予必须都落在 system_admin 分组内（越组授予=扩权）。
    let block = system_admin_block(&code);
    assert!(
        block.contains(read) && block.contains(create),
        "print-tasks 的 read+create 授予必须位于 system_admin 分组内"
    );
}

/// 判据③（HTTP）：无 print-tasks 权限码的非超管角色 GET/POST `/print-tasks` 均 403。
/// mock（Disconnected）DB 下 `check_permission` 对任何角色 fail-closed，证明拒绝面无泄漏。
#[tokio::test]
async fn non_privileged_role_is_403_on_print_tasks() {
    let app = build_app(AppState::default(), make_auth(2, "e2e_cashier", Some(999)));
    let get_resp = send(app, "GET", "/api/v1/erp/print-tasks").await;
    assert_eq!(
        get_resp.status(),
        StatusCode::FORBIDDEN,
        "无 print-tasks 权限码的角色 GET 应 403，实得 {}",
        get_resp.status()
    );

    let app = build_app(AppState::default(), make_auth(2, "e2e_cashier", Some(999)));
    let post_resp = send(app, "POST", "/api/v1/erp/print-tasks").await;
    assert_eq!(
        post_resp.status(),
        StatusCode::FORBIDDEN,
        "无 print-tasks 权限码的角色 POST 应 403，实得 {}",
        post_resp.status()
    );
}

/// 判据③（HTTP 负向对照）：未登记的资源段被白名单层直接 403「未知的资源路径」。
#[tokio::test]
async fn unregistered_segment_is_403_via_whitelist() {
    let app = build_app(AppState::default(), make_auth(2, "e2e_cashier", Some(999)));
    let resp = send(app, "GET", "/api/v1/erp/print-tasks-unregistered-xyz").await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "未登记段应被 validate_route_whitelist 403，实得 {}",
        resp.status()
    );
}

/// 判据③（HTTP）：缺少 role_id 的账号在 RBAC 前先被拒（403，不进入 print-tasks 授权判断）。
#[tokio::test]
async fn missing_role_is_403() {
    let app = build_app(AppState::default(), make_auth(3, "e2e_norole", None));
    let resp = send(app, "GET", "/api/v1/erp/print-tasks").await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "未分配角色应 403，实得 {}",
        resp.status()
    );
}

/// 反向确认公共路径判定不误命中打印任务端点（打印任务不是公共路径，须走 RBAC）。
#[test]
fn print_tasks_is_not_public_path() {
    assert!(
        !is_public_path("/api/v1/erp/print-tasks"),
        "print-tasks 不得被判定为公共路径（否则绕过 RBAC）"
    );
}

fn matrix_src() -> String {
    include_str!("../src/services/init_service_ops/permission.rs").replace('\r', "")
}

/// 只保留“代码 + 字符串字面量”：整行注释逐行剔除（授予判据只针对执行体，注释不计入）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// system_admin 分组体：从 `"system_admin",` 起，到其后第一个 `],`（本分组授予数组闭合）为止。
fn system_admin_block(code: &str) -> String {
    let start = code
        .find("\"system_admin\",")
        .expect("授权矩阵必须有 system_admin 分组（print-tasks 授予的归属）");
    let rest = &code[start..];
    let end = rest
        .find("],")
        .expect("system_admin 分组的授予数组必须以 ] 闭合");
    rest[..end].to_string()
}
