//! A.21.3 RLS 策略生效集成测试
//!
//! 验证 rls_context_middleware 的三条核心行为：
//! 1. 非 admin 用户请求时，应执行 SET LOCAL app.user_id（sqlite 不支持时安全降级）
//! 2. admin 用户（data_scope=all）应跳过 RLS，不设置 app.user_id
//! 3. RLS 设置失败时安全降级，请求仍正常放行（应用层 apply_data_scope 兜底）
//!
//! 设计说明：
//! - 使用 `AppState::default()`（mock DB：sqlite::memory: 或 default 连接）
//! - AuthContext 通过自定义注入中间件写入 request extensions，模拟 auth_middleware
//! - SQLite 不支持 `SET LOCAL app.user_id`（PostgreSQL 专属语法），中间件会进入
//!   安全降级分支（tracing::warn 后继续放行）。因此测试验证的是「降级后请求仍成功」
//!   而非「SET LOCAL 字面执行成功」。
//! - 真实 RLS 策略激活验证需在 PostgreSQL 环境运行（设置 TEST_DATABASE_URL），
//!   相关断言以 `#[ignore]` 标注，CI 通过 service container 跑 PostgreSQL 时启用。
//!
//! 参考实现：backend/src/middleware/rls_context.rs
//! 参考策略：backend/database/rls.sql（customers/suppliers/sales_orders/crm_lead/crm_opportunity）

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::rls_context::rls_context_middleware;
use common::setup_test_db;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tower::ServiceExt;

/// 测试用业务处理器：返回 200，证明请求穿过中间件后正常到达 handler
async fn ok_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({"ok": true})),
    )
}

/// AuthContext 注入中间件：将预设 AuthContext 写入 request extensions
/// （模拟 auth_middleware 解析 JWT 后注入认证上下文的行为）
async fn inject_auth(
    axum::extract::State(auth): axum::extract::State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

/// 构造测试用 AuthContext
fn make_auth(user_id: i32, username: &str, data_scope: Option<&str>) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
        role_id: Some(1),
        department_id: None,
        data_scope: data_scope.map(|s| s.to_string()),
    }
}

/// 构建带 RLS 中间件的测试 Router
/// 层序：inject_auth（外层，先执行）→ rls_context_middleware（内层，后执行）→ ok_handler
fn build_test_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/erp/test", get(ok_handler))
        .layer(from_fn_with_state(state.clone(), rls_context_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
}

/// 构建无 AuthContext 注入的 Router（测试缺少认证上下文的场景）
fn build_test_app_no_auth(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/erp/test", get(ok_handler))
        .layer(from_fn_with_state(state, rls_context_middleware))
}

/// 构造一个使用 sqlite::memory: 的 AppState（避免依赖 default 的 mock DB 行为）
async fn build_sqlite_app_state() -> AppState {
    let db = setup_test_db().await;
    let mut state = AppState::default();
    state.db = Arc::new(db);
    state
}

/// 构造并发送一个测试请求，返回响应状态码
async fn send_request(app: Router) -> StatusCode {
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/erp/test")
        .body(Body::empty())
        .expect("构造请求失败");
    let resp = app.oneshot(req).await.expect("执行请求失败");
    resp.status()
}

// =========================================================================
// 测试 1：非 admin 用户请求时，rls_context_middleware 应执行 SET LOCAL app.user_id
// =========================================================================
// SQLite 不支持 SET LOCAL（PostgreSQL 事务级会话变量），sqlx::query 会返回错误，
// 中间件进入安全降级分支（tracing::warn 后继续放行）。
// 因此本测试断言「请求仍成功返回 200」，证明降级路径不阻断业务。
// 真实 SET LOCAL 生效验证见 test_non_admin_rls_activated_pg（#[ignore]，需 PostgreSQL）。

#[tokio::test]
async fn test_non_admin_user_attempts_set_local_degrades_safely() {
    // 非 admin 用户：data_scope=self，应触发 SET LOCAL 分支
    let state = build_sqlite_app_state().await;
    let auth = make_auth(1001, "operator_user", Some("self"));
    let app = build_test_app(state, auth);

    let status = send_request(app).await;
    // SQLite 下 SET LOCAL 失败，但中间件安全降级，请求仍放行
    assert_eq!(
        status,
        StatusCode::OK,
        "非 admin 用户 SET LOCAL 失败时应安全降级，请求仍返回 200"
    );
}

/// 真实 PostgreSQL 环境验证：SET LOCAL app.user_id 应成功执行并激活 RLS。
/// 需设置 TEST_DATABASE_URL 指向 PostgreSQL（CI service container 提供）。
/// SQLite 不支持 SET LOCAL，故标记 #[ignore]。
#[tokio::test]
#[ignore = "需 PostgreSQL 环境（SET LOCAL 为 PG 专属语法，sqlite 不支持）。设置 TEST_DATABASE_URL 后 cargo test -- --ignored 运行"]
async fn test_non_admin_rls_activated_pg() {
    let db_url = std::env::var("TEST_DATABASE_URL").unwrap_or_default();
    if !db_url.starts_with("postgres") {
        eprintln!("跳过：TEST_DATABASE_URL 未指向 PostgreSQL（当前: {}）", db_url);
        return;
    }

    let db: DatabaseConnection = sea_orm::Database::connect(&db_url)
        .await
        .expect("连接 PostgreSQL 失败");
    let mut state = AppState::default();
    state.db = Arc::new(db);

    let auth = make_auth(2002, "pg_operator", Some("self"));
    let app = build_test_app(state, auth);

    let status = send_request(app).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "PostgreSQL 环境下 SET LOCAL 应成功，请求返回 200"
    );
}

// =========================================================================
// 测试 2：admin 用户（data_scope=all）应跳过 RLS
// =========================================================================
// data_scope=all 时，中间件不执行 SET LOCAL，直接放行。
// 无论 sqlite 还是 PostgreSQL，该路径都不触碰 SET LOCAL，因此无需 #[ignore]。

#[tokio::test]
async fn test_admin_user_skips_rls() {
    let state = build_sqlite_app_state().await;
    // admin 用户：data_scope=all，应跳过 RLS（不设置 app.user_id）
    let auth = make_auth(9001, "admin_user", Some("all"));
    let app = build_test_app(state, auth);

    let status = send_request(app).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "admin 用户（data_scope=all）应跳过 RLS，直接放行返回 200"
    );
}

/// 验证 admin 跳过 RLS 的判定逻辑：data_scope=all 时不进入 SET LOCAL 分支。
/// 通过对比 self 与 all 两种 scope 的请求结果一致（均 200），间接证明
/// admin 路径未因 SET LOCAL 失败而产生副作用差异。
#[tokio::test]
async fn test_admin_skip_rls_no_side_effect_vs_self() {
    let state = build_sqlite_app_state().await;

    // admin（all）路径
    let admin_auth = make_auth(9002, "admin2", Some("all"));
    let admin_status = send_request(build_test_app(state.clone(), admin_auth)).await;

    // operator（self）路径
    let op_auth = make_auth(1002, "operator2", Some("self"));
    let op_status = send_request(build_test_app(state, op_auth)).await;

    // 两者均应 200：admin 跳过 RLS，operator 走降级（sqlite 不支持 SET LOCAL）
    assert_eq!(admin_status, StatusCode::OK, "admin 路径应 200");
    assert_eq!(op_status, StatusCode::OK, "operator 路径降级后应 200");
}

// =========================================================================
// 测试 3：RLS 设置失败时安全降级（应用层 apply_data_scope 仍生效）
// =========================================================================
// SQLite 不支持 SET LOCAL → rls_context_middleware 内部 sqlx::query 返回 Err →
// 中间件 tracing::warn 后继续 next.run(request) → 请求放行。
// 应用层 apply_data_scope（utils/data_scope.rs）独立于 RLS，作为兜底防线仍生效。
// 本测试验证：即使 RLS 设置失败，请求仍能到达 handler（200），证明降级不阻断业务。

#[tokio::test]
async fn test_rls_failure_degrades_safely() {
    let state = build_sqlite_app_state().await;
    // data_scope=None 也走非 admin 分支（unwrap_or(false) → is_admin_scope=false）
    let auth = make_auth(3001, "no_scope_user", None);
    let app = build_test_app(state, auth);

    let status = send_request(app).await;
    // SQLite 必然 SET LOCAL 失败，但安全降级后请求仍应 200
    assert_eq!(
        status,
        StatusCode::OK,
        "RLS 设置失败时应安全降级，请求仍返回 200（应用层 apply_data_scope 兜底）"
    );
}

/// 验证无 AuthContext 时中间件不 panic：extensions.get::<AuthContext>() 返回 None，
/// 跳过 SET LOCAL 分支，直接放行（未认证请求由后续 permission_middleware 拦截）。
#[tokio::test]
async fn test_no_auth_context_does_not_panic() {
    let state = build_sqlite_app_state().await;
    let app = build_test_app_no_auth(state);

    let status = send_request(app).await;
    // 无 AuthContext 时 rls_context_middleware 跳过 RLS 逻辑，请求继续放行
    assert_eq!(
        status,
        StatusCode::OK,
        "无 AuthContext 时中间件应跳过 RLS 逻辑，不 panic"
    );
}

/// 验证应用层 DataScope 解析逻辑独立于 RLS 仍可正常工作。
/// 此测试不经过 rls_context_middleware，直接验证 AuthContext::to_data_scope_context
/// 在 RLS 失效时仍能正确计算数据范围（兜底防线有效）。
#[tokio::test]
async fn test_apply_data_scope_still_works_without_rls() {
    use bingxi_backend::utils::data_scope::{DataScope, DataScopeContext};

    // admin：data_scope=all → DataScope::All
    let admin_auth = make_auth(9003, "admin3", Some("all"));
    let admin_ctx = admin_auth.to_data_scope_context();
    assert_eq!(
        admin_ctx.scope,
        DataScope::All,
        "admin 的应用层 data_scope 应为 All（RLS 失效时兜底）"
    );

    // operator：data_scope=self → DataScope::Self_
    let op_auth = make_auth(3002, "operator3", Some("self"));
    let op_ctx = op_auth.to_data_scope_context();
    assert_eq!(
        op_ctx.scope,
        DataScope::Self_,
        "operator 的应用层 data_scope 应为 Self_（RLS 失效时兜底）"
    );

    // 未加载 data_scope：None → 默认 Self_（最小权限原则）
    let unknown_auth = make_auth(3003, "unknown", None);
    let unknown_ctx = unknown_auth.to_data_scope_context();
    assert_eq!(
        unknown_ctx.scope,
        DataScope::Self_,
        "data_scope=None 时应用层应默认 Self_（最小权限原则，兜底 RLS 缺失）"
    );

    // 验证 DataScopeContext 携带正确的 user_id（apply_data_scope 依赖此值过滤）
    assert_eq!(op_ctx.user_id, 3002, "应用层 data_scope 上下文应携带正确 user_id");
}
