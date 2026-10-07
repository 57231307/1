//! 契约波次 8（任务板）：提取器拒绝收口层的**活体锁**
//!
//! 被锁实现（源码侧已闭，本文件只证明"它真生效 + 不会被无声打开"）：
//! - 收口点：`backend/src/middleware/trace_context.rs:259-309` `normalize_extractor_rejection`
//!   （挂在 catch_panic 层响应回廊 `trace_context.rs:339-342`）。
//!   窄判据 = 状态 ∈ {400,415,422} **且** content-type 以 `text/plain` 开头才改写信封；
//!   serde 原文逐条 `tracing::warn`（事件名 `extractor.rejection_normalized`，含
//!   `original_status`/`raw_rejection`）只进日志不外显；JSON 信封原样放行。
//! - 构造入口：`backend/src/utils/error.rs:802-815` `AppError::request_decoding_failed()`
//!   ⇒ 400 + `code=VALIDATION_ERROR` + 恒定公开文案 `REQUEST_DECODING_PUBLIC`（error.rs:799-800）。
//!
//! 与既有测试的分工：`contract_wave7_extractor_rejection_envelope_test.rs` 锁了
//! "拒绝会被归一"的正向通道与鉴权面 message 常量；本文件补它没有、或被拆环后必然失守的四条：
//! ① 三形输入（畸形 JSON / 缺必填字段 / 缺 Content-Type）逐形钉"外显恒等于公开常量 +
//!   绝不含 serde 路径/字段名/结构体名"的双向钉；
//! ② 反向不误伤：业务 400（BUSINESS_ERROR）、403、401、404（AppError 信封与未注册路由
//!   两通道）逐字段不变——权限类拒绝按本仓裁定**只断 status + code + 信封键集合**，
//!   不断任何原因文案；
//! ③ 5xx 不被吞：AppError 内部错与 handler panic 仍分别回 500 + INTERNAL_ERROR，
//!   不经 normalize（收口层若被扩到 5xx 立即红）；
//! ④ **构造点不变量**（本文件最有价值的一条）：穷举 `AppError` 全部 14 个变体经
//!   `into_response()` 观测到的状态码集合，显式排除 415/422，并以**无通配 exhaustive
//!   match** 作为编译期棘轮——normalize 的"纯文本 422 必为提取器拒绝"闭合判据
//!   （trace_context.rs:246-247 目前只写在注释里）依赖"AppError 体系不存在 415/422
//!   构造点"；未来任何人新增一个产 415/422 的变体：轻则该 match 编译失败强制点名，
//!   重则该变体进入状态集合后断言必红，洞无法静默张开。
//! ⑤ 不静默锁：normalize 命中时日志必现 `extractor.rejection_normalized` 且带
//!   `original_status=<原状态码>` 与 serde 原文（`raw_rejection`）；未命中时
//!   （JSON 信封 400/403、内部错 500）该行必不出现——防"改写信封但把归一记成静默"、
//!   也防"判据外溢后每条业务 400 都刷归一日志"两个反向漂移。
//!
//! 夹具口径：纯 axum Router + tower `oneshot` + `catch_panic_middleware` 真实挂载
//! （同 wave7 / wave4 `contract_wave4_trace_and_envelope_test.rs` 范式），收口层不触库，
//! 无需 `setup_test_db()`；日志捕获沿用 wave4 的 `tracing_subscriber::fmt` 内存 writer
//! 线程局部夹具（`capture_logs`，仓内既有范式）。无 `#[ignore]`、无 skip、无 mock。

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::from_fn,
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
// 探针 DTO / handler（仅测试用最小形状；字段名/结构体名刻意做成可识别特征词，
// 泄露负断言才能钉到 serde 原文的真实形态）
// ============================================================================

#[derive(Deserialize)]
struct LockProbeOrderDto {
    order_no: String,
    quantity_count: i64,
}

/// 成功路径把两个字段都用上：探针不因"未读取"而需要 `#[allow(dead_code)]` 压警告，
/// 同时保证只在拒绝被错误放行时才会走到这里（返回体不参与任何断言）。
async fn probe_lock_json(Json(dto): Json<LockProbeOrderDto>) -> String {
    format!("ok:{}:{}", dto.order_no, dto.quantity_count)
}

/// 业务 400：JSON 信封（application/json），detail 含不可外显内部特征
async fn probe_business_400() -> AppError {
    AppError::business(
        "内部原因：仓库 WH01 库存不足，SQL: SELECT stock FROM inventory_wh WHERE wh_id=1",
    )
}

/// 权限 403：JSON 信封（utils/response.rs 同源 into_response 路径）
async fn probe_forbidden_403() -> AppError {
    AppError::permission_denied("内部原因：缺少权限键 crm/cross_owner_write，白名单段=privacy")
}

/// 认证 401：JSON 信封
async fn probe_unauthorized_401() -> AppError {
    AppError::unauthorized("内部原因：token 已过期，签发于 2026-01-01T00:00:00Z")
}

/// 业务 404：AppError::NotFound → JSON 信封（与未注册路由的默认 404 是两条通道，都锁）
async fn probe_not_found_404() -> AppError {
    AppError::not_found("内部：dye_batch id=999 不存在")
}

/// 5xx 通道一：AppError 内部错（JSON 500，normalize 判据不得触及）
async fn probe_internal_500() -> AppError {
    AppError::internal("内部：数据库连接池耗尽 pool=main waited=30s")
}

/// 5xx 通道二：handler panic（由 catch_panic 层转 500 信封，不经 normalize）
async fn probe_panic_500() -> &'static str {
    panic!("探针：模拟处理器内部崩溃 boom-lock-probe");
}

fn build_app() -> Router {
    Router::new()
        .route("/probe/lock-json", post(probe_lock_json))
        .route("/probe/business-400", get(probe_business_400))
        .route("/probe/forbidden-403", get(probe_forbidden_403))
        .route("/probe/unauthorized-401", get(probe_unauthorized_401))
        .route("/probe/not-found-404", get(probe_not_found_404))
        .route("/probe/internal-500", get(probe_internal_500))
        .route("/probe/panic-500", get(probe_panic_500))
        .layer(from_fn(catch_panic_middleware))
}

// ============================================================================
// 响应读取与通用形状断言
// ============================================================================

/// 读全响应：返回 (状态码, content-type 原文, body 文本, body 按 JSON 解析的结果)。
/// JSON 解析失败时不 panic 丢弃原文——由调用方按"必须是信封"的语义显式判红。
async fn read_response(resp: Response) -> (StatusCode, String, String, Option<Value>) {
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .expect("响应体应可读");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let json = serde_json::from_str::<Value>(&text).ok();
    (status, content_type, text, json)
}

/// serde 原文 / 内部标识特征串：绝不允许出现在归一封面的可外显 message 里
const SERDE_LEAK_MARKERS: &[&str] = &[
    "invalid type",
    "missing field",
    "invalid value",
    "at line",
    "column",
    "LockProbeOrderDto",
    "order_no",
    "quantity_count",
    "Failed to deserialize",
    "Expected request with",
    "MIME",
];

/// 失败信封固定四键（`utils/error.rs:303-308` 唯一形状，键名不允许漂移）
fn assert_envelope_keys(v: &Value) {
    let mut keys: Vec<&str> = v
        .as_object()
        .expect("失败信封应是 JSON 对象")
        .keys()
        .map(|k| k.as_str())
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["code", "message", "timestamp", "trace_id"],
        "失败信封必须恰好是四键唯一形状，实际键: {keys:?}"
    );
}

/// 归一信封的完整钉：400 + application/json + VALIDATION_ERROR + message 恒等于
/// 公开常量 + 双向负断言（不含任何 serde/结构体/字段名特征）+ 四键齐全。
fn assert_normalized_envelope(status: StatusCode, content_type: &str, body_text: &str, v: &Value) {
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "提取器拒绝必须被归一为 400（415/422 不得存活到出参），实际 {status}"
    );
    assert!(
        content_type.starts_with("application/json"),
        "归一后响应必须是 JSON 信封而非 axum 纯文本，实际 content-type: {content_type:?}，body: {body_text}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "实际信封: {v}");
    assert_eq!(
        v["message"], REQUEST_DECODING_PUBLIC,
        "归一信封 message 必须恒等于公开常量 REQUEST_DECODING_PUBLIC，实际: {v}"
    );
    let msg = v["message"].as_str().expect("message 应是字符串");
    for marker in SERDE_LEAK_MARKERS {
        assert!(
            !msg.contains(marker),
            "外显 message 不得含 serde/内部特征串 {marker:?}（serde 原文只进日志），实际: {msg}"
        );
    }
    assert_envelope_keys(v);
    let trace = v["trace_id"].as_str().expect("trace_id 应存在且为字符串");
    assert!(!trace.is_empty(), "trace_id 不得为空");
    assert!(
        v["timestamp"].is_i64() && v["timestamp"].as_i64().unwrap() > 0,
        "timestamp 应为正的 unix 秒，实际: {}",
        v["timestamp"]
    );
}

async fn post_lock_json(body: &'static str, content_type: Option<&'static str>) -> Response {
    let mut builder = Request::builder().method("POST").uri("/probe/lock-json");
    if let Some(ct) = content_type {
        builder = builder.header("content-type", ct);
    }
    build_app()
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap()
}

/// 信封族通用读取 + "JSON 四键形状"断言，返回 (status, Value)。
/// 解析失败即红并把原始纯文本贴进失败信息（暴露真实形状，不吞）。
async fn read_envelope(resp: Response, expect: &str) -> (StatusCode, Value) {
    let (status, content_type, text, json) = read_response(resp).await;
    let v = json.unwrap_or_else(|| {
        panic!("{expect}：失败响应必须是 AppError JSON 信封，实际 content-type={content_type:?} body={text}")
    });
    assert_envelope_keys(&v);
    (status, v)
}

// ============================================================================
// 锁①：三形输入打同一端点 → 各自 400 + JSON 信封 + VALIDATION_ERROR +
//       message 恒等于 REQUEST_DECODING_PUBLIC（双向钉）
// ============================================================================

/// 形一：畸形 JSON body（JsonSyntaxError，axum 原生 400 纯文本通道）
#[tokio::test]
async fn malformed_json_body_gets_normalized_envelope() {
    let resp = post_lock_json(
        "{\"order_no\": \"SO-1\", \"quantity_count\": ",
        Some("application/json"),
    )
    .await;
    let (status, content_type, text, json) = read_response(resp).await;
    let v = json.expect("归一后必须是合法 JSON（还是纯文本即收口被拆）");
    assert_normalized_envelope(status, &content_type, &text, &v);
}

/// 形二：缺必填字段（JsonDataError，axum 原生 422 纯文本通道——normalize 判据
/// 的核心依赖形，422 必为提取器拒绝的闭合由锁④兜底）
#[tokio::test]
async fn missing_required_field_gets_normalized_envelope() {
    let resp = post_lock_json("{\"order_no\": \"SO-1\"}", Some("application/json")).await;
    let (status, content_type, text, json) = read_response(resp).await;
    let v = json.expect("归一后必须是合法 JSON（422 纯文本存活即收口被拆）");
    assert_normalized_envelope(status, &content_type, &text, &v);
}

/// 形三：缺 `Content-Type: application/json`（MissingJsonContentType，原生 415 通道）
#[tokio::test]
async fn missing_content_type_gets_normalized_envelope() {
    let resp = post_lock_json("{\"order_no\": \"SO-1\", \"quantity_count\": 5}", None).await;
    let (status, content_type, text, json) = read_response(resp).await;
    let v = json.expect("归一后必须是合法 JSON（415 纯文本存活即收口被拆）");
    assert_normalized_envelope(status, &content_type, &text, &v);
}

// ============================================================================
// 锁②：反向不误伤——业务 400 / 403 / 401 / 404 的响应体与状态码逐字段不变
//       （对照既有信封形状；权限类拒绝只断 status + code + 键集合，不断原因文案）
// ============================================================================

/// 业务 400（BUSINESS_ERROR，JSON 信封）：状态、code、脱敏 message 常量、四键
/// 全部不因 normalize 挂在同一回廊而漂移——收口判据若被放宽到"所有 400"，
/// 这里的 message 会被改写成 REQUEST_DECODING_PUBLIC，本锁必红。
#[tokio::test]
async fn business_400_json_envelope_untouched() {
    let resp = build_app()
        .oneshot(
            Request::get("/probe/business-400")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, v) = read_envelope(resp, "业务 400 通道").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(
        v["message"],
        err_msg::BUSINESS_PUBLIC,
        "非外显业务错的脱敏常量不得被归一层改写"
    );
    assert_ne!(
        v["message"], REQUEST_DECODING_PUBLIC,
        "业务 400 绝不得被误改写为请求解码文案"
    );
}

/// 权限 403：按本仓裁定只断 status + code（FORBIDDEN）+ 四键形状，不断任何原因文案。
/// 收口判据若外溢到 application/json 或非 text/plain 通道，状态/形状先红。
#[tokio::test]
async fn forbidden_403_status_and_code_only() {
    let resp = build_app()
        .oneshot(
            Request::get("/probe/forbidden-403")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, v) = read_envelope(resp, "权限 403 通道").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
}

/// 认证 401：同上只断 status + code（UNAUTHORIZED）+ 四键形状。
#[tokio::test]
async fn unauthorized_401_status_and_code_only() {
    let resp = build_app()
        .oneshot(
            Request::get("/probe/unauthorized-401")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, v) = read_envelope(resp, "认证 401 通道").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(v["code"], "UNAUTHORIZED");
}

/// AppError::NotFound 404：信封族 404 逐字段不变（message 仍为脱敏常量，
/// 不被改写为解码文案）。
#[tokio::test]
async fn not_found_404_envelope_untouched() {
    let resp = build_app()
        .oneshot(
            Request::get("/probe/not-found-404")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, v) = read_envelope(resp, "业务 404 通道").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(v["code"], "NOT_FOUND");
    assert_eq!(v["message"], err_msg::NOT_FOUND_PUBLIC);
    assert_ne!(v["message"], REQUEST_DECODING_PUBLIC);
}

/// 未注册路由 404（axum 默认 not_found，非 AppError 通道）：状态必须仍是 404，
/// 不得被并入解码归一族（404 不在 normalize 状态判据内，trace_context.rs:251-252）。
#[tokio::test]
async fn unregistered_route_404_not_rewritten() {
    let resp = build_app()
        .oneshot(
            Request::get("/probe/this-route-does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "未注册路由 404 被改写即收口判据外溢"
    );
}

// ============================================================================
// 锁③：5xx 不被吞——内部错与 handler panic 仍分别回 500 与原 code，不经 normalize
// ============================================================================

#[tokio::test]
async fn app_error_internal_500_not_swallowed() {
    let resp = build_app()
        .oneshot(
            Request::get("/probe/internal-500")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, v) = read_envelope(resp, "内部错 500 通道").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(v["code"], "INTERNAL_ERROR", "500 的 code 不得被改成归一族");
    assert_ne!(
        v["message"], REQUEST_DECODING_PUBLIC,
        "内部错绝不得被映射成请求解码文案（那会把服务端故障洗成客户端可修正）"
    );
}

#[tokio::test]
async fn handler_panic_gets_full_500_envelope_not_validation() {
    let logs = capture_logs();
    let resp = build_app()
        .oneshot(
            Request::get("/probe/panic-500")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, v) = read_envelope(resp, "panic 捕获通道").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(v["code"], "INTERNAL_ERROR");
    assert_ne!(v["message"], REQUEST_DECODING_PUBLIC);
    // panic 路径的留痕由 catch_panic 层负责（不静默），同样不得伪装成归一事件
    assert!(
        logs.text().contains("panic.captured"),
        "handler panic 必须记 panic.captured 日志，实际日志:\n{}",
        logs.text()
    );
    assert!(
        !logs.text().contains("extractor.rejection_normalized"),
        "panic 响应不是提取器拒绝，绝不该触发归一日志"
    );
}

// ============================================================================
// 锁④：构造点不变量——AppError 全部变体经 into_response() 的状态码集合里
//       不存在 415/422（normalize 的"纯文本 422 必为提取器拒绝"闭合判据依赖它；
//       该前提今天只写在 trace_context.rs:246-247 注释里，本锁把它烤成可执行契约）
// ============================================================================

/// 穷举映射：每个变体 → (变体名, `error_status_and_type` 现行状态码，
/// 见 utils/error.rs:357-372)。**故意不写通配 `_ =>`**：新增任何 AppError 变体
/// 时本函数编译失败，强制作者重新过一遍"新状态码会不会拆掉 422 闭合前提"。
fn describe_variant(err: &AppError) -> (&'static str, StatusCode) {
    match err {
        AppError::DatabaseError(_) => ("DatabaseError", StatusCode::INTERNAL_SERVER_ERROR),
        AppError::ValidationError(_) => ("ValidationError", StatusCode::BAD_REQUEST),
        AppError::ValidationErrorDisplayable(_) => {
            ("ValidationErrorDisplayable", StatusCode::BAD_REQUEST)
        }
        AppError::NotFound(_) => ("NotFound", StatusCode::NOT_FOUND),
        AppError::BusinessError(_) => ("BusinessError", StatusCode::BAD_REQUEST),
        AppError::BusinessErrorDisplayable(_) => {
            ("BusinessErrorDisplayable", StatusCode::BAD_REQUEST)
        }
        AppError::Unauthorized(_) => ("Unauthorized", StatusCode::UNAUTHORIZED),
        AppError::InternalError(_) => ("InternalError", StatusCode::INTERNAL_SERVER_ERROR),
        AppError::PermissionDenied(_) => ("PermissionDenied", StatusCode::FORBIDDEN),
        AppError::BadRequest(_) => ("BadRequest", StatusCode::BAD_REQUEST),
        AppError::NotImplemented(_) => ("NotImplemented", StatusCode::NOT_IMPLEMENTED),
        AppError::TooManyRequests { .. } => ("TooManyRequests", StatusCode::TOO_MANY_REQUESTS),
        AppError::Timeout(_) => ("Timeout", StatusCode::REQUEST_TIMEOUT),
        AppError::ServiceUnavailable(_) => ("ServiceUnavailable", StatusCode::SERVICE_UNAVAILABLE),
    }
}

fn all_app_error_variants() -> Vec<AppError> {
    vec![
        AppError::DatabaseError("探针".to_string()),
        AppError::ValidationError("探针".to_string()),
        AppError::ValidationErrorDisplayable("探针".to_string()),
        AppError::NotFound("探针".to_string()),
        AppError::BusinessError("探针".to_string()),
        AppError::BusinessErrorDisplayable("探针".to_string()),
        AppError::Unauthorized("探针".to_string()),
        AppError::InternalError("探针".to_string()),
        AppError::PermissionDenied("探针".to_string()),
        AppError::BadRequest("探针".to_string()),
        AppError::NotImplemented("探针".to_string()),
        AppError::TooManyRequests {
            retry_after: Some(1),
            message: "探针".to_string(),
        },
        AppError::Timeout("探针".to_string()),
        AppError::ServiceUnavailable("探针".to_string()),
    ]
}

#[test]
fn app_error_variant_status_never_produces_415_or_422() {
    let variants = all_app_error_variants();
    // 条数棘轮：describe_variant 的 exhaustive match 只防"加变体不改映射"；
    // 新增变体的人补完 match 分支后若忘了把变体放进本构造集，闭合前提照样漏钉。
    // 数量硬钉 14（= utils/error.rs:144-177 变体全集），加变体不补构造=当场红。
    assert_eq!(
        variants.len(),
        14,
        "AppError 构造集与变体全集脱钩：新增变体必须先补进 all_app_error_variants()，\
         再确认其状态码不落入 415/422（否则 trace_context 收口判据被静默打开）"
    );
    for err in &variants {
        let (name, expected) = describe_variant(err);
        let observed = err.clone().into_response().status();
        assert_eq!(
            observed, expected,
            "变体 {name} 的出参状态码漂移（error.rs:357-372 映射已定稿，改族=拆 normalize 闭合前提）"
        );
        assert!(
            !matches!(
                observed,
                StatusCode::UNSUPPORTED_MEDIA_TYPE | StatusCode::UNPROCESSABLE_ENTITY
            ),
            "变体 {name} 产出 {observed}：AppError 体系出现 415/422 构造点，\
             trace_context 收口层'纯文本 422 必为提取器拒绝'的判据被静默打开，必须红"
        );
    }
    // 全集视角再钉一次：观测集合本身不含 415/422
    let seen: Vec<StatusCode> = variants
        .iter()
        .map(|e| e.clone().into_response().status())
        .collect();
    assert!(
        !seen.contains(&StatusCode::UNSUPPORTED_MEDIA_TYPE)
            && !seen.contains(&StatusCode::UNPROCESSABLE_ENTITY),
        "AppError 变体状态码全集 {seen:?} 混入 415/422"
    );
    // 状态码全集显式钉在已定稿集合内（400/401/403/404/408/429/500/501/503）
    for status in &seen {
        assert!(
            matches!(
                *status,
                StatusCode::BAD_REQUEST
                    | StatusCode::UNAUTHORIZED
                    | StatusCode::FORBIDDEN
                    | StatusCode::NOT_FOUND
                    | StatusCode::REQUEST_TIMEOUT
                    | StatusCode::TOO_MANY_REQUESTS
                    | StatusCode::INTERNAL_SERVER_ERROR
                    | StatusCode::NOT_IMPLEMENTED
                    | StatusCode::SERVICE_UNAVAILABLE
            ),
            "出现未定稿状态码 {status}：新增构造点必须先过 normalize 闭合性评审再进本集合"
        );
    }
}

// ============================================================================
// 锁⑤：不静默——normalize 命中必记 extractor.rejection_normalized 且带
//       original_status；未命中（JSON 信封族）必不记
// ============================================================================

/// 内存 writer + 线程局部 default subscriber（沿用 wave4
/// contract_wave4_trace_and_envelope_test.rs:42-83 既有夹具范式）
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

struct CapturedLogs {
    buf: Arc<Mutex<Vec<u8>>>,
    _guard: tracing::subscriber::DefaultGuard,
}

impl CapturedLogs {
    fn text(&self) -> String {
        let bytes = self.buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
        String::from_utf8_lossy(&bytes).to_string()
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

/// 形二（缺必填 → 原生 422）命中归一时：日志必现事件名 + original_status=422 +
/// serde 原文（missing field）进日志；同时出参 body **不得**含该原文——
/// "只进日志不外显"的两面在这一条里同时钉死。
#[tokio::test]
async fn normalize_hit_logs_original_status_422_and_keeps_serde_out_of_body() {
    let logs = capture_logs();
    let resp = post_lock_json("{\"order_no\": \"SO-1\"}", Some("application/json")).await;
    let (_, _, body_text, json) = read_response(resp).await;
    let v = json.expect("归一后必须是 JSON 信封");
    assert_eq!(v["code"], "VALIDATION_ERROR");
    let text = logs.text();
    assert!(
        text.contains("extractor.rejection_normalized"),
        "normalize 命中必须记 extractor.rejection_normalized（不静默归一），实际日志:\n{text}"
    );
    assert!(
        text.contains("original_status=422"),
        "归一日志必须带 original_status=422（原判据状态可回溯），实际日志:\n{text}"
    );
    assert!(
        text.contains("missing field"),
        "serde 原文必须逐条进日志（不外显≠丢弃），实际日志:\n{text}"
    );
    assert!(
        !body_text.contains("missing field"),
        "serde 原文严禁出现在 HTTP 出参里，实际 body:\n{body_text}"
    );
}

/// 形三（缺 Content-Type → 原生 415）与形一（语法错 → 原生 400）两通道同样留痕，
/// original_status 逐形不同——防止判据改成"只看 text/plain 不看状态"式粗化。
#[tokio::test]
async fn normalize_hit_logs_original_status_415_and_400() {
    let logs = capture_logs();
    let resp = post_lock_json("{\"order_no\": \"SO-1\", \"quantity_count\": 5}", None).await;
    let (_, _, _, json) = read_response(resp).await;
    let v = json.expect("415 归一后必须是 JSON 信封");
    assert_eq!(v["code"], "VALIDATION_ERROR");
    let text = logs.text();
    assert!(
        text.contains("extractor.rejection_normalized") && text.contains("original_status=415"),
        "415 通道归一必须留痕且带 original_status=415，实际日志:\n{text}"
    );

    let logs2 = capture_logs();
    let resp2 = post_lock_json("{\"order_no\": ", Some("application/json")).await;
    let (_, _, _, json2) = read_response(resp2).await;
    let v2 = json2.expect("400 语法错归一后必须是 JSON 信封");
    assert_eq!(v2["code"], "VALIDATION_ERROR");
    let text2 = logs2.text();
    assert!(
        text2.contains("extractor.rejection_normalized") && text2.contains("original_status=400"),
        "原生 400 纯文本通道归一必须留痕且带 original_status=400，实际日志:\n{text2}"
    );
}

/// 反向：JSON 信封的业务 400 / 权限 403 / 内部错 500 **绝不该**触发归一日志。
/// 若有人把判据的 text/plain 条件删掉（所有 400/415/422 都进归一分支），
/// 本锁当场红——这比出参形状漂移更早、更响。
#[tokio::test]
async fn json_envelope_responses_do_not_log_normalize_hit() {
    let logs = capture_logs();
    for uri in [
        "/probe/business-400",
        "/probe/forbidden-403",
        "/probe/unauthorized-401",
        "/probe/not-found-404",
        "/probe/internal-500",
    ] {
        let resp = build_app()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let (status, _) = read_envelope(resp, uri).await;
        // 顺带钉各通道状态不因日志侧改动漂移
        let expected = match uri {
            "/probe/business-400" => StatusCode::BAD_REQUEST,
            "/probe/forbidden-403" => StatusCode::FORBIDDEN,
            "/probe/unauthorized-401" => StatusCode::UNAUTHORIZED,
            "/probe/not-found-404" => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        assert_eq!(status, expected, "{uri} 状态码漂移");
    }
    let text = logs.text();
    assert!(
        !text.contains("extractor.rejection_normalized"),
        "JSON 信封响应绝不该被记成提取器归一事件，实际日志:\n{text}"
    );
}
