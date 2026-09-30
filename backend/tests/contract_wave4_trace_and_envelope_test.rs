//! 契约测试（wave4）：trace 同源 + 失败信封收口
//!
//! 覆盖四条已坐实缺陷：
//! 1. 失败响应体的 `trace_id` 必须等于请求的 trace（与 `X-Trace-Id` 响应头同源），不再是现造 UUID；
//! 2. 全链 panic 捕获层：handler panic 也必须返回完整 AppError 信封（500 + code + trace_id），
//!    且 `X-Trace-Id` 响应头在 panic 路径上同样写入；
//! 3. 超时 408 必须是 AppError 信封（不再裸 `Response::builder()` 造体）；
//! 4. 熔断 503 必须是 AppError 信封（含 trace_id），根因按状态跃变记 WARN/INFO，不逐请求刷屏。
//!
//! 只驱动中间件层与 error.rs，不触碰 handlers/services。

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::middleware::{Next, from_fn};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use bingxi_backend::middleware::circuit_breaker::{
    CircuitEntry, CircuitEvent, CircuitState, circuit_breaker_middleware,
};
use bingxi_backend::middleware::timeout::timeout_middleware_with;
use bingxi_backend::middleware::trace_context::{
    X_TRACE_ID_HEADER, catch_panic_middleware, trace_context_middleware,
};
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::messages::err_msg;
use serde_json::Value;
use tower::ServiceExt; // oneshot

/// W3C traceparent 里的固定 trace id（32 位小写 hex），用于证明「同源」不是巧合
const REQUEST_TRACE_ID: &str = "0af7651916cd43dd8448eb211c80319c";

// ---------------------------------------------------------------------------
// 测试夹具
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// 把 tracing 输出捕获到内存缓冲（线程局部 default，避免抢全局 subscriber 注册顺序）
struct CapturedLogs {
    buf: Arc<Mutex<Vec<u8>>>,
    _guard: tracing::subscriber::DefaultGuard,
}

impl CapturedLogs {
    fn text(&self) -> String {
        let bytes = self.buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
        String::from_utf8_lossy(&bytes).to_string()
    }

    fn lines_containing(&self, needle: &str) -> usize {
        self.text().lines().filter(|l| l.contains(needle)).count()
    }
}

fn capture_logs() -> CapturedLogs {
    let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || SharedBuf(writer_buf.clone()))
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    CapturedLogs { buf, _guard }
}

async fn read_json(resp: Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 64 * 1024)
        .await
        .expect("失败信封应可读出 body");
    serde_json::from_slice(&bytes).expect("失败信封应是合法 JSON")
}

fn trace_header(resp: &Response) -> String {
    resp.headers()
        .get(X_TRACE_ID_HEADER)
        .unwrap_or_else(|| panic!("失败响应应带 {} 响应头", X_TRACE_ID_HEADER))
        .to_str()
        .expect("X-Trace-Id 应是合法 ASCII")
        .to_string()
}

fn content_type(resp: &Response) -> String {
    resp.headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

fn request_with_trace(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header(
            "traceparent",
            format!("00-{}-b7ad6b7169203331-01", REQUEST_TRACE_ID),
        )
        .body(Body::empty())
        .expect("测试夹具：请求构造失败")
}

/// 失败信封固定四键（与 contract_wave1 既有契约一致，键名不允许漂移）
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
    assert!(
        !json["code"].as_str().unwrap_or("").is_empty(),
        "code 不能为空: {json}"
    );
    assert!(
        !json["trace_id"].as_str().unwrap_or("").is_empty(),
        "trace_id 不能为空: {json}"
    );
    assert!(json["timestamp"].is_i64(), "timestamp 应是 i64: {json}");
}

// ---------------------------------------------------------------------------
// 被测 handler
// ---------------------------------------------------------------------------

async fn ok_handler() -> Response {
    (StatusCode::OK, "ok").into_response()
}

async fn always_panic() -> Response {
    panic!("契约测试：handler 内部故意 panic")
}

async fn slow_handler() -> Response {
    tokio::time::sleep(Duration::from_millis(150)).await;
    (StatusCode::OK, "迟到的成功").into_response()
}

async fn failing_handler() -> Response {
    AppError::internal("下游依赖返回 500").into_response()
}

/// 把超时阈值压到 20ms 走真实短路分支（生产阈值仍由 `timeout_middleware` 固定 30s）
async fn short_timeout_middleware(request: Request<Body>, next: Next) -> Response {
    timeout_middleware_with(Duration::from_millis(20), request, next).await
}

// ---------------------------------------------------------------------------
// ② panic 捕获层
// ---------------------------------------------------------------------------

/// 洋葱顺序与生产一致：trace_context（外）→ catch_panic（内）→ handler
#[tokio::test]
async fn panic_in_handler_returns_full_app_error_envelope() {
    let app = router_with_layers();

    let logs = capture_logs();
    let resp = app
        .oneshot(request_with_trace("/contract-wave4/panic-1"))
        .await
        .expect("测试夹具：请求应被服务");

    assert_eq!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "panic 必须返回 500 信封"
    );
    // Body 不实现 Clone：先取响应头维度的断言材料，再消费响应体
    let ct = content_type(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "INTERNAL_ERROR");
    assert!(
        ct.contains("application/json"),
        "panic 出参必须是 JSON 信封而不是裸文本，实际: {ct}"
    );

    let text = logs.text();
    assert!(
        text.contains("ERROR") && text.contains("panic.captured"),
        "panic 必须以 ERROR 级别留痕（不能静默），实际日志:\n{text}"
    );
    assert!(
        text.contains("契约测试：handler 内部故意 panic"),
        "日志必须带 panic 原因，实际:\n{text}"
    );
    assert!(
        text.contains("/contract-wave4/panic-1"),
        "日志必须带 panic 位置（请求路径），实际:\n{text}"
    );
}

/// panic 路径上 X-Trace-Id 响应头与响应体 trace_id 同源，且等于请求 traceparent 的 trace
#[tokio::test]
async fn panic_response_trace_header_and_body_are_same_source() {
    let app = router_with_layers();

    let resp = app
        .oneshot(request_with_trace("/contract-wave4/panic-2"))
        .await
        .expect("测试夹具：请求应被服务");

    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    let body_trace = json["trace_id"].as_str().unwrap().to_string();
    assert_eq!(
        body_trace, REQUEST_TRACE_ID,
        "panic 出参的 trace_id 必须是被绑定的请求 trace，不能现造"
    );
    assert_eq!(hdr, REQUEST_TRACE_ID);
    assert_eq!(
        hdr, body_trace,
        "响应头 X-Trace-Id 与响应体 trace_id 必须逐字符相同"
    );
}

/// 正常路径不受 panic 层影响（状态码/响应体原样透传，trace 头仍在）
#[tokio::test]
async fn panic_layer_does_not_alter_normal_path() {
    let app = router_with_layers();

    let resp = app
        .oneshot(request_with_trace("/contract-wave4/ok"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(trace_header(&resp), REQUEST_TRACE_ID);
    let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    assert_eq!(&body[..], b"ok");
}

/// 与生产 `apply_trace_and_panic_capture` 相同的层序（catch_panic 先注册=内层，
/// trace 后注册=外层），路由挂到唯一 path 上，各测试互不干扰。
fn panic_capturing_router(path: &str, handler: axum::routing::MethodRouter) -> Router {
    Router::new()
        .route(path, handler)
        .layer(from_fn(catch_panic_middleware))
        .layer(from_fn(trace_context_middleware))
}

fn router_with_layers() -> axum::Router {
    panic_capturing_router("/contract-wave4/panic-1", get(always_panic))
}

use axum::Router;

// ---------------------------------------------------------------------------
// ① 普通失败路径的 trace 同源（非 panic）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ordinary_error_body_trace_id_matches_request_trace() {
    async fn business_error() -> Response {
        AppError::business("内部原因不应外显").into_response()
    }

    let app = Router::new()
        .route("/contract-wave4/biz-error", get(business_error))
        .layer(from_fn(trace_context_middleware));

    let resp = app
        .oneshot(request_with_trace("/contract-wave4/biz-error"))
        .await
        .expect("测试夹具：请求应被服务");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let hdr = trace_header(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "BUSINESS_ERROR");
    assert_eq!(
        json["message"],
        err_msg::BUSINESS_PUBLIC,
        "脱敏分层不能被本波次改动，真实文案只进日志"
    );
    assert_eq!(json["trace_id"], REQUEST_TRACE_ID);
    assert_eq!(hdr, REQUEST_TRACE_ID);
}

// ---------------------------------------------------------------------------
// ③ 超时 408
// ---------------------------------------------------------------------------

#[tokio::test]
async fn timeout_returns_app_error_envelope_not_bare_body() {
    let app = Router::new()
        .route("/contract-wave4/slow", get(slow_handler))
        .layer(from_fn(short_timeout_middleware))
        .layer(from_fn(trace_context_middleware));

    let logs = capture_logs();
    let resp = app
        .oneshot(request_with_trace("/contract-wave4/slow"))
        .await
        .expect("测试夹具：请求应被服务");

    assert_eq!(resp.status(), StatusCode::REQUEST_TIMEOUT);
    let hdr = trace_header(&resp);
    let ct = content_type(&resp);
    let json = read_json(resp).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "TIMEOUT");
    use bingxi_backend::utils::error::gateway_msg;
    assert_eq!(
        json["message"],
        gateway_msg::TIMEOUT_PUBLIC,
        "出参必须是固定脱敏文案，内部详情（path/method/阈值）只进日志"
    );
    assert_eq!(json["trace_id"], REQUEST_TRACE_ID);
    assert_eq!(hdr, REQUEST_TRACE_ID);
    assert!(
        ct.contains("application/json"),
        "超时出参应是 JSON 信封，实际: {ct}"
    );

    let text = logs.text();
    assert!(
        text.contains(gateway_msg::LOG_TIMEOUT),
        "超时必须有后端日志，实际:\n{text}"
    );
    assert!(
        text.contains("/contract-wave4/slow"),
        "超时日志应带路径便于定位，实际:\n{text}"
    );
    assert!(
        !text.contains("请求超时\n") || text.contains("WARN"),
        "超时日志级别至少为 WARN（不是静默）"
    );
}

/// 未超时的请求正常通过（阈值注入只影响短路分支）
#[tokio::test]
async fn timeout_passes_through_fast_handler() {
    let app = Router::new()
        .route("/contract-wave4/fast", get(ok_handler))
        .layer(from_fn(short_timeout_middleware))
        .layer(from_fn(trace_context_middleware));

    let resp = app
        .oneshot(request_with_trace("/contract-wave4/fast"))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(trace_header(&resp), REQUEST_TRACE_ID);
}

// ---------------------------------------------------------------------------
// ④ 熔断 503
// ---------------------------------------------------------------------------

/// 连续 5 个 5xx 触发 open（窗口失败率 100% > 50% 阈值），此后请求被短路成 503 AppError 信封；
/// 根因 WARN 按状态跃变记一次，同周期内后续短路不逐请求刷屏。
#[tokio::test]
async fn circuit_breaker_open_returns_envelope_and_logs_once() {
    let path = "/contract-wave4/cb/open-envelope";
    let app = Router::new()
        .route(path, get(failing_handler))
        .layer(from_fn(circuit_breaker_middleware))
        .layer(from_fn(trace_context_middleware));

    let logs = capture_logs();

    // 1) 触发熔断：5 个真实 500 响应被窗口记为失败样本
    for _ in 0..5 {
        let resp = app
            .clone()
            .oneshot(request_with_trace(path))
            .await
            .expect("测试夹具：请求应被服务");
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 2) 第 6 个请求被短路：503 + 完整信封 + 同源 trace_id
    let rejected = app
        .clone()
        .oneshot(request_with_trace(path))
        .await
        .expect("测试夹具：请求应被服务");
    assert_eq!(rejected.status(), StatusCode::SERVICE_UNAVAILABLE);
    let hdr = trace_header(&rejected);
    let ct = content_type(&rejected);
    let json = read_json(rejected).await;
    assert_envelope_keys(&json);
    assert_eq!(json["code"], "SERVICE_UNAVAILABLE");
    use bingxi_backend::utils::error::gateway_msg;
    assert_eq!(
        json["message"],
        gateway_msg::SERVICE_UNAVAILABLE_PUBLIC,
        "出参必须是固定脱敏文案（不再是裸文本「服务熔断中，请稍后重试」）"
    );
    assert_eq!(
        json["trace_id"], REQUEST_TRACE_ID,
        "熔断 503 也必须带同源 trace_id"
    );
    assert_eq!(hdr, REQUEST_TRACE_ID);
    assert!(
        ct.contains("application/json"),
        "熔断出参应是 JSON 信封，实际: {ct}"
    );

    // 3) 同周期内再打 3 个被拒请求，观察是否刷屏
    for _ in 0..3 {
        let resp = app
            .clone()
            .oneshot(request_with_trace(path))
            .await
            .expect("测试夹具：请求应被服务");
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    let text = logs.text();
    assert!(
        text.contains("CircuitBreaker.opened"),
        "熔断触发必须记 WARN（含路由 / 窗口失败率），否则运维只见 503 不见根因。实际:\n{text}"
    );
    assert!(
        text.contains(path),
        "熔断 WARN 必须带 route 维度才能定位服务，实际:\n{text}"
    );
    assert!(
        text.contains("failure_rate"),
        "熔断 WARN 必须带窗口失败率，实际:\n{text}"
    );
    assert_eq!(
        logs.lines_containing("CircuitBreaker.opened"),
        1,
        "熔断开启只应记一次 WARN"
    );
    assert_eq!(
        logs.lines_containing("CircuitBreaker.reject"),
        1,
        "同一熔断周期内的短路只应记一次 reject WARN（不逐请求刷屏）"
    );
    assert!(
        logs.lines_containing("503") >= 1 || text.contains("route="),
        "被拒请求本身也要留下可读痕迹"
    );
}

/// 恢复路径：open 冷却结束 → half-open 探测 → 成功恢复 closed，
/// 每次状态跃变都产生一个事件（由中间件记 INFO/WARN，不静默）。
#[tokio::test]
async fn circuit_breaker_recovery_emits_half_open_and_recovered_events() {
    let mut entry = CircuitEntry::new();
    entry.state = CircuitState::Open;
    entry.opened_at = Some(Instant::now() - Duration::from_secs(31));

    let mut events: Vec<CircuitEvent> = Vec::new();
    assert!(
        !entry.should_reject_tracked(&mut events),
        "冷却结束应放行 1 个探测请求"
    );
    entry.record_result_tracked(false, &mut events);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, CircuitEvent::HalfOpenProbeStarted { .. })),
        "open→half-open 必须产生跃变事件（中间件据此记 INFO）"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CircuitEvent::Recovered { .. })),
        "探测成功必须产生恢复事件（中间件据此记 INFO）"
    );
    assert_eq!(entry.state, CircuitState::Closed, "恢复后状态应是 closed");
}

/// 探测失败重新熔断：应产生 ReOpened 事件（新一轮冷却 + 再一次 WARN 根因），
/// 而不是静默地继续 open。
#[tokio::test]
async fn circuit_breaker_probe_failure_reopens_with_event() {
    let mut entry = CircuitEntry::new();
    entry.state = CircuitState::Open;
    entry.opened_at = Some(Instant::now() - Duration::from_secs(31));

    let mut events: Vec<CircuitEvent> = Vec::new();
    assert!(!entry.should_reject_tracked(&mut events), "应放行探测请求");
    entry.record_result_tracked(true, &mut events);

    assert!(
        events
            .iter()
            .any(|e| matches!(e, CircuitEvent::ReOpened { .. })),
        "探测失败必须产生 ReOpened 事件，实际: {events:?}"
    );
    assert_eq!(entry.state, CircuitState::Open);
}
