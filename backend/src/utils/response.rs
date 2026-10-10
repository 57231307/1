use axum::{
    Json,
    http::{HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::Serialize;

use utoipa::ToSchema;

use crate::middleware::trace_context::X_TRACE_ID_HEADER;
use crate::utils::error::{
    CODE_FORBIDDEN, CODE_UNAUTHORIZED, ErrorResponse, TraceIdSource, current_trace_id,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 分页总数（可选，仅列表分页接口由后端顶层返回；批次 91 P0-1：部分前端如 api-gateway 从 ApiResponse 顶层读 total 而非 data.items.total，该字段可选不影响不需要 total 的端点）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl<T> Default for ApiResponse<T> {
    fn default() -> Self {
        Self {
            code: Some(500),
            data: None,
            message: None,
            total: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

impl<T> Default for PaginatedResponse<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            page: 1,
            page_size: 10,
        }
    }
}

impl<T: Clone> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        Self {
            items: data,
            total,
            page,
            page_size,
        }
    }
}

// 将 PaginatedResponse 转换为 ApiResponse<Vec<T>>（保留兼容性）
// 注意：这会丢弃分页元数据，新代码应使用 ApiResponse::success(PaginatedResponse<T>)
impl<T> From<PaginatedResponse<T>> for ApiResponse<Vec<T>> {
    fn from(paginated: PaginatedResponse<T>) -> Self {
        ApiResponse {
            code: Some(200),
            data: Some(paginated.items),
            message: None,
            total: Some(paginated.total),
        }
    }
}

// P2 2-10 修复：移除 PaginatedResponse<T> 的 IntoResponse 实现
// 原实现将 items 放入 data、total/page 拼接到 message，丢失结构化分页信息。
// 新代码必须使用 ApiResponse::success(PaginatedResponse<T>) 或 ApiResponse::success_paginated，
// 确保分页元数据（total/page/page_size）以结构化形式保留在 data 字段中。

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: Some(200),
            data: Some(data),
            message: None,
            total: None,
        }
    }

    pub fn success_paginated(
        data: Vec<T>,
        total: u64,
        page: u64,
        page_size: u64,
    ) -> ApiResponse<PaginatedResponse<T>> {
        // data 字段统一放置 PaginatedResponse 结构，便于前端直接消费分页元数据
        ApiResponse {
            code: Some(200),
            data: Some(PaginatedResponse {
                items: data,
                total,
                page,
                page_size,
            }),
            message: None,
            total: None,
        }
    }

    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            code: Some(200),
            data: Some(data),
            message: Some(message.to_string()),
            total: None,
        }
    }
}

impl<T: Serialize> From<T> for ApiResponse<T> {
    fn from(data: T) -> Self {
        Self::success(data)
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status_code = StatusCode::from_u16(self.code.unwrap_or(200)).unwrap_or(StatusCode::OK);
        (status_code, Json(self)).into_response()
    }
}

pub fn unauthorized_response(message: &str) -> Response {
    unified_error_response(StatusCode::UNAUTHORIZED, CODE_UNAUTHORIZED, message)
}

pub fn forbidden_response(message: &str) -> Response {
    unified_error_response(StatusCode::FORBIDDEN, CODE_FORBIDDEN, message)
}

/// 认证/鉴权中间件失败出参：与 `AppError::into_response` 完全同构（复用
/// [`ErrorResponse`]，键与类型一致：`code` 为字符串码、`trace_id` 为 32 位小写 hex、
/// `timestamp` 为秒级 i64），HTTP 状态码由调用方（`StatusCode::UNAUTHORIZED` / `FORBIDDEN`）决定，保持不变。
///
/// trace_id 与 [`AppError::into_response`] **同一取源**：[`current_trace_id`] 读 `TRACE_ID`
/// task-local（由 `middleware::trace_context` 在链最外层绑定，覆盖 auth/permission/csrf/init_token
/// 全部内层），因此响应体 `trace_id` 与响应头 `X-Trace-Id` 是同一个值，用户报障给出的号码
/// 可以直接在日志里检索到。形态为 `Uuid::simple()` 的 32 位小写 hex（不带 `-`），
/// 与 `X-Trace-Id` 响应头逐字符一致；task-local 未绑定时由 `current_trace_id` 显式记 WARN 后回退，
/// 不静默。
///
/// message 不走 `AppError::unauthorized(...).into_response()`：`public_message()`（`utils/error.rs`）
/// 对 `Unauthorized`/`PermissionDenied` 一律返回脱敏常量（「未授权」/「无权限」），
/// 会抹掉中间件刻意告知用户下一步动作的文案（如「令牌已被吊销，请重新登录」）。
/// 这些文案是中间件自身的固定字面量、不含他人数据/内部 ID/SQL，外显不越出脱敏契约的安全边界。
pub fn unified_error_response(status: StatusCode, code: &str, message: &str) -> Response {
    let (trace_id, trace_source) = current_trace_id();
    let body = ErrorResponse {
        code: code.to_string(),
        message: message.to_string(),
        trace_id: trace_id.clone(),
        timestamp: Utc::now().timestamp(),
    };
    let mut response = (status, Json(body)).into_response();
    attach_trace_id_header(&mut response, &trace_id, trace_source);
    response
}

/// 失败信封的 `X-Trace-Id` 响应头补齐（与 `AppError::respond` 同语义）。
///
/// `entry().or_insert()`：`trace_context` 位于链最外层，正常路径下它已用同一个 task-local 值
/// 写好本头，这里不会覆盖；仅当响应产生于 trace 作用域之外（装配顺序被改动才会出现）时补齐，
/// 让「用户拿到的号码」与「响应体里的号码」始终是同一个值。
///
/// `trace_id` 必须由调用方与响应体**同一次** [`current_trace_id`] 调用产出后传入：
/// 若在补齐处再取一次，task-local 未绑定时会现造出第二个号码，同源就断了。
pub fn attach_trace_id_header(response: &mut Response, trace_id: &str, source: TraceIdSource) {
    match HeaderValue::from_str(trace_id) {
        Ok(v) => {
            response
                .headers_mut()
                .entry(HeaderName::from_static(X_TRACE_ID_HEADER))
                .or_insert(v);
        }
        // trace_id 来自 Uuid，必为合法 ASCII；不可达仍需显式记录，不静默。
        Err(e) => tracing::error!(
            trace_id = %trace_id,
            error = %e,
            source = ?source,
            "X-Trace-Id 响应头写入失败（trace_id 不是合法 header 值）"
        ),
    }
}
