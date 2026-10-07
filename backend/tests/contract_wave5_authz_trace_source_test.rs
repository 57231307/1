//! 契约测试（wave5）：鉴权 401 / 权限 403 的 trace_id 必须与 `X-Trace-Id` 响应头同源
//!
//! 对应复审结论（`ed5071bb` 未修透的部分）：
//! `utils/response.rs::unified_error_response` 自己 `Uuid::new_v4().to_string()` 现造号码，
//! 而 401/403 全部经它出参（`middleware/auth.rs`、`permission.rs`、`csrf.rs`、`init_token.rs`），
//! `middleware/auth_context.rs` 的 `AuthRejection` 同样现造 ⇒ 这些响应体的 `trace_id`
//! 与 `X-Trace-Id` 响应头（由最外层 `trace_context` 用 task-local 值写入）**并不同源**，
//! 用户按屏幕上的号码查不到任何日志；且现造值带 `-`，与全站统一的 32 位小写 hex 是两种形态。
//!
//! 本文件只驱动真实中间件（不 mock 出参构造函数），逐条锁定：
//! 1. 真实 `auth_middleware` 的 401（缺凭据 / 认证头格式非法 / 令牌为空）；
//! 2. 真实 `permission_middleware` 的 401（缺认证上下文）与 403（无角色 / 权限不足）；
//! 3. `AuthContext` 提取器（`AuthRejection::into_response`）的 401；
//! 4. `csrf` 中间件 403 出参（`csrf::csrf_error_response`）；
//! 5. 带连字符回归锁：出参 trace_id 恒为 32 位小写 hex、不含 `-`、且等于请求 traceparent 的 trace。
//!
//! message 文案与脱敏策略不在本文件射程内（权限文案永久脱敏是既有设计，本波次只换 trace_id 取源）。

use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::{Next, from_fn, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::middleware::auth::auth_middleware;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::csrf::csrf_error_response;
use bingxi_backend::middleware::permission::permission_middleware;
use bingxi_backend::middleware::trace_context::{X_TRACE_ID_HEADER, trace_context_middleware};
use serde_json::Value;
use tower::ServiceExt; // oneshot

/// W3C traceparent 里的固定 trace id（32 位小写 hex）：证明「同源」不是巧合，
/// 而是「响应体 trace_id == trace_context 绑定的请求 trace」。
const REQUEST_TRACE_ID: &str = "0af7651916cd43dd8448eb211c80319c";

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

/// 带 traceparent 的请求（trace_context 据此绑定 TRACE_ID task-local）
fn traceparent_request(method: &str, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(
            "traceparent",
            format!("00-{}-b7ad6b7169203331-01", REQUEST_TRACE_ID),
        )
        .body(Body::empty())
        .expect("测试夹具：请求构造失败")
}

async fn read_json(resp: Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .expect("失败信封应可读出 body");
    serde_json::from_slice(&bytes).expect("失败信封应是合法 JSON")
}

/// Body 不可 Clone：所有同源断言都遵循「先取头、再消费体」的顺序
fn trace_header(resp: &Response) -> String {
    resp.headers()
        .get(X_TRACE_ID_HEADER)
        .unwrap_or_else(|| panic!("鉴权失败响应应带 {} 响应头", X_TRACE_ID_HEADER))
        .to_str()
        .expect("X-Trace-Id 应是合法 ASCII")
        .to_string()
}

/// 形态锁（含带连字符回归锁）：32 位小写 hex，绝不出现 `Uuid::new_v4().to_string()` 的 `-` 形态
fn assert_simple_hex_form(body_trace: &str, scene: &str) {
    assert_eq!(
        body_trace.len(),
        32,
        "{scene}：trace_id 必须是 32 位小写 hex（Uuid::simple 形态），实际: {body_trace:?}"
    );
    assert!(
        !body_trace.contains('-'),
        "{scene}：trace_id 不得出现带连字符的另一种形态（回归锁：曾为 Uuid::new_v4().to_string()），实际: {body_trace:?}"
    );
    assert!(
        body_trace
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "{scene}：trace_id 应全为小写 hex 字符，实际: {body_trace:?}"
    );
}

/// 核心同源断言：响应体 `trace_id` == 响应头 `X-Trace-Id` == 本次请求 trace，且形态收口
fn assert_same_source(hdr: &str, json: &Value, scene: &str) {
    let body_trace = json["trace_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{scene}：失败信封的 trace_id 应是字符串，实际: {json}"));
    assert_eq!(
        body_trace, hdr,
        "{scene}：响应体 trace_id 与响应头 X-Trace-Id 必须逐字符相同（同源），\
         实际体={body_trace:?} 头={hdr:?}"
    );
    assert_eq!(
        body_trace, REQUEST_TRACE_ID,
        "{scene}：响应体 trace_id 必须是被绑定的请求 trace（traceparent），不能是现造 UUID"
    );
    assert_simple_hex_form(body_trace, scene);
}

/// 失败信封固定四键（全站唯一形状，不允许因本波次改动而漂移）
fn assert_envelope_keys(json: &Value) {
    let mut keys: Vec<&String> = json
        .as_object()
        .expect("失败信封应是 JSON 对象")
        .keys()
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["code", "message", "timestamp", "trace_id"],
        "失败信封固定四键，实际: {keys:?}"
    );
    assert!(json["timestamp"].is_i64(), "timestamp 应是 i64: {json}");
    assert!(
        !json["code"].as_str().unwrap_or("").is_empty(),
        "code 不能为空: {json}"
    );
    assert!(
        !json["message"].as_str().unwrap_or("").is_empty(),
        "message 不能为空: {json}"
    );
}

/// 与生产一致的层序：trace_context 在最外层（绑定 TRACE_ID），内层是待测鉴权中间件。
fn app_with_trace(inner: Router) -> Router {
    inner.layer(from_fn(trace_context_middleware))
}

async fn ok_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"ok": true})))
}

/// AuthContext 注入中间件（模拟 auth 通过后写入 extensions，供 permission 中间件读取；
/// 被测对象仍是真实的 permission_middleware）
async fn inject_auth(
    axum::extract::State(auth): axum::extract::State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

fn make_auth(role_id: Option<i32>) -> AuthContext {
    AuthContext {
        user_id: 4242,
        username: "wave5_trace_probe".to_string(),
        role_id,
        department_id: None,
        data_scope: None,
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

/// 真实 auth_middleware 的测试 Router
fn auth_app(state: AppState) -> Router {
    app_with_trace(
        Router::new()
            .route("/api/v1/erp/users", get(ok_handler))
            .layer(from_fn_with_state(state, auth_middleware)),
    )
}

/// 真实 permission_middleware 的测试 Router（auth 由 inject_auth 模拟注入，
/// 与 tests/test_permission_rbac.rs 的既有做法一致）
fn permission_app(state: AppState, auth: AuthContext) -> Router {
    app_with_trace(
        Router::new()
            .route("/api/v1/erp/users", get(ok_handler))
            .layer(from_fn_with_state(state, permission_middleware))
            .layer(from_fn_with_state(auth, inject_auth)),
    )
}

fn with_authorization(mut req: Request<Body>, value: &str) -> Request<Body> {
    // from_static 要求 'static，本夹具入参是调用栈上的 &str；from_str 做同样的合法性校验
    let hv = HeaderValue::from_str(value).expect("测试夹具：Authorization 头值应合法");
    req.headers_mut().insert(header::AUTHORIZATION, hv);
    req
}

// ---------------------------------------------------------------------------
// ① 真实 auth_middleware 的 401
// ---------------------------------------------------------------------------

/// 缺认证凭据（无 Authorization 头、无 Cookie）→ 401，响应体 trace_id 与响应头同源
#[tokio::test]
async fn auth_middleware_401_missing_credentials_is_trace_scoped() {
    let app = auth_app(AppState::default());

    let resp = app
        .oneshot(traceparent_request("GET", "/api/v1/erp/users"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "缺凭据必须 401");
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(
        json["code"], "UNAUTHORIZED",
        "code 取值不得因本次改动而漂移"
    );
    assert_same_source(&hdr, &json, "auth 401（缺认证凭据）");
}

/// Authorization 头格式非法（非 `Bearer ` 前缀）→ 401
#[tokio::test]
async fn auth_middleware_401_invalid_header_format_is_trace_scoped() {
    let app = auth_app(AppState::default());

    let req = with_authorization(
        traceparent_request("GET", "/api/v1/erp/users"),
        "Token not-a-bearer-token",
    );
    let resp = app.oneshot(req).await.expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "UNAUTHORIZED");
    assert_same_source(&hdr, &json, "auth 401（认证头格式非法）");
}

/// `Bearer ` 后为空串 → 401（令牌为空分支）
#[tokio::test]
async fn auth_middleware_401_empty_token_is_trace_scoped() {
    let app = auth_app(AppState::default());

    let req = with_authorization(traceparent_request("GET", "/api/v1/erp/users"), "Bearer ");
    let resp = app.oneshot(req).await.expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_eq!(json["code"], "UNAUTHORIZED");
    assert_same_source(&hdr, &json, "auth 401（令牌为空）");
}

/// 非法签名/乱码令牌 → 走 `unauthorized_response("无效的认证令牌")`（auth.rs 末段分支）。
/// 该分支需要进入 JWT 校验后再失败，这里用明显非法的 token 触发。
#[tokio::test]
async fn auth_middleware_401_invalid_token_is_trace_scoped() {
    let app = auth_app(AppState::default());

    let req = with_authorization(
        traceparent_request("GET", "/api/v1/erp/users"),
        "Bearer not.a.jwt.at.all",
    );
    let resp = app.oneshot(req).await.expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_eq!(json["code"], "UNAUTHORIZED");
    assert_same_source(&hdr, &json, "auth 401（令牌无效）");
}

// ---------------------------------------------------------------------------
// ② 真实 permission_middleware 的 401 / 403
// ---------------------------------------------------------------------------

/// 缺认证上下文 → 401
#[tokio::test]
async fn permission_middleware_401_without_auth_context_is_trace_scoped() {
    let app = app_with_trace(
        Router::new()
            .route("/api/v1/erp/users", get(ok_handler))
            .layer(from_fn_with_state(
                AppState::default(),
                permission_middleware,
            )),
    );

    let resp = app
        .oneshot(traceparent_request("GET", "/api/v1/erp/users"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "UNAUTHORIZED");
    assert_same_source(&hdr, &json, "permission 401（缺认证上下文）");
}

/// 有认证上下文但未关联角色 → 403
#[tokio::test]
async fn permission_middleware_403_without_role_is_trace_scoped() {
    let app = permission_app(AppState::default(), make_auth(None));

    let resp = app
        .oneshot(traceparent_request("GET", "/api/v1/erp/users"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "无角色必须 403");
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "FORBIDDEN");
    assert_same_source(&hdr, &json, "permission 403（未关联角色）");
}

/// 已认证 + 有角色，但 mock DB 权限集为空 → 权限不足 403（一线最常见的报障场景）
#[tokio::test]
async fn permission_middleware_403_denied_is_trace_scoped() {
    let app = permission_app(AppState::default(), make_auth(Some(7)));

    let resp = app
        .oneshot(traceparent_request("GET", "/api/v1/erp/users"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "mock DB 无权限记录应 403"
    );
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_eq!(json["code"], "FORBIDDEN");
    assert_same_source(&hdr, &json, "permission 403（权限不足）");
}

// ---------------------------------------------------------------------------
// ③ AuthContext 提取器（AuthRejection::into_response，middleware/auth_context.rs）
// ---------------------------------------------------------------------------

async fn extractor_handler(_auth: AuthContext) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"ok": true})))
}

/// 未经 auth 中间件注入时，`AuthContext` 提取器拒绝产生的 401 也必须同源
#[tokio::test]
async fn auth_context_extractor_401_is_trace_scoped() {
    let app = app_with_trace(Router::new().route("/api/v1/erp/users", get(extractor_handler)));

    let resp = app
        .oneshot(traceparent_request("GET", "/api/v1/erp/users"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "UNAUTHORIZED");
    assert_same_source(&hdr, &json, "AuthRejection 401（提取器拒绝）");
}

// ---------------------------------------------------------------------------
// ④ CSRF 403 出参（middleware/csrf.rs::csrf_error_response）
// ---------------------------------------------------------------------------

async fn csrf_error_handler() -> Response {
    csrf_error_response("CSRF_TOKEN_INVALID", "CSRF 校验失败，请刷新页面后重试")
}

#[tokio::test]
async fn csrf_403_response_is_trace_scoped() {
    let app =
        app_with_trace(Router::new().route("/api/v1/erp/sales-orders", get(csrf_error_handler)));

    let resp = app
        .oneshot(traceparent_request("GET", "/api/v1/erp/sales-orders"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "CSRF_TOKEN_INVALID");
    assert_same_source(&hdr, &json, "csrf 403");
}

// ---------------------------------------------------------------------------
// ⑤ 交叉锁：同一请求 trace 下两条鉴权链路必须给出同一个号码
// ---------------------------------------------------------------------------

#[tokio::test]
async fn authz_401_and_403_share_the_same_trace_within_one_request_scope() {
    let state = AppState::default();
    let auth_app = auth_app(state.clone());
    let perm_app = permission_app(state, make_auth(None));

    let auth_resp = auth_app
        .oneshot(traceparent_request("GET", "/api/v1/erp/users"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(auth_resp.status(), StatusCode::UNAUTHORIZED);
    let auth_hdr = trace_header(&auth_resp);
    let auth_json = read_json(auth_resp).await;
    assert_same_source(&auth_hdr, &auth_json, "auth 401");

    let perm_resp = perm_app
        .oneshot(traceparent_request("GET", "/api/v1/erp/users"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(perm_resp.status(), StatusCode::FORBIDDEN);
    let perm_hdr = trace_header(&perm_resp);
    let perm_json = read_json(perm_resp).await;
    assert_same_source(&perm_hdr, &perm_json, "permission 403");

    assert_eq!(
        auth_json["trace_id"], perm_json["trace_id"],
        "同一请求 trace 下两条鉴权链路的出参号码必须一致（都应是 {REQUEST_TRACE_ID}）"
    );
    assert_eq!(auth_hdr, perm_hdr);
}

/// 上游给出**非规范** traceparent（trace_id 用带 `-` 的 UUID 写法 ⇒ 整串被拆成 5 段）时，
/// 依据 R-8 采 W3C 严格丢弃：不得进入本次请求的 trace_id，出参必须是系统自生成的
/// 32 位小写 hex，且响应头与响应体同源。
///
/// 为什么不"宽容归一"（原判责此处要求归一，已被推翻）：trace_id 是可被外部注入的关联键，
/// 把非规范值洗成合法形态等于接受外部可控 id 进入日志/审计链，制造跨请求碰撞与污染面；
/// 本仓对"外部输入不信任"是一贯口径（源码现状 `from_traceparent` 段数≠4 即 None、
/// 落到 `new_root()` 自生成，见 observability/trace_context.rs:58-60、:136-145）。
#[tokio::test]
async fn hyphenated_upstream_traceparent_is_discarded_not_normalized() {
    use bingxi_backend::observability::trace_context::TraceContext;

    let upstream = "0af76519-16cd-43dd-8448-eb211c80319c"; // 带连字符的 128-bit 写法
    let canonical = upstream.replace('-', "");
    let header_value = format!("00-{upstream}-b7ad6b7169203331-01");

    // 判据①（函数层负向锁）：非规范值必须被丢弃，而不是被"洗"成可用 id
    assert!(
        TraceContext::from_traceparent(&header_value).is_none(),
        "带连字符 trace_id 使整串成 5 段，W3C 严格口径下必须解析失败"
    );

    let app = auth_app(AppState::default());
    let mut req = traceparent_request("GET", "/api/v1/erp/users");
    req.headers_mut().insert(
        "traceparent",
        HeaderValue::from_str(&header_value).expect("测试夹具：header 值应合法"),
    );

    let resp = app.oneshot(req).await.expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;

    // 判据②（不外泄上游可控 id）：原样串与去连字符形态都不得成为出参 trace_id
    assert_ne!(hdr, upstream, "响应头 trace_id 不得是上游非规范原样串");
    assert_ne!(
        hdr, canonical,
        "响应头 trace_id 不得是上游 id 的规范化形态（那是注入面）"
    );
    assert_ne!(
        json["trace_id"].as_str().unwrap_or_default(),
        canonical,
        "响应体 trace_id 不得采纳上游非规范 traceparent 的 id"
    );

    // 判据③（自生成 + 两侧同源 + 形态合法）
    let body_trace = json["trace_id"]
        .as_str()
        .unwrap_or_else(|| panic!("失败信封的 trace_id 应是字符串，实际: {json}"));
    assert_eq!(
        body_trace, hdr,
        "丢弃上游值后，响应体与响应头仍必须同源（都是本次自生成的 trace）"
    );
    assert_simple_hex_form(body_trace, "auth 401（非规范 traceparent 被丢弃后自生成）");
    assert_ne!(
        body_trace, REQUEST_TRACE_ID,
        "非规范上游值被丢弃 ⇒ 出参绝不等于夹具里那条规范 traceparent 的 id，也不应复用上游值"
    );
}

// ---------------------------------------------------------------------------
// ⑥ 源码扫描禁回潮锁（与 tests/contract_wave5_reference_precheck_and_family_test.rs
//    的源码扫描锁同一手法）：出参 trace_id 只允许一个取源＝utils/error.rs::current_trace_id
// ---------------------------------------------------------------------------

/// 读取被测源码文本（Windows 工作区是 CRLF，统一压成 LF 再匹配）
fn src_text(rel: &str) -> String {
    let text = match rel {
        "utils/response.rs" => include_str!("../src/utils/response.rs"),
        "middleware/auth.rs" => include_str!("../src/middleware/auth.rs"),
        "middleware/permission.rs" => include_str!("../src/middleware/permission.rs"),
        "middleware/auth_context.rs" => include_str!("../src/middleware/auth_context.rs"),
        "middleware/csrf.rs" => include_str!("../src/middleware/csrf.rs"),
        "middleware/init_token.rs" => include_str!("../src/middleware/init_token.rs"),
        "middleware/omni_audit.rs" => include_str!("../src/middleware/omni_audit.rs"),
        "handlers/omni_audit_handler.rs" => include_str!("../src/handlers/omni_audit_handler.rs"),
        other => panic!("未登记的源码扫描路径: {other}"),
    };
    text.replace('\r', "")
}

/// 鉴权出参链路上的每个文件都不得在 trace_id/失败信封构造处现造 UUID。
/// （csrf.rs 里的 `Uuid::new_v4` 是 CSRF token 本体，不是 trace_id，按整行关键字判定放行）
#[test]
fn authz_error_paths_must_not_self_generate_trace_id() {
    for rel in [
        "utils/response.rs",
        "middleware/auth.rs",
        "middleware/permission.rs",
        "middleware/auth_context.rs",
        "middleware/csrf.rs",
        "middleware/init_token.rs",
    ] {
        let text = src_text(rel);
        let offending = text.lines().any(|line| {
            line.contains("Uuid::new_v4()")
                && (line.contains("trace_id") || line.contains("ErrorResponse"))
        });
        assert!(
            !offending,
            "{rel} 不得在 trace_id/失败信封构造处现造 UUID（应读 current_trace_id），命中行:\n{}",
            text.lines()
                .filter(|l| l.contains("Uuid::new_v4()"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// `unified_error_response` 必须经 `current_trace_id()` 取源（全站唯一取源点）
#[test]
fn unified_error_response_reads_scoped_trace_id() {
    let text = src_text("utils/response.rs");
    // 取到下一个顶层条目为止，断言范围严格限定在本函数体内
    let body = text
        .split_once("pub fn unified_error_response")
        .expect("utils/response.rs 应有 unified_error_response")
        .1
        .split_once("\npub fn ")
        .map(|(b, _)| b)
        .unwrap_or("")
        .to_string();
    assert!(
        body.contains("current_trace_id()"),
        "unified_error_response 必须读 utils/error.rs 的 current_trace_id()，实际函数体:\n{body}"
    );
    assert!(
        !body.contains("Uuid::new_v4"),
        "unified_error_response 内不得再现造 UUID，实际函数体:\n{body}"
    );
}

/// 审计落库侧（旁路现造点）同样必须同源
#[test]
fn audit_trace_id_uses_scoped_source() {
    for rel in ["middleware/omni_audit.rs", "handlers/omni_audit_handler.rs"] {
        let text = src_text(rel);
        assert!(
            text.contains("current_trace_id()"),
            "{rel} 的 trace_id 必须取 current_trace_id()（与 X-Trace-Id 响应头同源）"
        );
        assert!(
            !text.contains("trace_id = uuid::Uuid::new_v4")
                && !text.contains("trace_id = Uuid::new_v4"),
            "{rel} 不得再现造 trace_id"
        );
    }
}
