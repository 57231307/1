// 防御性 allow：clippy 1.94 对 integration test 文件的 import 严格，
// 场景裁剪（如仅保留 IP mismatch 测试）可能导致部分 import 暂时未消费，
// 预先抑制避免 CI 抖动。
#![allow(unused_imports)]

//! CSRF 中间件集成测试
//!
//! 覆盖以下场景：
//! 1. ✅ GET 请求无 token → 通过
//! 2. ✅ POST 请求无 X-CSRF-Token 头 → 403 CSRF_TOKEN_MISSING
//! 3. ✅ POST 请求 token 无效 → 403 CSRF_TOKEN_INVALID
//! 4. ✅ POST 请求 token 有效 → 200 通过
//! 5. ✅ 公开路径 POST（如 login）无 token → 通过
//!
//! 设计说明：
//! - 测试通过 `tower::ServiceExt::oneshot` 配合 axum 0.7 的 Router 隔离中间件逻辑
//! - 注入最小化 `AppState::default()`（仅用到 `state.cache`）
//! - 不依赖数据库，OmniAuditEngine 的后台任务在默认连接上静默失败

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bingxi_backend::middleware::csrf::csrf_middleware;
use bingxi_backend::utils::app_state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;

/// 测试用业务处理器：直接返回 200 + 简易 JSON
async fn ok_handler() -> impl IntoResponse {
    Json(json!({"ok": true, "message": "业务处理器返回成功"}))
}

/// 构建用于测试的最小化 Router
///
/// 仅注册 csrf 中间件 + 一个 GET 处理器 + 一个 POST 处理器。
/// 路径与后端 `public_routes::PUBLIC_PATHS` 保持一致，便于覆盖公开路径分支。
fn build_test_app(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/erp/business/list", get(ok_handler))
        .route("/api/v1/erp/business/create", post(ok_handler))
        // 模拟公开路径（与 PUBLIC_PATHS 中的 /api/v1/erp/auth/login 对齐）
        .route("/api/v1/erp/auth/login", post(ok_handler))
        .layer(from_fn_with_state(state, csrf_middleware))
}

/// 读取响应体为 JSON 值
async fn read_json(body: axum::body::Body) -> Value {
    let bytes = axum::body::to_bytes(body, 4096)
        .await
        .expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("响应体不是合法 JSON")
}

/// 场景 1：GET 请求无 token → 应放行通过
#[tokio::test]
async fn test_get_request_passes_without_csrf_token() {
    let state = AppState::default();
    let app = build_test_app(state);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/erp/business/list")
        .body(Body::empty())
        .expect("构造 GET 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET 请求应被 CSRF 中间件放行（无副作用）"
    );
}

/// 场景 2：POST 请求无 X-CSRF-Token 头 → 403 CSRF_TOKEN_MISSING
#[tokio::test]
async fn test_post_without_csrf_header_returns_missing() {
    let state = AppState::default();
    let app = build_test_app(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("构造 POST 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "无 X-CSRF-Token 头的 POST 应返回 403"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        body.get("success").and_then(|v| v.as_bool()),
        Some(false),
        "success 字段应为 false"
    );
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_TOKEN_MISSING"),
        "业务码应为 CSRF_TOKEN_MISSING，实际: {:?}",
        body.get("code")
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some("CSRF Token 缺失"),
        "message 应为常量 CSRF Token 缺失"
    );
}

/// 场景 3：POST 请求带无效 token → 403 CSRF_TOKEN_INVALID
#[tokio::test]
async fn test_post_with_invalid_csrf_token_returns_invalid() {
    let state = AppState::default();
    let app = build_test_app(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json")
        .header("x-csrf-token", "this-token-does-not-exist-in-cache")
        .body(Body::from("{}"))
        .expect("构造 POST 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "无效 CSRF Token 的 POST 应返回 403"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_TOKEN_INVALID"),
        "业务码应为 CSRF_TOKEN_INVALID，实际: {:?}",
        body.get("code")
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some("CSRF Token 无效或已过期"),
        "message 应为常量 CSRF Token 无效或已过期"
    );
}

/// 场景 4：POST 请求带有效 token → 200 通过（且验证 rotation：第二次请求应被拒绝）
#[tokio::test]
async fn test_post_with_valid_csrf_token_passes_then_rotated() {
    let state = AppState::default();

    // 模拟登录流程：写入一个 CSRF Token 到缓存（Wave 3 #7：含 IP 绑定 + 反向索引）
    let valid_token = "valid-csrf-token-for-test-12345";
    state.cache.set_csrf_token(
        valid_token.to_string(),
        "session-1".to_string(),
        "203.0.113.10".to_string(), // 绑定 IP
        1001,                        // user_id（用于反向索引）
        None,
    );

    // 4.1 第一次请求：使用有效 token + 正确 IP → 应通过
    let app = build_test_app(state.clone());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json")
        .header("x-csrf-token", valid_token)
        .header("x-real-ip", "203.0.113.10")
        .body(Body::from("{}"))
        .expect("构造 POST 请求失败");

    let resp = app.oneshot(req).await.expect("执行第一次请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "有效 CSRF Token + 匹配 IP 的 POST 应返回 200"
    );

    // 4.2 第二次请求：同一 token 已被消费（rotation 模式）→ 应返回 403
    let app2 = build_test_app(state.clone());
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json")
        .header("x-csrf-token", valid_token)
        .header("x-real-ip", "203.0.113.10")
        .body(Body::from("{}"))
        .expect("构造第二次 POST 请求失败");

    let resp2 = app2.oneshot(req2).await.expect("执行第二次请求失败");
    assert_eq!(
        resp2.status(),
        StatusCode::FORBIDDEN,
        "rotation 模式下第二次使用同一 token 应被拒绝"
    );

    let body = read_json(resp2.into_body()).await;
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_TOKEN_INVALID"),
        "rotation 后第二次请求业务码应为 CSRF_TOKEN_INVALID"
    );
}

/// 场景 5：公开路径（如 auth/login）POST 无 token → 应放行通过
#[tokio::test]
async fn test_public_path_post_passes_without_csrf_token() {
    let state = AppState::default();
    let app = build_test_app(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/auth/login")
        .header("content-type", "application/json")
        .body(Body::from("{\"username\":\"admin\",\"password\":\"x\"}"))
        .expect("构造登录请求失败");

    let resp = app.oneshot(req).await.expect("执行登录请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "公开路径（login）的 POST 应被 CSRF 中间件放行"
    );
}

/// 场景 6：HEAD / OPTIONS 方法无 token → 应放行通过（无副作用）
#[tokio::test]
async fn test_head_options_methods_skip_csrf_check() {
    let state = AppState::default();
    let app = build_test_app(state);

    for method in ["HEAD", "OPTIONS"] {
        let req = Request::builder()
            .method(method)
            .uri("/api/v1/erp/business/list")
            .body(Body::empty())
            .expect("构造 HEAD/OPTIONS 请求失败");

        let resp = app
            .clone()
            .oneshot(req)
            .await
            .expect("执行 HEAD/OPTIONS 请求失败");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "{} 方法应被 CSRF 中间件放行（无副作用）",
            method
        );
    }
}

/// 场景 7：AppCache::consume_csrf_token 单元测试：消费后立即失效
#[tokio::test]
async fn test_consume_csrf_token_one_time_use() {
    let state = AppState::default();
    let cache = state.cache.clone();

    let token = "unit-test-token-xyz";
    // Wave 3 #7：新 API（接受 IP 参数 + 反向索引）
    cache.set_csrf_token(
        token.to_string(),
        "session-x".to_string(),
        "203.0.113.99".to_string(),
        2002,
        None,
    );

    // 第一次消费：IP 匹配 → 应成功
    assert_eq!(
        cache.consume_csrf_token(token, "203.0.113.99"),
        bingxi_backend::utils::cache::CsrfConsumeResult::Ok,
        "第一次消费（IP 匹配）应返回 Ok"
    );
    // 第二次消费：应失败（rotation 模式）
    assert_eq!(
        cache.consume_csrf_token(token, "203.0.113.99"),
        bingxi_backend::utils::cache::CsrfConsumeResult::NotFound,
        "第二次消费应返回 NotFound（token 已被移除）"
    );
    // 不存在的 token：应失败
    assert_eq!(
        cache.consume_csrf_token("non-existent-token", "203.0.113.99"),
        bingxi_backend::utils::cache::CsrfConsumeResult::NotFound,
        "不存在的 token 应返回 NotFound"
    );
}

/// 场景 8（Wave 3 #7）：POST 请求带有效 token 但 IP 不匹配 → 403 CSRF_IP_MISMATCH
#[tokio::test]
async fn test_post_with_valid_token_but_wrong_ip_returns_ip_mismatch() {
    let state = AppState::default();

    // 模拟登录：绑定 IP 203.0.113.10
    let valid_token = "valid-csrf-token-with-ip-binding";
    state.cache.set_csrf_token(
        valid_token.to_string(),
        "session-2".to_string(),
        "203.0.113.10".to_string(),
        1003,
        None,
    );

    // 客户端从不同 IP（198.51.100.99）发起请求
    let app = build_test_app(state);
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json")
        .header("x-csrf-token", valid_token)
        .header("x-real-ip", "198.51.100.99") // 与绑定 IP 不一致
        .body(Body::from("{}"))
        .expect("构造 POST 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "IP 不匹配的 POST 应返回 403"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_IP_MISMATCH"),
        "业务码应为 CSRF_IP_MISMATCH，实际: {:?}",
        body.get("code")
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some("CSRF Token IP 不匹配"),
        "message 应为常量 CSRF Token IP 不匹配"
    );
}

/// 场景 9（Wave 3 #7）：clear_old_csrf_token_for_user 强制轮换：旧 token 立即失效
#[tokio::test]
async fn test_clear_old_csrf_token_for_user_invalidates_token() {
    let state = AppState::default();
    let cache = state.cache.clone();

    let old_token = "old-csrf-token-before-rotation";
    cache.set_csrf_token(
        old_token.to_string(),
        "session-3".to_string(),
        "203.0.113.10".to_string(),
        1004,
        None,
    );

    // 用户重新登录 → 强制轮换
    let rotated = cache.clear_old_csrf_token_for_user(1004);
    assert!(rotated, "应返回 true（存在旧 token）");

    // 旧 token + 正确 IP → 仍应被拒绝（已被强制清除）
    let app = build_test_app(state);
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json")
        .header("x-csrf-token", old_token)
        .header("x-real-ip", "203.0.113.10")
        .body(Body::from("{}"))
        .expect("构造 POST 请求失败");

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "被强制轮换的旧 token 应返回 403"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_TOKEN_INVALID"),
        "强制轮换后旧 token 业务码应为 CSRF_TOKEN_INVALID"
    );
}
