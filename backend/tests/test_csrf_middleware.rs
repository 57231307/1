//! CSRF 中间件活体用例（tower oneshot，无数据库依赖）
//!
//! 覆盖以下场景：
//! 1. 安全方法（GET/HEAD/OPTIONS）无 token 放行
//! 2. POST 缺 X-CSRF-Token 即 403 CSRF_TOKEN_MISSING，并下发恢复 token
//! 3. POST token 无效 即 403 CSRF_TOKEN_INVALID，并下发恢复 token
//! 4. POST token 有效 得 200，rotation 后同 token 二次使用被拒
//! 5. 公开路径 POST（如 login）无 token 放行
//! 6. 恢复 token 可被同一会话同一用户直接消费（绑定核验不得把恢复通道打死）
//! 7. 缓存层一次性消费语义
//! 8. 同会话同用户下 IP 变化 时放行，且已按新 IP 重发 token，旧 token 在宽限窗口内仍可用
//! 9. 反向：token 属于别的用户 / 别的会话 / 请求无认证上下文 即 403 CSRF_TOKEN_BINDING_MISMATCH
//! 10. 反向：伪造 X-Real-IP 与绑定值逐字相同也不能替代会话核验
//! 11. 既有行为不回退：TTL 过期与强制轮换仍是 403 CSRF_TOKEN_INVALID
//!
//! 设计说明：
//! - 测试通过 `tower::ServiceExt::oneshot` 配合 axum Router 隔离中间件逻辑
//! - 注入最小化 `AppState::default()`（仅用到 `state.cache` 与 `state.omni_audit` 通道）
//! - 认证上下文与 JWT 会话必须与生产链路口径一致（csrf 挂在 auth 之内层），
//!   因此每个请求都要注入 AuthContext 与 CsrfSession，否则测的不是真实形态
//! - 不依赖数据库，OmniAuditEngine 的后台任务在默认连接上静默失败

use axum::{
    Json, Router,
    body::Body,
    http::{Request, StatusCode, header},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::middleware::auth_context::{AuthContext, CsrfSession};
use bingxi_backend::middleware::csrf::csrf_middleware;
use bingxi_backend::utils::cache::CsrfConsumeResult;
use serde_json::{Value, json};
use tower::ServiceExt;

const USER_ID: i32 = 1001;
const OTHER_USER_ID: i32 = 2002;
const SESSION: &str = "session-aaaa";
const OTHER_SESSION: &str = "session-bbbb";
const IP_WIFI: &str = "203.0.113.10";
const IP_CELL: &str = "198.51.100.99";

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
        .route("/api/v1/erp/business/list", get(ok_handler).options(ok_handler))
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

/// 业务码提取
fn code_of(body: &Value) -> Option<&str> {
    body.get("code").and_then(|v| v.as_str())
}

/// 构造认证上下文（字段与生产形态一致，仅 user_id 参与绑定核验）
fn auth_of(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("user-{user_id}"),
        role_id: Some(1),
        department_id: None,
        data_scope: None,
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

/// 写请求构造参数：token / 观测 IP / 用户 / 会话。
/// None 表示该维度缺失（用于覆盖无认证上下文与无 token 分支）。
struct WriteReq {
    token: Option<&'static str>,
    ip: Option<&'static str>,
    user: Option<i32>,
    session: Option<&'static str>,
}

impl WriteReq {
    /// 默认形态：带 token、带 IP、带用户与会话
    fn signed(token: &'static str, ip: &'static str, user: i32, session: &'static str) -> Self {
        Self {
            token: Some(token),
            ip: Some(ip),
            user: Some(user),
            session: Some(session),
        }
    }
}

/// 按生产链路口径构造一个打到业务写端点的 POST 请求
fn build_write_request(spec: WriteReq) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json");
    if let Some(token) = spec.token {
        builder = builder.header("x-csrf-token", token);
    }
    if let Some(ip) = spec.ip {
        builder = builder.header("x-real-ip", ip);
    }
    let mut req = builder.body(Body::from("{}")).expect("构造 POST 请求失败");
    if let Some(user) = spec.user {
        req.extensions_mut().insert(auth_of(user));
    }
    if let Some(session) = spec.session {
        req.extensions_mut().insert(CsrfSession::new(session));
    }
    req
}

/// 签发一枚带完整绑定的 CSRF Token
fn issue_token(state: &AppState, token: &'static str, session: &str, ip: &str, user: i32) {
    state.cache.set_csrf_token(
        token.to_string(),
        session.to_string(),
        ip.to_string(),
        user,
        None,
    );
}

/// 从 Set-Cookie 头里取 csrf_token 的值（多条 Set-Cookie 中挑业务 Cookie）
fn csrf_cookie_of(resp: &axum::response::Response) -> Option<String> {
    resp.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|raw| {
            let pair = raw.split(';').next().unwrap_or("").trim();
            pair.strip_prefix("csrf_token=")
                .map(|s| s.trim().to_string())
        })
}

/// 场景 1：GET 请求无 token 应放行通过
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

/// 场景 2：POST 请求无 X-CSRF-Token 头 即 403 CSRF_TOKEN_MISSING
#[tokio::test]
async fn test_post_without_csrf_header_returns_missing() {
    let state = AppState::default();
    let app = build_test_app(state);

    let req = build_write_request(WriteReq {
        token: None,
        ip: Some(IP_WIFI),
        user: Some(USER_ID),
        session: Some(SESSION),
    });

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "无 X-CSRF-Token 头的 POST 应返回 403"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        code_of(&body),
        Some("CSRF_TOKEN_MISSING"),
        "业务码应为 CSRF_TOKEN_MISSING，实际: {:?}",
        body.get("code")
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some("CSRF Token 缺失"),
        "message 应为常量 CSRF Token 缺失"
    );
    // 统一失败信封（utils/response.rs::unified_error_response 出参 ErrorResponse）：
    // {code, message, trace_id, timestamp}，无废弃的 success 字段。
    assert!(
        body.get("success").is_none(),
        "统一错误信封不得含废弃的 success 字段"
    );
    assert!(
        body.get("trace_id")
            .map(|v| v.is_string() && !v.as_str().unwrap().is_empty())
            .unwrap_or(false),
        "trace_id 应为非空字符串，实际: {:?}",
        body.get("trace_id")
    );
    assert!(
        body.get("timestamp").map(|v| v.is_i64()).unwrap_or(false),
        "timestamp 应为数字（秒级 i64），实际: {:?}",
        body.get("timestamp")
    );
}

/// 场景 3：POST 请求带无效 token 即 403 CSRF_TOKEN_INVALID
#[tokio::test]
async fn test_post_with_invalid_csrf_token_returns_invalid() {
    let state = AppState::default();
    let app = build_test_app(state);

    let req = build_write_request(WriteReq {
        token: Some("this-token-does-not-exist-in-cache"),
        ip: Some(IP_WIFI),
        user: Some(USER_ID),
        session: Some(SESSION),
    });

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "无效 CSRF Token 的 POST 应返回 403"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        code_of(&body),
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

/// 场景 4：POST 请求带有效 token 得 200 通过（且验证 rotation：第二次请求应被拒绝）
#[tokio::test]
async fn test_post_with_valid_csrf_token_passes_then_rotated() {
    let state = AppState::default();

    let valid_token = "valid-csrf-token-for-test-12345";
    issue_token(
        &state,
        valid_token,
        SESSION,
        IP_WIFI,
        USER_ID, // user_id（同时进反向索引，支持强制轮换）
    );

    // 4.1 第一次请求：绑定与 IP 全一致 应通过
    let app = build_test_app(state.clone());
    let req = build_write_request(WriteReq::signed(valid_token, IP_WIFI, USER_ID, SESSION));

    let resp = app.oneshot(req).await.expect("执行第一次请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "有效 CSRF Token + 匹配绑定的 POST 应返回 200"
    );

    // 4.2 第二次请求：同一 token 已被消费（rotation 模式）→ 应返回 403
    let app2 = build_test_app(state.clone());
    let req2 = build_write_request(WriteReq::signed(valid_token, IP_WIFI, USER_ID, SESSION));

    let resp2 = app2.oneshot(req2).await.expect("执行第二次请求失败");
    assert_eq!(
        resp2.status(),
        StatusCode::FORBIDDEN,
        "rotation 模式下第二次使用同一 token 应被拒绝"
    );

    let body = read_json(resp2.into_body()).await;
    assert_eq!(
        code_of(&body),
        Some("CSRF_TOKEN_INVALID"),
        "rotation 后第二次请求业务码应为 CSRF_TOKEN_INVALID"
    );
}

/// 场景 5：公开路径（如 auth/login）POST 无 token 应放行通过
#[tokio::test]
async fn test_public_path_post_passes_without_csrf_token() {
    let state = AppState::default();
    let app = build_test_app(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/auth/login")
        .header("content-type", "application/json")
        .header("x-requested-with", "XMLHttpRequest")
        .body(Body::from("{\"username\":\"admin\",\"password\":\"x\"}"))
        .expect("构造登录请求失败");

    let resp = app.oneshot(req).await.expect("执行登录请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "公开路径（login）的 POST 应被 CSRF 中间件放行"
    );
}

/// 场景 6：HEAD / OPTIONS 方法无 token 应放行通过（无副作用）
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
    cache.set_csrf_token(
        token.to_string(),
        SESSION.to_string(),
        "203.0.113.99".to_string(),
        2002,
        None,
    );

    // 第一次消费：绑定匹配 应成功
    assert_eq!(
        cache.consume_csrf_token(token, "203.0.113.99", Some(2002), Some(SESSION)),
        CsrfConsumeResult::Ok,
        "第一次消费（绑定匹配）应返回 Ok"
    );
    // 第二次消费：应失败（rotation 模式）
    assert_eq!(
        cache.consume_csrf_token(token, "203.0.113.99", Some(2002), Some(SESSION)),
        CsrfConsumeResult::NotFound,
        "第二次消费应返回 NotFound（token 已被移除）"
    );
    // 不存在的 token：应失败
    assert_eq!(
        cache.consume_csrf_token(
            "non-existent-token",
            "203.0.113.99",
            Some(2002),
            Some(SESSION)
        ),
        CsrfConsumeResult::NotFound,
        "不存在的 token 应返回 NotFound"
    );
}

/// 场景 8（本次修复主判据）：同会话同用户下 IP 变化不再拒绝，
/// 请求成功且响应已下发一枚绑定到新 IP 的新 token，旧 token 在宽限窗口内仍可服务在途请求。
#[tokio::test]
async fn test_ip_change_within_session_passes_and_reissues_token() {
    let state = AppState::default();

    let valid_token = "valid-csrf-token-before-network-switch";
    issue_token(&state, valid_token, SESSION, IP_WIFI, USER_ID);

    // 客户端从蜂窝出口发起（绑定值是 Wi-Fi 时期的 IP）
    let app = build_test_app(state.clone());
    let req = build_write_request(WriteReq::signed(valid_token, IP_CELL, USER_ID, SESSION));

    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "同会话同用户下 IP 变化必须放行，改回按 IP 全等拒绝会把移动会话硬断"
    );

    // 8.1 新 token 已随 Set-Cookie 下发，且不等于旧 token
    let new_token = csrf_cookie_of(&resp).expect("漂移放行后必须下发重发的 csrf_token Cookie");
    assert_ne!(
        new_token, valid_token,
        "响应必须携带轮换出的新 token，而不是把旧 token 原样回填"
    );

    // 8.2 新 token 绑定到新 IP：以新 IP 消费成功，以旧 IP 消费会记为再一次漂移（不拒绝）
    assert_eq!(
        state
            .cache
            .consume_csrf_token(&new_token, IP_CELL, Some(USER_ID), Some(SESSION)),
        CsrfConsumeResult::Ok,
        "重发的 token 必须绑定到本次观测到的新 IP 且属于同一会话同一用户"
    );

    // 8.3 旧 token 未被立刻作废：在途并发请求仍带着它，应能完成
    let app2 = build_test_app(state.clone());
    let inflight = build_write_request(WriteReq::signed(valid_token, IP_CELL, USER_ID, SESSION));
    let resp2 = app2.oneshot(inflight).await.expect("执行在途请求失败");
    assert_eq!(
        resp2.status(),
        StatusCode::OK,
        "宽限窗口内的在途请求不得被一次换网打成 403"
    );
}

/// 场景 9a（反向）：token 属于别的用户 必拒，且不得进入自动恢复通道
#[tokio::test]
async fn test_token_of_other_user_is_rejected_without_recovery() {
    let state = AppState::default();

    let token = "token-owned-by-another-user";
    issue_token(&state, token, SESSION, IP_WIFI, OTHER_USER_ID);

    let app = build_test_app(state.clone());
    // 攻击方连 IP 都伪造得一模一样，仍必须因用户绑定不过而被拒
    let req = build_write_request(WriteReq::signed(token, IP_WIFI, USER_ID, SESSION));
    let resp = app.oneshot(req).await.expect("执行请求失败");

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "跨用户消费 CSRF Token 必须 403"
    );
    assert!(
        resp.headers()
            .get("x-new-csrf-token")
            .and_then(|v| v.to_str().ok())
            .is_none(),
        "绑定核验失败属盗用面，不得下发恢复 token（那等于替攻击方自动补一枚可用凭证）"
    );
    assert!(
        csrf_cookie_of(&resp).is_none(),
        "绑定核验失败的响应不得附带 csrf_token Cookie"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        code_of(&body),
        Some("CSRF_TOKEN_BINDING_MISMATCH"),
        "业务码应为 CSRF_TOKEN_BINDING_MISMATCH，实际: {:?}",
        body.get("code")
    );

    // 拒绝不得消耗原主的 token
    assert_eq!(
        state
            .cache
            .consume_csrf_token(token, IP_WIFI, Some(OTHER_USER_ID), Some(SESSION)),
        CsrfConsumeResult::Ok,
        "被拒的攻击请求不得顺带把原主的 token 消费掉"
    );
}

/// 场景 9b（反向）：同用户的另一个会话拿不到授权 必拒
#[tokio::test]
async fn test_token_of_other_session_is_rejected() {
    let state = AppState::default();

    let token = "token-owned-by-another-session";
    issue_token(&state, token, OTHER_SESSION, IP_WIFI, USER_ID);

    let app = build_test_app(state.clone());
    let req = build_write_request(WriteReq::signed(token, IP_WIFI, USER_ID, SESSION));
    let resp = app.oneshot(req).await.expect("执行请求失败");

    let status = resp.status();
    let body = read_json(resp.into_body()).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨会话消费必须 403（IP 一致也不例外）"
    );
    assert_eq!(
        code_of(&body),
        Some("CSRF_TOKEN_BINDING_MISMATCH"),
        "跨会话应归到绑定核验失败码，实际: {:?}",
        body.get("code")
    );
    assert_eq!(
        state
            .cache
            .consume_csrf_token(token, IP_WIFI, Some(USER_ID), Some(OTHER_SESSION)),
        CsrfConsumeResult::Ok,
        "跨会话被拒时原会话的 token 必须仍在"
    );
}

/// 场景 9c（反向）：请求没有认证上下文时不得消费任何业务 token
#[tokio::test]
async fn test_request_without_auth_context_is_rejected() {
    let state = AppState::default();

    let token = "token-requiring-auth-context";
    issue_token(&state, token, SESSION, IP_WIFI, USER_ID);

    let app = build_test_app(state.clone());
    let req = build_write_request(WriteReq {
        token: Some(token),
        ip: Some(IP_WIFI),
        user: None,
        session: None,
    });
    let resp = app.oneshot(req).await.expect("执行请求失败");

    let status = resp.status();
    let body = read_json(resp.into_body()).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "无认证上下文不得消费用户绑定的 CSRF Token"
    );
    assert_eq!(
        code_of(&body),
        Some("CSRF_TOKEN_BINDING_MISMATCH"),
        "无认证上下文归绑定核验失败，实际: {:?}",
        body.get("code")
    );
}

/// 场景 10（反向）：伪造 X-Forwarded-For / X-Real-For 不能作为绕过会话绑定的手段。
/// 攻击方把 X-Real-IP 伪造成与绑定值逐字相同，会话不符依然被拒。
#[tokio::test]
async fn test_forged_forwarded_ip_cannot_bypass_session_binding() {
    let state = AppState::default();

    let token = "token-bound-to-known-ip";
    issue_token(&state, token, OTHER_SESSION, IP_WIFI, USER_ID);

    let mut req = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json")
        .header("x-csrf-token", token)
        // 伪造到与绑定 IP 完全一致
        .header("x-real-ip", IP_WIFI)
        .header("x-forwarded-for", format!("{IP_WIFI}, 10.0.0.1"))
        .body(Body::from("{}"))
        .expect("构造伪造请求失败");
    req.extensions_mut().insert(auth_of(USER_ID));
    req.extensions_mut().insert(CsrfSession::new(SESSION));

    let app = build_test_app(state.clone());
    let resp = app.oneshot(req).await.expect("执行请求失败");
    let status = resp.status();
    let restored_token = resp.headers().get("x-new-csrf-token").cloned();
    let body = read_json(resp.into_body()).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "IP 逐字相等不得被当成会话归属证明"
    );
    assert_eq!(
        code_of(&body),
        Some("CSRF_TOKEN_BINDING_MISMATCH"),
        "伪造 IP 后的拒绝必须仍走绑定核验码，实际: {:?}",
        body.get("code")
    );
    assert!(
        restored_token.is_none(),
        "伪造来源不得换来一枚可用的恢复 token"
    );
}

/// 场景 11（正向，回归保护）：NotFound 下发的恢复 token 可被同会话同用户直接消费。
/// 绑定核验上线后，恢复通道若仍用占位身份签发就会把所有重放打死，这条是防那个回退的锁。
#[tokio::test]
async fn test_recovery_token_is_consumable_by_same_session_and_user() {
    let state = AppState::default();
    let app = build_test_app(state.clone());

    // 先拿一次 NotFound，取回恢复 token
    let req = build_write_request(WriteReq {
        token: Some("expired-or-already-consumed-token"),
        ip: Some(IP_WIFI),
        user: Some(USER_ID),
        session: Some(SESSION),
    });
    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "无效 token 应 403");
    let recovery = resp
        .headers()
        .get("x-new-csrf-token")
        .and_then(|v| v.to_str().ok())
        .expect("NotFound 必须下发恢复 token")
        .to_string();

    // 前端恢复重放：携恢复 token 再打一次，应放行
    let app2 = build_test_app(state.clone());
    let mut replay = Request::builder()
        .method("POST")
        .uri("/api/v1/erp/business/create")
        .header("content-type", "application/json")
        .header("x-csrf-token", recovery.as_str())
        .header("x-real-ip", IP_WIFI)
        .body(Body::from("{}"))
        .expect("构造重放请求失败");
    replay.extensions_mut().insert(auth_of(USER_ID));
    replay.extensions_mut().insert(CsrfSession::new(SESSION));

    let resp2 = app2.oneshot(replay).await.expect("执行重放请求失败");
    assert_eq!(
        resp2.status(),
        StatusCode::OK,
        "恢复 token 必须能被同一会话同一用户消费，否则绑定核验会把并发恢复通道打死"
    );
}

/// 场景 12：TTL 过期不回退为放行，仍是 403 CSRF_TOKEN_INVALID（并附恢复通道）
#[tokio::test]
async fn test_expired_token_still_rejected_as_invalid() {
    let state = AppState::default();
    let expired = "token-already-expired";
    state.cache.set_csrf_token(
        expired.to_string(),
        SESSION.to_string(),
        IP_WIFI.to_string(),
        USER_ID,
        Some(std::time::Duration::from_millis(10)),
    );
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    let app = build_test_app(state.clone());
    let req = build_write_request(WriteReq {
        token: Some(expired),
        ip: Some(IP_WIFI),
        user: Some(USER_ID),
        session: Some(SESSION),
    });
    let resp = app.oneshot(req).await.expect("执行请求失败");
    let status = resp.status();
    let body = read_json(resp.into_body()).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "过期 token 不得因漂移逻辑被救活"
    );
    assert_eq!(
        code_of(&body),
        Some("CSRF_TOKEN_INVALID"),
        "过期应归到 CSRF_TOKEN_INVALID，实际: {:?}",
        body.get("code")
    );
}

/// 场景 13（既有强制轮换行为）：clear_old_csrf_token_for_user 后旧 token 立即失效
#[tokio::test]
async fn test_clear_old_csrf_token_for_user_invalidates_token() {
    let state = AppState::default();
    let cache = state.cache.clone();

    let old_token = "old-csrf-token-before-rotation";
    issue_token(&state, old_token, SESSION, IP_WIFI, 1004);

    let rotated = cache.clear_old_csrf_token_for_user(1004);
    assert!(rotated, "应返回 true（存在旧 token）");

    let app = build_test_app(state);
    let req = build_write_request(WriteReq::signed(old_token, IP_WIFI, 1004, SESSION));
    let resp = app.oneshot(req).await.expect("执行请求失败");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "被强制轮换的旧 token 应返回 403"
    );

    let body = read_json(resp.into_body()).await;
    assert_eq!(
        code_of(&body),
        Some("CSRF_TOKEN_INVALID"),
        "强制轮换后旧 token 业务码应为 CSRF_TOKEN_INVALID"
    );
}

/// 场景 14（判定器自证，源码形态锁）：中间件不得把 IP 差异当拒绝条件，
/// 且漂移处置必须同时包含"留痕"与"重发"两半，只 log 不处置或只处置不留痕都判红。
/// 与 frontend/scripts/check-csrf-ip-drift.mjs 静态锁同源，这里的价值是把锁钉在被测 crate 的编译单元里：
/// 任何把 IP 改回拒绝条件的改动会在 cargo test 阶段就失败，而不是等静态判定器跑动。
#[test]
fn test_middleware_source_keeps_drift_as_signal_and_binding_as_rejection() {
    let src = include_str!("../src/middleware/csrf.rs");
    assert!(
        !src.contains("IpMismatch") && !src.contains("CSRF_IP_MISMATCH"),
        "csrf.rs 出现以 IP 差异拒绝的形态：移动与 NAT 客户端会重新硬故障"
    );
    assert!(
        src.contains("record_ip_drift_audit"),
        "漂移必须写审计留痕，不许静默"
    );
    assert!(
        src.contains("omni_audit.log"),
        "漂移留痕必须走既有综合审计通道 omni_audit"
    );
    assert!(
        src.contains("rotate_csrf_after_consume"),
        "漂移必须原子重发 token，不许只记日志不处置"
    );
    assert!(
        src.contains("CODE_BIND"),
        "csrf.rs 必须保留绑定维度的拒绝通道"
    );
    assert!(
        !src.contains("CODE_IP_MM"),
        "CODE_IP_MM 是被退役的拒绝码，重新出现即判红"
    );

    let cache_src = include_str!("../src/utils/cache.rs");
    assert!(
        cache_src.contains("install_csrf_token"),
        "宽限副本必须经统一安装路径登记用户索引，否则强制轮换清不掉"
    );
    assert!(
        !cache_src.contains("let _ = session_id"),
        "会话标识必须参与核验，不许再被丢弃"
    );
}
