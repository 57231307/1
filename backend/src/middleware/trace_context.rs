//! 分布式追踪上下文中间件 + panic 捕获层
//!
//! 职责：
//! 1. 从请求的 `traceparent` header 解析或生成新的 `TraceContext`
//! 2. 把 `TraceContext` 存入 `Request::extensions()` 供 handler / service 读取
//! 3. 创建 root `tracing::Span`，把 trace_id / span_id 等写入 span 字段
//! 4. 把本次请求的 trace_id 绑定到 `utils::error::TRACE_ID` task-local，使
//!    **`X-Trace-Id` 响应头与失败响应体里的 `trace_id` 严格同源**
//! 5. 在响应头回写 `X-Trace-Id`，便于客户端关联日志
//!
//! 注：handler 主要通过 `Request::extensions()` 取出 ctx。
//!
//! ## panic 捕获层（[`catch_panic_middleware`]）
//!
//! 与追踪上下文的配合关系决定了洋葱顺序：`trace_context`（外）→ `catch_panic`（内）→ 其余层。
//! - `catch_panic` 在 `trace_context` **内层**，因此它把 panic 转成的 `AppError` 信封仍在
//!   `TRACE_ID` task-local 作用域内构造，响应体 `trace_id` 与请求 trace 同源；
//! - 又因为 panic 已被内层消化成一个**正常的** 500 响应，`trace_context` 里 `next.run()`
//!   之后的代码（含第 6 步回写 `X-Trace-Id`）仍会执行 —— 这才是「panic 路径也回写
//!   `X-Trace-Id`」的真实实现方式；
//! - 反之，若 panic 发生在 `trace_context` 自身或比它更外的层，本中间件之后的代码不会
//!   执行（unwind 直接穿出去），那种场景**不会**有 `X-Trace-Id` 响应头。注释与实现一致，
//!   不再有「span 内 panic 仍能回写」的错误声明。

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderName, HeaderValue, StatusCode, header::CONTENT_TYPE},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::task::Poll;
use std::time::Instant;
use uuid::Uuid;

use crate::observability::span::root_span;
use crate::observability::trace_context::{TraceContext, extract_or_new};
use crate::utils::error::{AppError, TRACE_ID, current_trace_id};

/// 用于在响应头回写 `X-Trace-Id`，方便客户端日志关联
pub const X_TRACE_ID_HEADER: &str = "x-trace-id";

/// V15 P2 20.1-C：tail-based sampling 慢请求阈值（毫秒）
/// 超过此阈值的请求强制采样（100%），可通过环境变量 `OTEL_SLOW_REQUEST_MS` 配置。
fn slow_request_threshold_ms() -> u64 {
    use std::sync::LazyLock;
    static THRESHOLD: LazyLock<u64> = LazyLock::new(|| {
        std::env::var("OTEL_SLOW_REQUEST_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2000) // 默认 2s（与 P95 告警阈值对齐）
    });
    *THRESHOLD
}

/// 把 `TraceContext.trace_id` 规范化成一个 `Uuid`（`TRACE_ID` task-local 的类型）。
///
/// trace_id 的两种来源都必须与响应头 / 响应体逐字符一致：
/// - 本服务自生成：`Uuid::new_v4().simple()` → 天然可解析；
/// - 上游 `traceparent` 透传：W3C 规定为 32 位 hex，`extract_or_new` 已校验长度与 hex，
///   但 hex 大小写可能与 `Uuid::simple()` 输出不同 → 需要显式规范化并记 WARN，
///   否则会出现「响应头是上游原样串、响应体是小写串」的不同源。
///
/// 解析失败（理论上只可能是上游脏数据）时**不静默沿用**：记 ERROR 并重建 trace_id，
/// 保证 span 字段、响应头、响应体三者仍是同一个值。
fn canonicalize_trace_id(mut ctx: TraceContext) -> (TraceContext, Uuid) {
    match Uuid::parse_str(&ctx.trace_id) {
        Ok(uuid) => {
            let canonical = uuid.simple().to_string();
            if canonical != ctx.trace_id {
                tracing::warn!(
                    raw_trace_id = %ctx.trace_id,
                    canonical_trace_id = %canonical,
                    "上游 traceparent 的 trace_id 不是 Uuid::simple() 形态（32 位小写 hex），\
                     已按 canonical 值规范化：span 字段 / X-Trace-Id 响应头 / 响应体 trace_id \
                     将统一使用该规范化值以保证同源"
                );
                ctx.trace_id = canonical;
            }
            (ctx, uuid)
        }
        Err(e) => {
            let uuid = Uuid::new_v4();
            let canonical = uuid.simple().to_string();
            tracing::error!(
                raw_trace_id = %ctx.trace_id,
                new_trace_id = %canonical,
                error = %e,
                "trace_id 无法解析为 128-bit UUID，本次请求重建新 trace_id（不使用上游非法值）"
            );
            ctx.trace_id = canonical;
            (ctx, uuid)
        }
    }
}

/// 追踪上下文中间件
pub async fn trace_context_middleware(mut request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    // 1. 解析 / 生成 trace 上下文
    let traceparent = request
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok());
    let ctx = extract_or_new(traceparent);
    // 1b. trace_id 规范化为 Uuid，作为 span / 响应头 / 响应体唯一的同源来源
    let (ctx, trace_uuid) = canonicalize_trace_id(ctx);

    // 2. 把 ctx 放入 request extensions，供下游 handler/service 读取
    request.extensions_mut().insert(ctx.clone());

    // 3. 创建 root span 并在 span 内执行下游
    let method = request.method().clone();
    let uri_path = request.uri().path().to_string();
    let span = root_span(&ctx, method.as_str(), &uri_path);

    // 4. 绑定请求级 trace 上下文并执行下游
    //
    //    `TRACE_ID.scope` 让本次请求 future 内部（含 `catch_panic_middleware`、
    //    超时 / 熔断短路、auth/permission/handler 等所有内层）构造的 `AppError`
    //    都能读到同一个 trace_id，从而保证响应体 `trace_id` == `X-Trace-Id` 响应头。
    //
    //    注意：**这里不承诺「panic 也能回写头」**。panic 的回写能力来自内层的
    //    `catch_panic_middleware`（它把 panic 变成正常的 500 响应，本函数之后的代码
    //    因此得以继续执行）；若 panic 穿透到本函数之上，下面第 5/6 步不会运行。
    //    见模块文档「panic 捕获层」。
    let _guard = span.enter();
    let mut response = TRACE_ID.scope(trace_uuid, next.run(request)).await;

    // 5. V15 P2 20.1-C：tail-based sampling — 5xx / 慢请求强制采样
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let status = response.status();
    let is_5xx = status.is_server_error();
    let is_slow = elapsed_ms > slow_request_threshold_ms();

    if is_5xx || is_slow {
        // 强制采样：在响应头中标记 `X-Trace-Sampled: forced`
        // OTel Collector 可据此决定保留此 trace
        let v = HeaderValue::from_static("forced");
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-trace-sampled"), v);
        tracing::warn!(
            trace_id = %ctx.trace_id,
            span_id = %ctx.span_id,
            method = %method,
            path = %uri_path,
            status = %status,
            elapsed_ms = %elapsed_ms,
            is_5xx = is_5xx,
            is_slow = is_slow,
            "trace.tail_sampled"
        );
    }

    // 6. 把 trace_id 写入响应头（X-Trace-Id）
    //
    //    与失败响应体的 trace_id 同源：两者都来自第 4 步绑定的 `trace_uuid`
    //    （`AppError::into_response` 读 `TRACE_ID` task-local）。这里用 `or_insert`
    //    语义仅在缺失时写入，已写入时保持同值、不产生第二种形态。
    let header_value = trace_uuid.simple().to_string();
    let expected = HeaderValue::from_str(&header_value).unwrap_or_else(|e| {
        // trace_uuid 来自 Uuid，必定是可进 header 的 ASCII hex；不可达仍需显式记录
        tracing::error!(trace_id = %header_value, error = %e, "X-Trace-Id 头值构造失败");
        HeaderValue::from_static("")
    });
    response
        .headers_mut()
        .entry(HeaderName::from_static(X_TRACE_ID_HEADER))
        .or_insert(expected);

    tracing::info!(
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        method = %method,
        path = %uri_path,
        status = %status,
        elapsed_ms = %elapsed_ms,
        "trace.complete"
    );

    response
}

/// 把 panic 载荷转成人可读的原因字符串。
///
/// `catch_unwind` 的 payload 由 panic 点决定：`panic!("...")` / `unwrap()` / `expect()`
/// 一般是 `String` 或 `&'static str`；非字符串 payload 不能吞成空串，必须显式说明。
fn panic_payload_reason(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic 载荷非字符串类型（无法从 payload 取到原因；请结合本日志的 trace_id 与进程 stderr 定位）"
            .to_string()
    }
}

/// 在**当前 poll 内**捕获 future 的 panic。
///
/// 实现约束（对应验收「不要用阻塞式 poll 包装、不改变正常路径性能语义」）：
/// - 用 `poll_fn` 在原有 waker 驱动机制下逐次 `poll`，`Pending` 时原样把 `Pending`
///   交回 executor，不 `block_on`、不起新 task、不额外 spawn；
/// - 只在 `Poll::Ready` 时才取出结果（panic 发生时返回 `Ready(Err(payload))`）；
/// - `catch_unwind` 只包住单次 `poll` 调用（这是 `AssertUnwindSafe` 的正当用法：
///   future 的 `Pin<&mut>` 在 poll 期间的可变性由 unwind 终止，不再被复用）。
async fn catch_unwind_in_poll<F: Future>(fut: F) -> Result<F::Output, Box<dyn Any + Send>> {
    let mut boxed = AssertUnwindSafe(Box::pin(fut));
    std::future::poll_fn(move |cx| {
        let AssertUnwindSafe(pinned) = &mut boxed;
        let poll_result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let p: Pin<&mut _> = pinned.as_mut();
            p.poll(cx)
        }));
        match poll_result {
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            // Pending：不取结果、不包装，正常路径零额外开销
            Ok(Poll::Pending) => Poll::Pending,
            Err(payload) => Poll::Ready(Err(payload)),
        }
    })
    .await
}

/// 提取器拒绝响应体读取上限：axum rejection 正文是短文案（含 serde 错误一行），
/// 超限说明响应异常，显式记 ERROR 后仍转统一信封，不放行超大 body、不静默。
const MAX_REJECTION_BODY_BYTES: usize = 64 * 1024;

/// H 族收口（CI #4669）：把**提取器拒绝**的纯文本响应归一为统一 `AppError` 失败信封。
///
/// 背景：`Json<T>` 解码失败 → axum 默认 422 纯文本、`Query<T>`/`Path<T>`/`Form<T>`
/// 解码失败 → 400 纯文本、缺 `Content-Type: application/json` → 415 纯文本；
/// 三者都不带 `code/trace_id/timestamp`，违反本仓「失败只有 `AppError` 一种形状」
/// 硬规则（`utils/error.rs` 模块文档）。
///
/// 识别判据（精确锁定 extractor rejection，绝不外溢到鉴权面）：
/// - 状态 ∈ {400, 415, 422}：axum/axum-core 全部「请求解码拒绝」族状态
///   （`JsonDataError`/`FailedToDeserializeFormBody`=422，`JsonSyntaxError`/
///   `FailedToDeserializeQueryString`/`FailedToDeserializePathParams`=400，
///   `MissingJsonContentType`/`InvalidFormContentType`=415；422 在本仓既有
///   AppError 体系中**不存在任何构造点**，纯文本 422 必为 extractor 拒绝）；
/// - `content-type` 以 `text/plain` 开头：这是 axum rejection 的固定出参形态。
///   `AppError` 信封、auth 401 / permission 403（`utils/response.rs` JSON 构造）
///   全是 `application/json`，被本判据整体隔离——**鉴权拒绝出参零变化**。
/// - 404/405（未注册路由 / 方法不匹配）与 413（body 超限）不在本轮映射范围
///   （见 wave-f 报告第四节的待拍板语义；413 属独立网关族，不得并入 VALIDATION）。
///
/// 处置：serde 原文只进 `tracing::warn`（含 method/path/query 定位上下文），
/// 出参替换为 [`AppError::request_decoding_failed`]（400 + `code=VALIDATION_ERROR`
/// + 固定公开规则文案 [`crate::utils::error::REQUEST_DECODING_PUBLIC`]），
/// trace_id/timestamp 复用 `AppError::into_response` 的既有同源路径。
/// 响应体读取失败（断链等）时不静默放行残破响应：记 ERROR 后同样转信封。
async fn normalize_extractor_rejection(
    response: Response,
    method: &str,
    path: &str,
    query: &str,
) -> Response {
    let status = response.status();
    let is_decoding_rejection_status = matches!(
        status,
        StatusCode::BAD_REQUEST
            | StatusCode::UNSUPPORTED_MEDIA_TYPE
            | StatusCode::UNPROCESSABLE_ENTITY
    );
    if !is_decoding_rejection_status {
        return response;
    }
    let is_plain_text = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.to_ascii_lowercase().starts_with("text/plain"));
    if !is_plain_text {
        // JSON 信封（AppError / 鉴权拒绝等）原样放行，绝不触碰。
        return response;
    }

    let (_parts, body) = response.into_parts();
    let raw = match axum::body::to_bytes(body, MAX_REJECTION_BODY_BYTES).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(e) => {
            tracing::error!(
                method = %method,
                path = %path,
                error = %e,
                "extractor.rejection_body_unreadable：纯文本解码拒绝响应体读取失败，仍转统一信封（不静默放行残破响应）"
            );
            String::from("<响应体读取失败>")
        }
    };
    // 不吞原因：serde 原文逐条进日志（warn 级，属客户端可修正的 LOW 族），
    // 只不外进 HTTP 出参——原文可能含结构体名/类型路径/行列号，属内部详情。
    tracing::warn!(
        method = %method,
        path = %path,
        query = %query,
        original_status = %status,
        raw_rejection = %raw,
        "extractor.rejection_normalized：axum 提取器拒绝（纯文本、无机器码）已转统一 AppError 信封（400 + code=VALIDATION_ERROR），serde 原文只进本日志不外显"
    );
    AppError::request_decoding_failed().into_response()
}

/// panic 捕获层：把请求处理链任意内层抛出的 panic 转成**完整的 `AppError` 信封**。
///
/// 修复前：洋葱链没有 catch-panic 层，handler panic 会直接 unwind 穿到 hyper，
/// 连接被拆，客户端拿到的是空响应/断链而不是 `{code, message, trace_id, timestamp}`，
/// 运维侧也缺少「哪个请求、什么原因 panic」的结构化记录。
///
/// 修复后语义：
/// - 状态码 500 + `code=INTERNAL_ERROR` + `trace_id`（与 `X-Trace-Id` 响应头同源，
///   因为本层注册在 `trace_context_middleware` **内层**，`TRACE_ID` task-local 已绑定）；
/// - 以 **ERROR** 级别记录 panic 的位置（HTTP method + 请求路径 + 当前 tracing span）
///   与原因（panic payload）；
/// - 正常路径（无 panic）只是多一次 `poll` 转发，不引入阻塞；响应侧另做一次
///   O(1) 状态/content-type 判据检查，仅命中「提取器拒绝纯文本」时才改写信封
///   （见 [`normalize_extractor_rejection`]，这是本层除 panic 外的第二个
///   **失败形状收口**职责：422/400/415 纯文本 → 400 + `VALIDATION_ERROR` 信封）。
///
/// 为什么收口在本层（洋葱顺序证据，`bootstrap/middleware_bootstrap.rs:71-74`）：
/// 提取器拒绝发生在 handler 调用点——比全部中间件层都靠内，因此任何响应回廊
/// 都必然穿过本层；而本层紧贴 `trace_context` 内侧，`TRACE_ID` task-local 已绑定，
/// 构造的信封 trace_id 与请求 trace 严格同源。同时本层由
/// `apply_trace_and_panic_capture` 在完整模式与 Setup 模式**两条链共用**
/// （middleware_bootstrap.rs:173-177, 363），单点改动即全站生效，无需碰 routes/handlers。
///
/// 挂载位置见 `bootstrap::middleware_bootstrap::apply_trace_and_panic_capture`。
pub async fn catch_panic_middleware(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("").to_string();

    match catch_unwind_in_poll(next.run(request)).await {
        Ok(response) => {
            normalize_extractor_rejection(response, method.as_str(), &path, &query).await
        }
        Err(payload) => {
            let reason = panic_payload_reason(&*payload);
            // task-local 已绑定（本层在 trace_context 内层）→ 复用同一 trace_id；
            // 万一被挂载在 trace 之外，current_trace_id 会显式记 WARN 后现造，不静默。
            let (trace_id, trace_source) = current_trace_id();
            tracing::error!(
                method = %method,
                path = %path,
                query = %query,
                panic_reason = %reason,
                trace_id = %trace_id,
                trace_source = ?trace_source,
                "panic.captured：请求处理链抛出 panic，已捕获并转成 AppError 信封（500 + code + trace_id），\
                 panic 点的源码行号由 Rust 默认 panic hook 输出到进程 stderr"
            );
            // detail 只进日志（InternalError 出参为固定脱敏文案），这里带上定位所需上下文
            AppError::internal(format!(
                "请求处理链 panic（method={} path={} trace_id={}）原因：{}",
                method, path, trace_id, reason
            ))
            .into_response()
        }
    }
}
