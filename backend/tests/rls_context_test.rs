//! A.21.3 RLS 上下文机制集成测试（2026-09-08 重新设计后）
//!
//! 验证 rls_context_middleware 的核心行为（task-local 方案）：
//! 1. 非 admin 用户请求进入 handler 时，task-local RLS_USER_ID 已绑定其 user_id
//!    （生产环境中连接池钩子据此在同一连接上执行 set_config，激活 PG RLS）
//! 2. admin 用户（data_scope=all）无 task-local 上下文（借出钩子走 RESET 分支）
//! 3. 无 AuthContext（未认证/公开路径）无 task-local 上下文
//! 4. task-local 未进入作用域的独立 task（模拟 spawn 旁路）读到 None
//!
//! 设计说明：
//! - task-local 在进程内可直接观测（探针 handler 读 current_rls_user_id()），
//!   无需 PG 即可断言中间件行为；sqlite 连接池不安装钩子，无副作用
//! - 连接池钩子机制（set_config 在同一连接生效）的 PG 真实验证见
//!   test_rls_guc_visible_in_same_pool_pg（#[ignore]，需 PostgreSQL）
//!
//! 参考实现：backend/src/middleware/rls_context.rs
//! 参考策略：backend/migration/src/domain/finance/mod.rs（RLS 策略 fail-open）

#[path = "test_common/mod.rs"]
mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::get,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::rls_context::{
    current_rls_user_id, rls_context_middleware, with_rls_context,
};
use bingxi_backend::container::AppState;
use common::setup_test_db;
use std::sync::Arc;
use tower::ServiceExt;

/// 探针处理器：读当前 task 的 RLS user_id 并返回给断言
async fn probe_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "rls_user_id": current_rls_user_id() })),
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

/// 构建测试 Router，层序与生产一致：
/// inject_auth（后注册=外层，先执行）→ rls_context_middleware（内层，auth 之后执行）→ probe_handler
fn build_test_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/erp/test", get(probe_handler))
        .layer(from_fn_with_state(state.clone(), rls_context_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
}

/// 构建无 AuthContext 注入的 Router（模拟未认证/公开路径）
fn build_test_app_no_auth(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/erp/test", get(probe_handler))
        .layer(from_fn_with_state(state, rls_context_middleware))
}

/// 构造一个使用 sqlite::memory: 的 AppState（RLS 钩子仅注册于 PG 构建路径，sqlite 无副作用）
async fn build_sqlite_app_state() -> AppState {
    let db = setup_test_db().await;
    AppState {
        db: Arc::new(db),
        ..AppState::default()
    }
}

/// 发送测试请求并解析探针返回的 rls_user_id
async fn send_probe(app: Router) -> Option<i32> {
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/erp/test")
        .body(Body::empty())
        .expect("构造请求失败");
    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(resp.status(), StatusCode::OK, "探针请求应返回 200");
    let body = axum::body::to_bytes(resp.into_body(), 4096)
        .await
        .expect("读取响应体失败");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("解析 JSON 失败");
    json["rls_user_id"].as_i64().map(|v| v as i32)
}

// =========================================================================
// 测试 1：非 admin 用户请求时，task-local 已绑定 user_id
// =========================================================================
// 生产环境中，连接池钩子读到该值后在业务查询的同一连接上执行
// set_config('app.user_id', '<id>', false)，激活 PG RLS 策略。

#[tokio::test]
async fn test_non_admin_user_binds_rls_context() {
    let state = build_sqlite_app_state().await;
    let auth = make_auth(1001, "operator_user", Some("self"));
    let app = build_test_app(state, auth);

    let bound = send_probe(app).await;
    assert_eq!(
        bound,
        Some(1001),
        "非 admin 用户的 user_id 应绑定到请求 task-local（连接池钩子据此激活 RLS）"
    );
}

// =========================================================================
// 测试 2：admin 用户（data_scope=all）无 RLS 上下文
// =========================================================================
// admin 跳过 RLS：task-local 为 None，借出钩子走 RESET 分支，
// current_setting('app.user_id', true) 返回 NULL，策略放行全量数据。

#[tokio::test]
async fn test_admin_user_skips_rls_context() {
    let state = build_sqlite_app_state().await;
    let auth = make_auth(9001, "admin_user", Some("all"));
    let app = build_test_app(state, auth);

    let bound = send_probe(app).await;
    assert_eq!(
        bound, None,
        "admin（data_scope=all）应无 RLS 上下文（钩子走 RESET 分支，策略 NULL 放行）"
    );
}

// =========================================================================
// 测试 3：无 AuthContext（未认证/公开路径）无 RLS 上下文
// =========================================================================

#[tokio::test]
async fn test_no_auth_context_has_no_rls_binding() {
    let state = build_sqlite_app_state().await;
    let app = build_test_app_no_auth(state);

    let bound = send_probe(app).await;
    assert_eq!(
        bound, None,
        "未认证请求应无 RLS 上下文，中间件不 panic"
    );
}

// =========================================================================
// 测试 4：data_scope=None 的用户走非 admin 分支（最小权限原则）
// =========================================================================
// data_scope=None 时 unwrap_or(false) → is_admin_scope=false → 绑定 user_id。

#[tokio::test]
async fn test_none_data_scope_treated_as_non_admin() {
    let state = build_sqlite_app_state().await;
    let auth = make_auth(3001, "no_scope_user", None);
    let app = build_test_app(state, auth);

    let bound = send_probe(app).await;
    assert_eq!(
        bound,
        Some(3001),
        "data_scope=None 应视为非 admin（最小权限原则），绑定 RLS 上下文"
    );
}

// =========================================================================
// 测试 5：未进入中间件作用域的独立 task 读到 None（spawn 旁路任务语义）
// =========================================================================
// spawn 出的任务不继承 task-local：钩子对其借出的连接执行 RESET，
// RLS fail-open 放行（与旧行为一致，无回归），应用层兜底。

#[tokio::test]
async fn test_spawned_task_without_scope_reads_none() {
    let inner = tokio::spawn(async move { current_rls_user_id() });
    let bound = inner.await.expect("spawn task 执行失败");
    assert_eq!(
        bound, None,
        "未进入 rls_context_middleware 作用域的 task 应读到 None（fail-open 语义）"
    );
}

// =========================================================================
// 测试 6：with_rls_context 作用域内读到绑定值（钩子验证辅助 API）
// =========================================================================

#[tokio::test]
async fn test_with_rls_context_scopes_value() {
    let bound = with_rls_context(Some(4242), async { current_rls_user_id() }).await;
    assert_eq!(bound, Some(4242), "with_rls_context 作用域内应读到绑定值");

    let cleared = with_rls_context(None, async { current_rls_user_id() }).await;
    assert_eq!(cleared, None, "with_rls_context(None) 作用域内应为 None");
}

// =========================================================================
// 测试 7（PG 机制锚点）：连接池钩子在业务查询的同一连接上设置 GUC
// =========================================================================
// 核心机制端到端验证：用与生产一致的 connect 路径（安装 RLS 钩子）连接 PostgreSQL，
// 在 task-local 上下文内执行查询，断言 current_setting('app.user_id') 在该连接上可见。
// 这正是旧实现（独立连接 SET LOCAL，事务外无效）做不到的事。
// SQLite 不支持 set_config，故标记 #[ignore]；TEST_DATABASE_URL 指向 PostgreSQL 时运行：
//   cargo test --test rls_context_test -- --ignored
#[tokio::test]
#[ignore = "需 PostgreSQL 环境（set_config 为 PG 专属）。设置 TEST_DATABASE_URL 后 cargo test -- --ignored 运行"]
async fn test_rls_guc_visible_in_same_pool_pg() {
    use bingxi_backend::middleware::rls_context::install_rls_pool_hooks;
    use sea_orm::{ConnectOptions, ConnectionTrait, QueryResult};

    let db_url = std::env::var("TEST_DATABASE_URL").unwrap_or_default();
    if !db_url.starts_with("postgres") {
        eprintln!(
            "跳过：TEST_DATABASE_URL 未指向 PostgreSQL（当前: {db_url}）"
        );
        return;
    }

    // 与生产 connect_database 相同的钩子安装路径；max=1 保证两次查询
    // 复用同一物理连接，使「设置可见 / RESET 清理」断言具有确定性
    let mut opts = ConnectOptions::new(db_url);
    opts.max_connections(1).min_connections(1);
    install_rls_pool_hooks(&mut opts);
    let db = sea_orm::Database::connect(opts)
        .await
        .expect("连接 PostgreSQL 失败");

    // current_setting(name, true) 在未设置时返回空字符串；归一化为 None 便于断言
    let read_uid = |row: Option<sea_orm::QueryResult>| -> Option<String> {
        row.and_then(|r| r.try_get::<String>("", "uid").ok())
            .filter(|s| !s.is_empty())
    };

    // 1) 有上下文：查询应在同一连接上看到 user_id
    let seen = with_rls_context(Some(2002), async {
        let row = db
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT current_setting('app.user_id', true) AS uid".to_owned(),
            ))
            .await
            .expect("查询 current_setting 失败");
        read_uid(row)
    })
    .await;
    assert_eq!(
        seen.as_deref(),
        Some("2002"),
        "RLS 钩子应在业务查询的同一连接上设置 app.user_id（机制核心）"
    );

    // 2) 无上下文：借出钩子应 RESET 掉上一任设置（不泄漏）
    let cleared = with_rls_context(None, async {
        let row = db
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT current_setting('app.user_id', true) AS uid".to_owned(),
            ))
            .await
            .expect("查询 current_setting 失败");
        read_uid(row)
    })
    .await;
    assert_eq!(
        cleared, None,
        "无上下文借出时钩子应 RESET app.user_id，current_setting 应为空（防跨请求泄漏）"
    );
}
