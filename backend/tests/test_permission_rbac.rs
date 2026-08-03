//! 权限 RBAC 中间件集成测试（V15 P1 缺陷 14.11-A）
//!
//! 覆盖以下场景：
//! 1. 非 admin 角色访问受限资源 → 403（无权限拒绝）
//! 2. 公共路径放行（无需权限校验）
//! 3. 缺少 AuthContext → 401（未认证）
//! 4. 缺少 role_id → 403（未关联角色）
//! 5. 缓存失效函数不 panic（14.11-C 公共 API 边界）
//!
//! 设计说明：
//! - 使用 `AppState::default()`（mock DB），`is_admin_role` fail-closed 返回 false，
//!   `check_permission` 查询 mock DB 返回空权限列表，非 admin 请求必被 403 拒绝。
//! - AuthContext 通过自定义注入中间件写入 request extensions，模拟 auth_middleware 行为。

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use bingxi_backend::container::AppState;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{
    invalidate_all_permission_cache, invalidate_permission_cache, permission_middleware,
};
use serde_json::Value;
use tower::ServiceExt;

/// 测试用业务处理器：直接返回 200 + 简易 JSON
async fn ok_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({"ok": true, "message": "业务处理器返回成功"})),
    )
}

/// AuthContext 注入中间件：将预设的 AuthContext 写入 request extensions
async fn inject_auth(
    axum::extract::State(auth): axum::extract::State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

/// 构建带 AuthContext 注入的测试 Router（permission_middleware + inject_auth）
fn build_test_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/erp/users", get(ok_handler))
        .route("/api/v1/erp/products", get(ok_handler))
        // 层序：inject_auth（外层，先执行）→ permission_middleware（内层，后执行）
        .layer(from_fn_with_state(state.clone(), permission_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
}

/// 构建无 AuthContext 注入的测试 Router（仅 permission_middleware，用于测试 401 场景）
fn build_test_app_no_auth(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/erp/users", get(ok_handler))
        .layer(from_fn_with_state(state, permission_middleware))
}

/// 构建带公共路径的测试 Router
fn build_test_app_with_public_path(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/erp/users", get(ok_handler))
        .route("/api/v1/erp/auth/login", get(ok_handler))
        .layer(from_fn_with_state(state.clone(), permission_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
}

/// 构造测试用 AuthContext
fn make_auth(user_id: i32, username: &str, role_id: Option<i32>) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
        role_id,
        department_id: None,
        data_scope: None,
    }
}

/// 读取响应体为 JSON 值
async fn read_json(body: axum::body::Body) -> Value {
    let bytes = axum::body::to_bytes(body, 4096)
        .await
        .expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("响应体不是合法 JSON")
}

/// 场景 1（14.11-A）：非 admin 角色（role_id=999）访问受限资源 → 403
#[tokio::test]
async fn test_non_admin_denied_without_permission() {
    let state = AppState::default();
    let auth = make_auth(1001, "operator_user", Some(999));
    let app = build_test_app(state, auth);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/erp/users")
        .body(Body::empty())
        .expect("构造 GET 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "非 admin 角色无权限应返回 403"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        body.get("code").and_then(|v| v.as_i64()),
        Some(403),
        "业务码应为 403，实际: {:?}",
        body.get("code")
    );
}

/// 场景 2（14.11-A）：不同非 admin 角色（role_id=888）访问另一受限资源 → 403
#[tokio::test]
async fn test_non_admin_denied_on_different_resource() {
    let state = AppState::default();
    let auth = make_auth(2002, "manager_user", Some(888));
    let app = build_test_app(state, auth);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/erp/products")
        .body(Body::empty())
        .expect("构造 GET 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "非 admin 角色访问任意受限资源都应返回 403"
    );
}

/// 场景 3（14.11-A 边界）：公共路径放行（无需权限校验）→ 200
#[tokio::test]
async fn test_public_path_bypasses_permission_check() {
    let state = AppState::default();
    let auth = make_auth(3003, "any_user", Some(777));
    let app = build_test_app_with_public_path(state, auth);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/erp/auth/login")
        .body(Body::empty())
        .expect("构造 GET 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "公共路径应放行，不受权限校验限制"
    );
}

/// 场景 4（14.11-A 边界）：缺少 AuthContext → 401（未认证）
#[tokio::test]
async fn test_missing_auth_context_returns_401() {
    let state = AppState::default();
    let app = build_test_app_no_auth(state);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/erp/users")
        .body(Body::empty())
        .expect("构造 GET 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "缺少 AuthContext 应返回 401"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        body.get("code").and_then(|v| v.as_i64()),
        Some(401),
        "业务码应为 401，实际: {:?}",
        body.get("code")
    );
}

/// 场景 5（14.11-A 边界）：AuthContext 无 role_id → 403（未关联角色）
#[tokio::test]
async fn test_missing_role_id_returns_403() {
    let state = AppState::default();
    let auth = make_auth(4004, "no_role_user", None);
    let app = build_test_app(state, auth);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/erp/users")
        .body(Body::empty())
        .expect("构造 GET 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "无 role_id 的用户应返回 403"
    );
}

/// 场景 6（14.11-A）：POST 请求非 admin 角色 → 403（create 动作也被拒绝）
#[tokio::test]
async fn test_non_admin_denied_for_create_action() {
    let state = AppState::default();
    let auth = make_auth(5005, "create_attempt_user", Some(555));
    let app = build_test_app(state, auth);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/users")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("构造 POST 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "非 admin 角色的 POST 请求应返回 403"
    );
}

/// 场景 7（14.11-A）：DELETE 请求非 admin 角色 → 403（delete 动作被拒绝）
#[tokio::test]
async fn test_non_admin_denied_for_delete_action() {
    let state = AppState::default();
    let auth = make_auth(6006, "delete_attempt_user", Some(444));
    let app = build_test_app(state, auth);

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/v1/erp/users/123")
        .body(Body::empty())
        .expect("构造 DELETE 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "非 admin 角色的 DELETE 请求应返回 403"
    );
}

/// 场景 8（14.11-C 公共 API 边界）：invalidate_permission_cache 不 panic
#[tokio::test]
async fn test_invalidate_permission_cache_does_not_panic() {
    // 失效不存在的角色缓存不应 panic
    invalidate_permission_cache(99991);
    invalidate_permission_cache(99992);
    invalidate_permission_cache(0);
    invalidate_permission_cache(i32::MAX);
}

/// 场景 9（14.11-C 公共 API 边界）：invalidate_all_permission_cache 不 panic
#[tokio::test]
async fn test_invalidate_all_permission_cache_does_not_panic() {
    // 清空全部缓存不应 panic（重复调用也安全）
    invalidate_all_permission_cache();
    invalidate_all_permission_cache();
}

/// 场景 10（14.11-C 公共 API 边界）：先失效单个再清空全部，不 panic
#[tokio::test]
async fn test_invalidate_cache_mixed_operations_safe() {
    invalidate_permission_cache(99993);
    invalidate_all_permission_cache();
    invalidate_permission_cache(99994);
    invalidate_all_permission_cache();
}
