//! 系统更新权限矩阵 HTTP 层测试（doto 2026-09-09 system-update 权限门禁缺口）
//!
//! 覆盖以下场景：拒绝类用 AppState::default()（mock DB 对角色查询 fail-closed 返回 None），
//! 只读版本类用真库 setup_test_db（current-version 现读 system_version 取真实发布日期与说明）：
//! 1. 未登录（无 AuthContext）→ AuthContext extractor 拒绝 → 401
//! 2. 非 admin（role_id=999，mock DB fail-closed）→ require_admin_role 拒绝 → 403
//! 3. 只读端点 /system-update/current-version（无权限校验，真库）→ 200，无当前版本行时返回 null 未知
//! 4. 高危端点 download_and_update / rollback_version 非 admin 全部 403
//!
//! 设计说明：
//! - 复用 test_permission_rbac.rs 的 AuthContext 注入中间件模式
//! - require_admin_role 按角色 code 属 admin/ops 集合判定，get_role_code 对 mock DB
//!   fail-closed 返回 None ⇒ 任何 code 都解析不到 ⇒ 非授权集合一律拒绝（403）
//! - admin/ops 对照（真实 roles 表 code 命中允许集合）由 E2E 40-system-update-authz
//!   真实登录链路覆盖（40 spec），本测试聚焦未认证/非授权角色拒绝语义

mod test_common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::system_update_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use serde_json::Value;
use test_common::setup_test_db;
use tower::ServiceExt;

/// AuthContext 注入中间件（与 test_permission_rbac 相同模式）
async fn inject_auth(
    axum::extract::State(auth): axum::extract::State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

/// 构建系统更新测试 Router（带 AuthContext 注入）：真实 handler + 注入中间件
fn build_app(state: AppState, auth: AuthContext) -> Router {
    let routes = Router::new()
        .route(
            "/api/v1/erp/system-update/current-version",
            get(system_update_handler::get_version),
        )
        .route(
            "/api/v1/erp/system-update/download-and-update",
            post(system_update_handler::download_and_update),
        )
        .route(
            "/api/v1/erp/system-update/rollback",
            post(system_update_handler::rollback_version),
        );
    // with_state 消费 AppState（handlers 有 State<AppState> extractor），再挂注入层
    routes
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

/// 构建系统更新测试 Router（无 AuthContext）：用于未登录 401 场景
fn build_app_no_auth(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v1/erp/system-update/current-version",
            get(system_update_handler::get_version),
        )
        .route(
            "/api/v1/erp/system-update/download-and-update",
            post(system_update_handler::download_and_update),
        )
        .route(
            "/api/v1/erp/system-update/rollback",
            post(system_update_handler::rollback_version),
        )
        .with_state(state)
}

/// 构造测试用 AuthContext（与 test_permission_rbac 相同字段）
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

async fn read_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 4096)
        .await
        .expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("响应体不是合法 JSON")
}

/// 场景 1：未登录（无 AuthContext extensions）→ 401
#[tokio::test]
async fn test_system_update_unauthenticated_401() {
    let app = build_app_no_auth(AppState::default());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/erp/system-update/download-and-update")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "未登录调用高危更新端点应 401，实际 {}",
        resp.status()
    );
    let json = read_json(resp.into_body()).await;
    // 统一失败信封（`utils/error.rs` 的 `ErrorResponse`）的 code 是字符串机器码，
    // 不再是数字状态码；数字 200 只出现在成功信封里。
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("UNAUTHORIZED"),
        "未登录应返回字符串机器码 UNAUTHORIZED，实际: {:?}",
        json.get("code")
    );
    assert!(
        json.get("trace_id").and_then(|v| v.as_str()).is_some(),
        "失败信封应带 trace_id，实际: {json}"
    );
}

/// 场景 2：非 admin（role_id=999，mock fail-closed）调用 download_and_update → 403
#[tokio::test]
async fn test_system_update_non_admin_403_download() {
    let app = build_app(AppState::default(), make_auth(2, "e2e_cashier", Some(999)));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/erp/system-update/download-and-update")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "非 admin 调用 download_and_update 应 403，实际 {}",
        resp.status()
    );
    let json = read_json(resp.into_body()).await;
    let message = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
    // 拒绝消息按真实责任分支给出：mock DB 无法解析角色 code ⇒ 角色信息不可用（fail-closed），
    // 真实库中 code 非 admin/ops ⇒ 仅限管理员或系统运维。二者都以"角色"为核心说明授权被拒，
    // 不再锁旧版单一"管理员"措辞（require_admin_role 现按角色集合判定，拒绝原因随责任分支而不同）。
    assert!(
        message.contains("角色") || message.contains("权限"),
        "拒绝消息应说明授权被拒（角色维度），实际: {message}"
    );
}

/// 场景 3：非 admin 调用 rollback_version → 403
/// （axum extractor 顺序执行：auth 在 Json 前，auth 403 先于 body 解析；带合法 JSON 确保 422 不先触发）
#[tokio::test]
async fn test_system_update_non_admin_403_rollback() {
    let app = build_app(AppState::default(), make_auth(2, "e2e_cashier", Some(999)));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/erp/system-update/rollback")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"version":"0.0.1"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "非 admin 调用 rollback 应 403，实际 {}",
        resp.status()
    );
}

/// 场景 4：非 admin 缺 role_id（None）→ 403（require_admin_role 先行拒绝）
#[tokio::test]
async fn test_system_update_missing_role_403() {
    let app = build_app(AppState::default(), make_auth(3, "e2e_norole", None));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/erp/system-update/download-and-update")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "未分配角色应 403，实际 {}",
        resp.status()
    );
}

/// 场景 5：只读端点 current-version 无权限校验，登录态可达 → 200
/// 该端点现按 is_current 读 system_version 表返回真实发布日期与更新说明，故夹具须为真库：
/// setup_test_db 已迁移并清空业务表，system_version 无当前版本行，接口如实返回 release_date
/// 与 changelog 为 null（未知），这是真查库无行的结果，不是请求时刻、也不是 mock 库默认值。
#[tokio::test]
async fn test_system_update_version_readonly_200() {
    let db = setup_test_db().await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..AppState::default()
    };
    let app = build_app(state, make_auth(1, "e2e_admin", Some(1)));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/erp/system-update/current-version")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "只读版本端点登录态应 200，实际 {}",
        resp.status()
    );
    let json = read_json(resp.into_body()).await;
    let data = json.get("data").expect("响应应含 data 字段");
    assert!(
        data.get("version").and_then(|v| v.as_str()).is_some(),
        "应返回 version 字段"
    );
    // 无当前版本行时发布日期与更新说明必须为 null，钉死不再回落请求当前时刻或空串。
    assert!(
        data.get("release_date").is_some() && data["release_date"].is_null(),
        "查无当前版本行时 release_date 应为 null（未知），实际: {:?}",
        data.get("release_date")
    );
    assert!(
        data.get("changelog").is_some() && data["changelog"].is_null(),
        "查无当前版本行时 changelog 应为 null（未知），实际: {:?}",
        data.get("changelog")
    );
}
