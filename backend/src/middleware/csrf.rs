// CSRF（Cross-Site Request Forgery）防护中间件
//
// 设计目标：
// - 对所有「有副作用」的 HTTP 方法（POST/PUT/PATCH/DELETE）强制要求请求头 `X-CSRF-Token`。
// - 安全方法（GET/HEAD/OPTIONS）天然无副作用，跳过校验。
// - 公开路径（登录、刷新、初始化、健康检查等）跳过校验，由 [is_public_path] 控制。
// - 校验通过后立即从缓存移除 token，实现「一次性使用」rotation，防止重放。
// - 缺失/无效 token 均以 403 + 业务 code 返回，由前端拦截并跳转登录。
//
// 安全约束：
// - 错误消息走常量 [CSRF_MISSING_MSG] / [CSRF_INVALID_MSG] / [CSRF_IP_MISMATCH_MSG]，
//   禁止硬编码到响应体中。
// - 命名遵循 ≤9 个英文字符的内部约定（如 `CSRF_HDR`、`CODE_MISS` 等仅在本文件内使用）。
// - 死代码处理遵循项目规范：逐项评估，接入业务或删除，禁止保留无标注死代码。
//
// Wave 3 安全漏洞 #7 增强（IP 绑定）：
// - 消费时校验 token 绑定的 IP 与请求 IP 是否一致；不一致返回 403 + 业务码
//   `CSRF_IP_MISMATCH`。IP 来源：X-Real-IP → X-Forwarded-For → ConnectInfo → "unknown"。

use crate::container::AppState;
use crate::middleware::audit_context::extract_client_ip as extract_client_ip_helper;
use crate::middleware::public_routes::is_public_path;
use crate::utils::cache::CsrfConsumeResult;
use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// CSRF 请求头名称（小写形式，对应 HTTP/2 规范）
const CSRF_HDR_NAME: &str = "x-csrf-token";

/// 业务错误码：缺失 CSRF Token
const CODE_MISS: &str = "CSRF_TOKEN_MISSING";

/// 业务错误码：CSRF Token 无效或已过期
const CODE_INVAL: &str = "CSRF_TOKEN_INVALID";

/// 业务错误码：CSRF Token 绑定的 IP 与请求 IP 不一致（Wave 3 #7）
const CODE_IP_MM: &str = "CSRF_IP_MISMATCH";

/// 业务错误消息：缺失 CSRF Token
const CSRF_MISSING_MSG: &str = "CSRF Token 缺失";

/// 业务错误消息：CSRF Token 无效或已过期
const CSRF_INVALID_MSG: &str = "CSRF Token 无效或已过期";

/// 业务错误消息：CSRF Token 绑定的 IP 与请求 IP 不一致（Wave 3 #7）
const CSRF_IP_MISMATCH_MSG: &str = "CSRF Token IP 不匹配";

/// 从请求中提取客户端 IP（Wave 3 #7）：转发至 audit_context helper
fn extract_client_ip(request: &Request<Body>) -> String {
    extract_client_ip_helper(request)
}

/// 判断是否为无副作用的 HTTP 安全方法（GET/HEAD/OPTIONS）
fn is_safe_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// 公开路径 L-1 防御：要求携带 X-Requested-With 或 X-CSRF-Token 自定义请求头
fn check_public_path_header(
    request: &Request<Body>,
    path: &str,
    method: &Method,
) -> Result<(), Box<Response>> {
    let has_xhr = request
        .headers()
        .get("x-requested-with")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("XMLHttpRequest"))
        .unwrap_or(false);
    let has_csrf = request.headers().contains_key(CSRF_HDR_NAME);

    if !has_xhr && !has_csrf {
        tracing::warn!(
            path = %path,
            method = %method,
            "CSRF 验证失败：公开端点的非安全方法缺少自定义请求头（L-1 防御）"
        );
        return Err(Box::new(csrf_error_response(
            CODE_MISS,
            "缺少必要的请求头，请使用 AJAX 方式请求",
        )));
    }
    Ok(())
}

/// 从请求头提取并清理 CSRF Token（去空白、过滤空串）
fn extract_csrf_token(request: &Request<Body>) -> Option<String> {
    request
        .headers()
        .get(CSRF_HDR_NAME)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 一次性消费 CSRF Token，含 IP 绑定校验（Wave 3 #7）
fn consume_csrf_token(
    state: &AppState,
    token: &str,
    client_ip: &str,
    path: &str,
    method: &Method,
) -> Result<(), Box<Response>> {
    match state.cache.consume_csrf_token(token, client_ip) {
        CsrfConsumeResult::Ok => Ok(()),
        CsrfConsumeResult::IpMismatch => {
            tracing::warn!(
                path = %path,
                method = %method,
                client_ip = %client_ip,
                "CSRF 验证失败：Token 绑定的 IP 与请求 IP 不一致（Wave 3 #7 防御）"
            );
            Err(Box::new(csrf_error_response(
                CODE_IP_MM,
                CSRF_IP_MISMATCH_MSG,
            )))
        }
        CsrfConsumeResult::NotFound => {
            tracing::warn!(
                path = %path,
                method = %method,
                "CSRF 验证失败：Token 不存在或已被消费/过期"
            );
            Err(Box::new(csrf_error_response(CODE_INVAL, CSRF_INVALID_MSG)))
        }
    }
}

/// CSRF 验证中间件：跳过安全方法，公开路径要求自定义请求头，非公开路径校验 Token + IP
pub async fn csrf_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 1. 跳过无副作用方法
    if is_safe_method(&method) {
        return Ok(next.run(request).await);
    }

    // 2. 公开路径：L-1 修复 - 要求自定义请求头（防御简单表单提交 CSRF）
    if is_public_path(&path) {
        check_public_path_header(&request, &path, &method).map_err(|e| *e)?;
        return Ok(next.run(request).await);
    }

    // 3. 提取并校验 CSRF Token 头
    let token = match extract_csrf_token(&request) {
        Some(t) => t,
        None => {
            tracing::warn!(
                path = %path,
                method = %method,
                "CSRF 验证失败：请求头 X-CSRF-Token 缺失"
            );
            return Err(csrf_error_response(CODE_MISS, CSRF_MISSING_MSG));
        }
    };

    // 4. 提取客户端 IP + 一次性消费 token（Wave 3 #7 含 IP 校验）
    let client_ip = extract_client_ip(&request);
    consume_csrf_token(&state, &token, &client_ip, &path, &method).map_err(|e| *e)?;

    Ok(next.run(request).await)
}

/// 构造 403 CSRF 错误响应（统一 JSON 格式）
fn csrf_error_response(code: &str, message: &str) -> Response {
    let body = json!({
        "success": false,
        "code": code,
        "message": message,
        "data": null,
    });
    (StatusCode::FORBIDDEN, Json(body)).into_response()
}
