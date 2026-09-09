//! 系统更新权限矩阵 HTTP 层测试（doto 2026-09-09 system-update 权限门禁缺口）
//!
//! 覆盖以下场景（无需真实 PG，AppState::default() mock DB）：
//! 1. 未登录（无 AuthContext）→ AuthContext extractor 拒绝 → 401
//! 2. 非 admin（role_id=999，mock DB fail-closed）→ require_admin_role 拒绝 → 403
//! 3. 只读端点 /system-update/current-version（无权限校验）→ 200 可达
//! 4. 高危端点 download_and_update / rollback_version 非 admin 全部 403
//!
//! 设计说明：
//! - 复用 test_permission_rbac.rs 的 AuthContext 注入中间件模式
//! - is_admin_role 对 mock DB fail-closed 返回 false → 非 admin 必拒
//! - admin 对照（真实 roles 表 code=admin 查询）由 E2E 40-system-update-authz
//!   真实登录链路覆盖（40 spec），本测试聚焦未认证/非 admin 拒绝语义

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::system_update_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use serde_json::Value;
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

/// 构建系统更新测试 Router：真实 handler + AuthContext 注入
fn build_app(state: AppState, auth: Option<AuthContext>) -> Router {
    let base = Router::new()
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
    match auth {
        Some(a) => base.layer(from_fn_with_state(a, inject_auth)),
        None => base,
    }
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
    let bytes = axum::body::to_bytes(body, 4096).await.expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("响应体不是合法 JSON")
}

/// 场景 1：未登录（无 AuthContext extensions）→ 401
#[tokio::test]
async fn test_system_update_unauthenticated_401() {
    let app = build_app(AppState::default(), None);
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
    let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    assert_ne!(code, 200, "未登录不应返回业务码 200");
}

/// 场景 2：非 admin（role_id=999，mock fail-closed）调用 download_and_update → 403
#[tokio::test]
async fn test_system_update_non_admin_403_download() {
    let app = build_app(AppState::default(), Some(make_auth(2, "e2e_cashier", Some(999))));
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
    assert!(
        message.contains("管理员") || message.contains("权限"),
        "拒绝消息应说明 admin 限制，实际: {message}"
    );
}

/// 场景 3：非 admin 调用 rollback_version → 403
/// （axum extractor 顺序执行：auth 在 Json 前，auth 403 先于 body 解析；带合法 JSON 确保 422 不先触发）
#[tokio::test]
async fn test_system_update_non_admin_403_rollback() {
    let app = build_app(AppState::default(), Some(make_auth(2, "e2e_cashier", Some(999))));
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
    let app = build_app(AppState::default(), Some(make_auth(3, "e2e_norole", None)));
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
#[tokio::test]
async fn test_system_update_version_readonly_200() {
    let app = build_app(AppState::default(), Some(make_auth(1, "e2e_admin", Some(1))));
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
    let version = json
        .get("data")
        .and_then(|d| d.get("version"))
        .and_then(|v| v.as_str());
    assert!(version.is_some(), "应返回 version 字段");
}
