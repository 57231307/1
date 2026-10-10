//! 可选 JSON 请求体提取器：理由选填的动作端点必须能把「未按 JSON 提交」当合法输入。
//!
//! 背景（为什么必须自研、不能用 `Option<Json<T>>`）：axum 0.8.9 的
//! `OptionalFromRequest for Json<T>`（`axum-0.8.9/src/json.rs:116-136`）只在
//! **请求完全不带 `Content-Type` 头**时返回 `Ok(None)`；一旦带了 JSON 头但体为空，
//! 仍会走 `Json::from_bytes` 触发 `JsonRejection::InvalidJsonBody(EOF while
//! parsing a value)`，由 `middleware::trace_context::normalize_extractor_rejection`
//! 收口成 400 + `code=VALIDATION_ERROR`。e2e 负例 helper（`frontend/e2e/flow/
//! helpers.ts` `apiCallExpectFail`）恰好是"带 JSON 头、不带体"形态，因此
//! `Option<Json<T>>` 对它是**假的可选**，业务门永不可达。
//!
//! 本提取器把"选填"的裁决权收回本仓自己实现，语义表：
//! - 无 `Content-Type` 头 ⇒ `Ok(None)`（整体未按 JSON 提交，视为未采集）；
//! - `Content-Type` 为 JSON 族（`application/json` 或 `+json` 后缀，判定口径
//!   对齐 axum `json_content_type`，`axum-0.8.9/src/json.rs:138-154`）且体为空
//!   （长度 0 或仅 ASCII 空白）⇒ `Ok(None)`，合法放行给业务门（状态门/存在性门）；
//! - 体非空且为合法 JSON 且能反序列化为 `T` ⇒ `Ok(Some(T))`；
//! - 体非空但 JSON 语法错 / 字段类型不匹配 / 必填字段缺失 ⇒ **400
//!   `VALIDATION_ERROR`**（既有信封 `AppError::request_decoding_failed()`，
//!   不得退化成 `Ok(None)`——那会把"有体但非法"吞成"未采集"，违反
//!   "字段级校验=VALIDATION_ERROR"的既有裁定）；
//! - `multipart` / `text/plain` 等**非 JSON content-type 且体非空** ⇒ 同样显式
//!   拒绝并走 400 `VALIDATION_ERROR` 收口。依据：`REQUEST_DECODING_PUBLIC`
//!   固定文案本身覆盖"内容类型错误"（`utils/error.rs:850-851`），与
//!   `normalize_extractor_rejection` 把 axum 原生 415 折进同一 400 信封的口径
//!   一致（`trace_context.rs:266-271` 把 400/415/422 三态统一收口为 400），
//!   不另造第二套 415 直出形态；
//! - 非 JSON content-type 且体为空 ⇒ `Ok(None)`（空体无任何信息量，与"缺体"
//!   同一归一，不属于"体非空"的显式拒绝分支）。
//!
//! 纪律对齐：
//! - serde 原始拒绝详情逐条进 `tracing::warn`（不吞原因），**严禁**进入 HTTP
//!   出参（原文可能含结构体名/类型路径/行列号，属内部详情，口径同 trace_context）；
//! - 体上限 [`MAX_OPTIONAL_JSON_BODY_BYTES`] 对齐 axum `Json` 默认 2MB body
//!   limit，不因新通道放宽输入面；
//! - 消费请求体，handler 中若有多个 body 提取器必须放最后（同 axum `Json` 约束）。

use axum::body::to_bytes;
use axum::extract::{FromRequest, Request};
use axum::http::header;
use serde::de::DeserializeOwned;

use crate::utils::error::AppError;

/// 可选 JSON 体的上限：对齐 axum `DefaultBodyLimit` 对 `Json` 的默认 2MB 限制。
/// 调小会拒绝合法的厚体请求，调大等于给本通道放宽输入面，均不允许。
const MAX_OPTIONAL_JSON_BODY_BYTES: usize = 2 * 1024 * 1024;

/// 选填 JSON 请求体：`None` = 客户端确实未按 JSON 提交（无头 / JSON 头 + 空体）；
/// `Some(T)` = 体非空且已成功反序列化。
///
/// 新类型只包一层 `Option<T>`，`Deref` 语义由调用点显式 `.0` 解构，不隐藏分支。
#[derive(Debug)]
pub struct OptionalJson<T>(pub Option<T>);

/// content-type 是否属 JSON 族：`application/json` 或 `xxx/+json` 后缀。
/// 判定口径对齐 axum `json_content_type`（`axum-0.8.9/src/json.rs:138-154`，
/// mime 解析后 type_=="application" 且 subtype/suffix 为 json），此处手写等价
/// 字符串判定（本仓不直依赖 mime）。允许 `; charset=utf-8` 等参数尾巴。
fn is_json_content_type(value: &str) -> bool {
    let main = value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    main == "application/json" || main.ends_with("+json")
}

impl<T, S> FromRequest<S> for OptionalJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let uri = req.uri().to_string();
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let bytes = match to_bytes(req.into_body(), MAX_OPTIONAL_JSON_BODY_BYTES).await {
            Ok(b) => b,
            Err(e) => {
                // 读体失败（超限/断链）：不静默放行残破输入，按解码失败显式拒绝。
                tracing::warn!(
                    uri = %uri,
                    error = %e,
                    "optional_json.body_read_failed：请求体读取失败，转统一解码失败信封"
                );
                return Err(AppError::request_decoding_failed());
            }
        };

        // 空体（长度 0 或仅 ASCII 空白）双侧同判：无论 content-type 是否 JSON 族，
        // 未提交任何信息 ⇒ 归一为"未采集"。
        if bytes.iter().all(u8::is_ascii_whitespace) {
            if let Some(ct) = &content_type {
                if !is_json_content_type(ct) {
                    // 防御分支：非 JSON 头 + 空体也放行为 None（空体无内容可误读）；
                    // 与"非 JSON 头 + 体非空"的显式拒绝分支互斥，见模块语义表。
                    tracing::debug!(
                        uri = %uri,
                        content_type = %ct,
                        "optional_json.empty_body_without_json_ct：空体按未采集归一（未按 JSON 提交）"
                    );
                }
            }
            return Ok(Self(None));
        }

        // 以下体非空。
        let Some(ct) = content_type else {
            // 无 content-type 但体非空：客户端未按 JSON 协议提交，语义上等同
            // axum `OptionalFromRequest for Json` 的"无头 ⇒ None"放行分支。
            // 此处刻意跟随该先例（否则同一请求在选填/必填两种端点间行为翻转），
            // 但留 WARN 让"带头缺失却带厚体"的异常调用方在日志里可见、不静默。
            tracing::warn!(
                uri = %uri,
                body_len = bytes.len(),
                "optional_json.no_content_type_with_body：无 content-type 但体非空，按未采集归一放行（调用方疑似漏带头，日志留痕不静默）"
            );
            return Ok(Self(None));
        };
        if !is_json_content_type(&ct) {
            // 非 JSON content-type 且体非空（multipart/text/plain 等）：显式拒绝。
            // 依据见模块头语义表（与既有 415→400 收口同信封，不另造 415 直出）。
            tracing::warn!(
                uri = %uri,
                content_type = %ct,
                body_len = bytes.len(),
                "optional_json.unsupported_content_type：非 JSON 内容类型且体非空，按解码失败拒绝"
            );
            return Err(AppError::request_decoding_failed());
        }

        match serde_json::from_slice::<T>(&bytes) {
            Ok(v) => Ok(Self(Some(v))),
            Err(e) => {
                // serde 原文只进日志不外显（可能含结构体名/字段路径/行列号）。
                tracing::warn!(
                    uri = %uri,
                    serde_error = %e,
                    "optional_json.deserialize_failed：体非空但 JSON 语法/字段校验失败，转统一 400 VALIDATION_ERROR 信封"
                );
                Err(AppError::request_decoding_failed())
            }
        }
    }
}
