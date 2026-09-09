//! A.21.3 RLS 上下文机制集成测试（dept 语义版，m_rls_dept_domain）
//!
//! 验证 rls_context_middleware 的核心行为（task-local RlsGuc 方案）：
//! 1. 非 admin 用户请求进入 handler 时，task-local RLS_GUC 已绑定其 user_id
//!    （dept 用户额外绑定 dept_ids 逗号串）
//! 2. admin 用户（data_scope=all）无 task-local 上下文（钩子走 RESET 分支）
//! 3. 无 AuthContext（未认证/公开路径）无 task-local 上下文
//! 4. task-local 未进入作用域的独立 task（spawn 旁路）读到 None
//!
//! 设计说明：
//! - task-local 在进程内可直接观测（探针 handler 读 current_rls_guc），无需 PG
//! - 连接池钩子机制（set_config 在同一连接生效）的 PG 真实验证见
//!   test_rls_guc_visible_in_same_pool_pg（#[ignore]，需 PostgreSQL）
//!
//! 参考实现：backend/src/middleware/rls_context.rs
//! 参考策略：backend/migration/src/domain/rls_dept/mod.rs（RLS 策略 fail-open）

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
use bingxi_backend::container::AppState;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::rls_context::{
    current_rls_guc, rls_context_middleware, with_rls_context, RlsGuc,
};
use common::setup_test_db;
use std::sync::Arc;
use tower::ServiceExt;

/// 探针处理器：读当前 task 的 RLS GUC 并返回给断言
async fn probe_handler() -> impl IntoResponse {
    let snapshot = current_rls_guc().map(|g| {
        serde_json::json!({
            "user_id": g.user_id,
            "dept_ids": g.dept_ids.as_ref().map(|s| s.as_str().to_string()),
        })
    });
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "rls_guc": snapshot })),
    )
}

/// AuthContext 注入中间件：将预设 AuthContext 写入 request extensions
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
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

/// 构造 dept 用户 AuthContext（含 dept_ids）
fn make_dept_auth(
    user_id: i32,
    username: &str,
    dept_ids_csv: &str,
) -> AuthContext {
    let mut auth = make_auth(user_id, username, Some("dept"));
    auth.dept_ids = Some(Arc::new(dept_ids_csv.to_string()));
    auth
}

/// 构建测试 Router，层序与生产一致：
/// inject_auth（外层，先执行）→ rls_context_middleware（内层，auth 之后执行）→ probe_handler
fn build_test_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/erp/test", get(probe_handler))
        .layer(from_fn_with_state(state.clone(), rls_context_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
}

/// 构建无 AuthContext 注入的 Router
fn build_test_app_no_auth(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/erp/test", get(probe_handler))
        .layer(from_fn_with_state(state, rls_context_middleware))
}

async fn build_sqlite_app_state() -> AppState {
    let db = setup_test_db().await;
    AppState {
        db: Arc::new(db),
        ..AppState::default()
    }
}

/// 发送测试请求并解析探针返回的 RLS GUC 快照
async fn send_probe(app: Router) -> Option<serde_json::Value> {
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
    // null 字段归一化为 None（admin/未认证场景 rls_guc 序列化为 null）
    match json.get("rls_guc") {
        Some(serde_json::Value::Null) | None => None,
        Some(v) => Some(v.clone()),
    }
}

// =========================================================================
// 测试 1：self 用户绑定 user_id，无 dept_ids
// =========================================================================

#[tokio::test]
async fn test_self_user_binds_user_id_only() {
    let state = build_sqlite_app_state().await;
    let auth = make_auth(1001, "operator_user", Some("self"));
    let app = build_test_app(state, auth);

    let snap = send_probe(app).await.expect("self 用户应有 GUC 快照");
    assert_eq!(snap["user_id"], 1001, "self 用户应绑定 user_id");
    assert!(
        snap["dept_ids"].is_null(),
        "self 用户 dept_ids 应为 None"
    );
}

// =========================================================================
// 测试 2：dept 用户绑定 user_id + dept_ids 逗号串
// =========================================================================

#[tokio::test]
async fn test_dept_user_binds_user_id_and_dept_ids() {
    let state = build_sqlite_app_state().await;
    let auth = make_dept_auth(2002, "sales_manager", "1,3,5");
    let app = build_test_app(state, auth);

    let snap = send_probe(app).await.expect("dept 用户应有 GUC 快照");
    assert_eq!(snap["user_id"], 2002, "dept 用户应绑定 user_id");
    assert_eq!(
        snap["dept_ids"].as_str(),
        Some("1,3,5"),
        "dept 用户应绑定 dept_ids 逗号串（连接池钩子据此设置 app.dept_ids GUC）"
    );
}

// =========================================================================
// 测试 3：admin 用户（data_scope=all）无 RLS 上下文
// =========================================================================

#[tokio::test]
async fn test_admin_user_skips_rls_context() {
    let state = build_sqlite_app_state().await;
    let auth = make_auth(9001, "admin_user", Some("all"));
    // 即便手动塞了 dept_ids，all 分支也不应绑定（中间件跳过）
    let app = build_test_app(state, auth);

    let snap = send_probe(app).await;
    assert!(snap.is_none(), "admin（data_scope=all）应无 RLS 上下文");
}

// =========================================================================
// 测试 4：无 AuthContext 无 RLS 上下文
// =========================================================================

#[tokio::test]
async fn test_no_auth_context_has_no_rls_binding() {
    let state = build_sqlite_app_state().await;
    let app = build_test_app_no_auth(state);

    let snap = send_probe(app).await;
    assert!(snap.is_none(), "未认证请求应无 RLS 上下文");
}

// =========================================================================
// 测试 5：data_scope=None 视为非 admin（最小权限原则）
// =========================================================================

#[tokio::test]
async fn test_none_data_scope_treated_as_non_admin() {
    let state = build_sqlite_app_state().await;
    let auth = make_auth(3001, "no_scope_user", None);
    let app = build_test_app(state, auth);

    let snap = send_probe(app).await.expect("data_scope=None 用户应有 GUC 快照");
    assert_eq!(snap["user_id"], 3001, "data_scope=None 应视为非 admin，绑定 user_id");
}

// =========================================================================
// 测试 6：spawn 出的 task 无 task-local（fail-open 语义）
// =========================================================================

#[tokio::test]
async fn test_spawned_task_without_scope_reads_none() {
    let inner = tokio::spawn(async move { current_rls_guc() });
    let snap = inner.await.expect("spawn task 执行失败");
    assert!(snap.is_none(), "未进入 rls_context_middleware 作用域的 task 应读到 None");
}

// =========================================================================
// 测试 7：with_rls_context 作用域内读到绑定值（含 dept_ids）
// =========================================================================

#[tokio::test]
async fn test_with_rls_context_scopes_guc() {
    let guc = Some(RlsGuc {
        user_id: 4242,
        dept_ids: Some(Arc::new("7,8".to_string())),
    });
    let bound = with_rls_context(guc, async { current_rls_guc() }).await;
    let b = bound.expect("作用域内应读到 GUC");
    assert_eq!(b.user_id, 4242);
    assert_eq!(b.dept_ids.as_ref().map(|s| s.as_str()), Some("7,8"));

    let cleared = with_rls_context(None, async { current_rls_guc() }).await;
    assert!(cleared.is_none(), "with_rls_context(None) 作用域内应为 None");
}

// =========================================================================
// 测试 8（PG 机制锚点）：连接池钩子在业务查询的同一连接上设置双 GUC
// =========================================================================
// 验证 set_config('app.user_id') + set_config('app.dept_ids') 在借出连接上生效，
// 且无上下文借出时 RESET 清理双 GUC。需 PostgreSQL（set_config 为 PG 专属）。
// SQLite 不支持，故标记 #[ignore]；TEST_DATABASE_URL 指向 PG 时运行：
//   cargo test --test rls_context_test -- --ignored
#[tokio::test]
#[ignore = "需 PostgreSQL 环境。设置 TEST_DATABASE_URL 后 cargo test -- --ignored 运行"]
async fn test_rls_guc_visible_in_same_pool_pg() {
    use bingxi_backend::middleware::rls_context::install_rls_pool_hooks;
    use sea_orm::{ConnectOptions, ConnectionTrait, QueryResult};

    let db_url = std::env::var("TEST_DATABASE_URL").unwrap_or_default();
    if !db_url.starts_with("postgres") {
        eprintln!("跳过：TEST_DATABASE_URL 未指向 PostgreSQL（当前: {db_url}）");
        return;
    }

    // max=1 保证两次查询复用同一物理连接，使「设置可见 / RESET 清理」断言确定性
    let mut opts = ConnectOptions::new(db_url);
    opts.max_connections(1).min_connections(1);
    install_rls_pool_hooks(&mut opts);
    let db = sea_orm::Database::connect(opts)
        .await
        .expect("连接 PostgreSQL 失败");

    // current_setting(name, true) 未设置时返回空字符串；归一化为 None 便于断言
    let read_setting = |row: Option<QueryResult>, key: &str| -> Option<String> {
        row.and_then(|r| r.try_get::<String>("", key).ok())
            .filter(|s| !s.is_empty())
    };

    // 1) dept 上下文：双 GUC 都应在借出连接上可见
    let guc = RlsGuc {
        user_id: 2002,
        dept_ids: Some(Arc::new("1,3,5".to_string())),
    };
    let uid = with_rls_context(Some(guc), async {
        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT current_setting('app.user_id', true) AS uid".to_owned(),
            ))
            .await
            .expect("查询 app.user_id 失败");
        read_setting(row, "uid")
    })
    .await;
    assert_eq!(uid.as_deref(), Some("2002"), "app.user_id 应在借出连接上设置");

    let dept_csv = with_rls_context(
        Some(RlsGuc {
            user_id: 2002,
            dept_ids: Some(Arc::new("1,3,5".to_string())),
        }),
        async {
            let row = db
                .query_one_raw(sea_orm::Statement::from_string(
                    sea_orm::DatabaseBackend::Postgres,
                    "SELECT current_setting('app.dept_ids', true) AS dept_csv".to_owned(),
                ))
                .await
                .expect("查询 app.dept_ids 失败");
            read_setting(row, "dept_csv")
        },
    )
    .await;
    assert_eq!(
        dept_csv.as_deref(),
        Some("1,3,5"),
        "app.dept_ids 应在借出连接上设置（dept 语义激活）"
    );

    // 2) 无上下文：双 GUC 应被 RESET 清理（防跨请求泄漏）
    let uid2 = with_rls_context(None, async {
        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT current_setting('app.user_id', true) AS uid".to_owned(),
            ))
            .await
            .expect("查询 app.user_id 失败");
        read_setting(row, "uid")
    })
    .await;
    assert_eq!(uid2, None, "无上下文借出应 RESET app.user_id");

    let dept_csv2 = with_rls_context(None, async {
        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT current_setting('app.dept_ids', true) AS dept_csv".to_owned(),
            ))
            .await
            .expect("查询 app.dept_ids 失败");
        read_setting(row, "dept_csv")
    })
    .await;
    assert_eq!(dept_csv2, None, "无上下文借出应 RESET app.dept_ids");
}
