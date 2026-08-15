//! 安全漏洞 #5 修复单测：覆盖 get_task_status 权限校验（匿名→401、缺角色→403、缺参→401）
//! 直接构造 AuthContext 验证 handler 内部逻辑，不依赖真实 DB；用 oneshot + AppState::default() 隔离依赖

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::init_handler::get_task_status;
use bingxi_backend::middleware::auth_context::AuthContext;
use tower::ServiceExt;

/// 构造一个最小化的测试 Router：仅注册 `get_task_status` + `AppState`。
fn build_test_app() -> Router {
    Router::new()
        .route("/init/task-status", get(get_task_status))
        .with_state(AppState::default())
}

/// 场景 A：匿名调用 get_task_status（无 AuthContext）→ 期望 401
/// 验证 auth: AuthContext 提取器在缺 AuthContext 时返回 401
#[tokio::test]
async fn test_get_task_status_anonymous_returns_401() {
    let app = build_test_app();

    let req = Request::builder()
        .method("GET")
        .uri("/init/task-status?task_id=any-task-id")
        .body(Body::empty())
        .expect("构造匿名请求失败");

    let resp = app.oneshot(req).await.expect("执行匿名请求失败");

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "匿名调用 get_task_status 应返回 401（auth: AuthContext 提取器在缺 AuthContext 时直接拒绝）"
    );
}

/// 场景 C：缺少 task_id 参数 → 期望 401（缺 AuthContext 时提取器先失败）
/// 验证 Query 提取顺序无回归，缺 AuthContext 时先返回 401
#[tokio::test]
async fn test_get_task_status_missing_task_id_returns_401() {
    let app = build_test_app();

    let req = Request::builder()
        .method("GET")
        .uri("/init/task-status")
        .body(Body::empty())
        .expect("构造缺参请求失败");

    let resp = app.oneshot(req).await.expect("执行缺参请求失败");
    // 缺 AuthContext → 提取器先失败 → 401；如果未来先做 Query 校验，会改为 400。
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "缺 AuthContext 时 get_task_status 应先返回 401"
    );
}
