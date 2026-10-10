//! Wave-H 契约测（CI H 族）：提取器拒绝必须走统一 `AppError` 失败信封
//!
//! 锁定口径（收口链路见 `middleware/trace_context.rs::catch_panic_middleware` 与
//! `utils/error.rs::into_response`，各条由本文件断言自证）：
//! - `Json<T>` 解码失败（缺必填键 / 字段类型错 / JSON 语法错）：axum 默认 **422/400
//!   纯文本、无机器码** → 必须被 `middleware::trace_context` 的 catch_panic 响应侧
//!   收口归一为 **HTTP 400 + code=VALIDATION_ERROR** + 固定公开规则文案；
//! - `Query<T>` 解码失败（缺参 / 类型错）：默认 400 纯文本 → 同上，与 Json 族统一；
//! - 出参**只含** `code/message/trace_id/timestamp` 四键（trace_id/timestamp 与既有
//!   AppError 同源：`utils/error.rs` 的 `into_response` 唯一构造路径）；
//! - serde 原文（`invalid type`、`missing field`、结构体名、内部字段标识、行列号）
//!   **严禁**进入 HTTP 出参（只进 tracing::warn，由映射点负责）；
//! - 鉴权面隔离回归锁：401 / 403 出参 message 必须仍是脱敏常量
//!   （`err_msg::UNAUTHORIZED_PUBLIC` / `PERMISSION_PUBLIC`），本轮改动不得外溢。
//!
//! 纯 axum Router + tower `oneshot`，不需要 DB。

use axum::{
    Json, Router,
    body::Body,
    extract::Query,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bingxi_backend::middleware::trace_context::catch_panic_middleware;
use bingxi_backend::utils::error::{AppError, REQUEST_DECODING_PUBLIC};
use bingxi_backend::utils::messages::err_msg;
use serde::Deserialize;
use serde_json::Value;
use tower::ServiceExt;

// ============================================================================
// 探针 DTO / handler（仅测试用最小形状；字段名刻意与触发用例同族特征词对齐，
// 以便泄露断言能钉到 serde 原文的真实形态）
// ============================================================================

#[derive(Deserialize)]
#[allow(dead_code)]
struct ProbeHazardDto {
    hazard_name: String,
    threshold_value: i64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ProbeMonitorQuery {
    monitor_type: String,
    total_orders: i64,
}

async fn probe_json(Json(_dto): Json<ProbeHazardDto>) -> &'static str {
    "ok"
}

async fn probe_query(Query(_q): Query<ProbeMonitorQuery>) -> &'static str {
    "ok"
}

async fn probe_ok() -> &'static str {
    "ok"
}

/// 模拟鉴权拒绝面：401 走 AppError 信封（JSON），detail 为不可外显的内部原因
async fn gate_401(_req: Request<Body>, _next: Next) -> Response {
    AppError::unauthorized("内部原因：token 已过期，签发于 2026-01-01T00:00:00Z").into_response()
}

/// 模拟权限拒绝面：403 走 AppError 信封（JSON），detail 为不可外显的内部原因
async fn gate_403(_req: Request<Body>, _next: Next) -> Response {
    AppError::permission_denied("内部原因：缺少权限键 crm/cross_owner_write，白名单段=privacy")
        .into_response()
}

/// 与生产洋葱同款的最内两段之一：catch_panic 响应侧收口（trace_context 不在本测试
/// Router 内挂载，信封 trace_id 走 `current_trace_id` 的显式 WARN 回退路径——
/// 仍是同一 `into_response` 构造，四键与同源逻辑不变）。
fn build_app() -> Router {
    Router::new()
        .route("/probe/json", post(probe_json))
        .route("/probe/query", get(probe_query))
        .route("/probe/ok", get(probe_ok))
        .route(
            "/probe/401",
            get(probe_ok).layer(middleware::from_fn(gate_401)),
        )
        .route(
            "/probe/403",
            get(probe_ok).layer(middleware::from_fn(gate_403)),
        )
        .layer(middleware::from_fn(catch_panic_middleware))
}

async fn read_json(resp: Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .expect("失败响应体应可读取");
    serde_json::from_slice(&bytes).expect("失败信封必须是合法 JSON（失败形状唯一）")
}

/// serde 原文 / 内部标识特征串：绝不允许出现在可外显 message 里
const SERDE_LEAK_MARKERS: &[&str] = &[
    "invalid type",
    "missing field",
    "invalid value",
    "at line",
    "column",
    "ProbeHazardDto",
    "hazard_name",
    "threshold_value",
    "monitor_type",
    "total_orders",
    "Failed to deserialize",
    "Expected request with",
];

/// 统一校验：400 + VALIDATION_ERROR + 恰四键 + 非空公开文案 + 零 serde 泄露 + trace/timestamp 齐全
fn assert_validation_envelope(status: StatusCode, v: &Value) {
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "提取器拒绝必须归一为 400（422→400 本轮目标），实际 {status}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "实际信封: {v}");
    assert_eq!(v["message"], REQUEST_DECODING_PUBLIC, "实际信封: {v}");
    let msg = v["message"].as_str().expect("message 应是字符串");
    assert!(!msg.is_empty(), "公开规则文案不得为空");
    for marker in SERDE_LEAK_MARKERS {
        assert!(
            !msg.contains(marker),
            "出参 message 不得含 serde 原文特征串 {marker:?}，实际: {msg}"
        );
    }
    let mut keys: Vec<&str> = v
        .as_object()
        .expect("信封应是 JSON 对象")
        .keys()
        .map(|k| k.as_str())
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["code", "message", "timestamp", "trace_id"],
        "失败信封必须恰好是四键唯一形状，实际键: {keys:?}"
    );
    let trace = v["trace_id"].as_str().expect("trace_id 应存在且为字符串");
    assert_eq!(
        trace.len(),
        32,
        "trace_id 应为 32 位小写 hex（simple 形态），实际: {trace}"
    );
    assert!(
        v["timestamp"].is_i64() && v["timestamp"].as_i64().unwrap() > 0,
        "timestamp 应为正的 unix 秒，实际: {}",
        v["timestamp"]
    );
}

async fn post_json(app: Router, body: &'static str) -> Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/probe/json")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap(),
    )
    .await
    .unwrap()
}

// ============================================================================
// 锁①：Json 提取器解码失败 → 400 + VALIDATION_ERROR + 四键 + 零 serde 泄露
// ============================================================================

/// 缺全部必填键（69 spec 复刻形状：UI 提交空对象）→ 修复前 422 纯文本
#[tokio::test]
async fn json_missing_required_keys_gets_envelope() {
    let resp = post_json(build_app(), "{}").await;
    assert!(
        resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.starts_with("application/json")),
        "失败信封必须是 application/json，不得再是 text/plain"
    );
    let status = resp.status();
    let v = read_json(resp).await;
    assert_validation_envelope(status, &v);
}

/// 字段类型错（threshold_value 传 string；对应 `total_orders 无法解码为整数` 脆断通道）
#[tokio::test]
async fn json_wrong_field_type_gets_envelope() {
    let resp = post_json(
        build_app(),
        r#"{"hazard_name":"粉尘","threshold_value":"abc"}"#,
    )
    .await;
    let status = resp.status();
    let v = read_json(resp).await;
    assert_validation_envelope(status, &v);
}

/// JSON 语法错（JsonSyntaxError 400 纯文本同族通道）
#[tokio::test]
async fn json_syntax_error_gets_envelope() {
    let resp = post_json(build_app(), "{\"hazard_name\": ").await;
    let status = resp.status();
    let v = read_json(resp).await;
    assert_validation_envelope(status, &v);
}

/// 缺 content-type（MissingJsonContentType 415 纯文本同族通道）
#[tokio::test]
async fn json_missing_content_type_gets_envelope() {
    let resp = build_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/probe/json")
                .body(Body::from(r#"{"hazard_name":"粉尘","threshold_value":1}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let v = read_json(resp).await;
    assert_validation_envelope(status, &v);
}

// ============================================================================
// 锁②：Query 提取器解码失败 → 与 Json 族统一 400 + VALIDATION_ERROR + 四键
// ============================================================================

async fn get_query(app: Router, uri: &'static str) -> Response {
    app.oneshot(
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

/// 非法枚举/类型值（60 spec 复刻通道：值无法解码为 i64）→ 修复前 400 纯文本无 code
#[tokio::test]
async fn query_illegal_value_gets_envelope() {
    let resp = get_query(
        build_app(),
        "/probe/query?monitor_type=exhaust_gas&total_orders=abc",
    )
    .await;
    let status = resp.status();
    let v = read_json(resp).await;
    assert_validation_envelope(status, &v);
}

/// 缺全部必填 query 参数（第二节 400 族全部走的就是这条通道）
#[tokio::test]
async fn query_missing_required_params_gets_envelope() {
    let resp = get_query(build_app(), "/probe/query").await;
    let status = resp.status();
    let v = read_json(resp).await;
    assert_validation_envelope(status, &v);
}

// ============================================================================
// 锁③：鉴权拒绝面零外溢 + 正常路径零干扰
// ============================================================================

/// 401 出参仍是脱敏常量（内部原因不外泄、形状不因本轮改动漂移）
#[tokio::test]
async fn unauthorized_stays_sanitized_constant() {
    let resp = get_query(build_app(), "/probe/401").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let v = read_json(resp).await;
    assert_eq!(v["code"], "UNAUTHORIZED");
    assert_eq!(
        v["message"],
        err_msg::UNAUTHORIZED_PUBLIC,
        "401 message 必须仍是脱敏常量，本轮 H 族改动不得触碰鉴权面"
    );
    assert!(
        !v["message"].as_str().unwrap().contains("token"),
        "内部原因严禁外显: {}",
        v["message"]
    );
}

/// 403 出参仍是脱敏常量（同上，含白名单/权限键等判定依据不外泄）
#[tokio::test]
async fn forbidden_stays_sanitized_constant() {
    let resp = get_query(build_app(), "/probe/403").await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let v = read_json(resp).await;
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(v["message"], err_msg::PERMISSION_PUBLIC);
    assert!(
        !v["message"]
            .as_str()
            .unwrap()
            .contains("crm/cross_owner_write"),
        "权限判定依据严禁外显: {}",
        v["message"]
    );
}

/// 解码成功的正常路径不被收口层改写
#[tokio::test]
async fn success_path_untouched() {
    let resp = get_query(build_app(), "/probe/ok").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    assert_eq!(bytes.as_ref(), b"ok");
}

/// 正常解码通过的 JSON POST 不被误伤
#[tokio::test]
async fn json_success_path_untouched() {
    let resp = post_json(build_app(), r#"{"hazard_name":"粉尘","threshold_value":8}"#).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
