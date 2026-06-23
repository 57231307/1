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
// - 任何死代码必须显式标注 `#[allow(dead_code)]` + TODO(tech-debt)，与 utils/ 模板保持一致。
//
// Wave 3 安全漏洞 #7 增强（IP 绑定）：
// - 消费时校验 token 绑定的 IP 与请求 IP 是否一致；不一致返回 403 + 业务码
//   `CSRF_IP_MISMATCH`。IP 来源：X-Real-IP → X-Forwarded-For → ConnectInfo → "unknown"。

use crate::middleware::public_routes::is_public_path;
use crate::utils::app_state::AppState;
use crate::utils::cache::CsrfConsumeResult;
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::SocketAddr;

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

/// 从请求中提取客户端 IP（Wave 3 #7）
///
/// 优先级与 [crate::middleware::audit_context::extract_ip] 一致：
/// X-Real-IP → X-Forwarded-For（取首段）→ ConnectInfo(SocketAddr) → "unknown"。
/// 失败时的"unknown"与 [cache::consume_csrf_token] 的 IP 比对语义：
/// 若登录时也是 unknown（无 IP header 场景），则能正常消费；
/// 若登录时有 IP 但消费时 unknown，则触发 IP 不匹配（符合预期：IP 失配即拒绝）。
fn extract_client_ip(request: &Request<Body>) -> String {
    let headers = request.headers();
    if let Some(real_ip) = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        return real_ip.to_string();
    }
    if let Some(forwarded) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = forwarded.split(',').next() {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    if let Some(ConnectInfo(addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }
    "unknown".to_string()
}

/// CSRF 验证中间件
///
/// 校验策略：
/// 1. 跳过方法：GET / HEAD / OPTIONS（HTTP 语义无副作用）。
/// 2. 跳过路径：[PUBLIC_PATHS](crate::middleware::public_routes::PUBLIC_PATHS) 中的端点。
/// 3. 其它情况：要求 `X-CSRF-Token` 头存在且与 [AppCache::consume_csrf_token] 匹配，
///    且请求 IP 与 token 绑定的 IP 一致（Wave 3 #7）。
///
/// 失败响应：
/// - 缺失头 → 403 + `{success:false, code:CSRF_TOKEN_MISSING, message:"CSRF Token 缺失"}`
/// - 无效/过期 → 403 + `{success:false, code:CSRF_TOKEN_INVALID, message:"CSRF Token 无效或已过期"}`
/// - IP 不匹配 → 403 + `{success:false, code:CSRF_IP_MISMATCH, message:"CSRF Token IP 不匹配"}`
pub async fn csrf_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 1. 跳过无副作用方法
    if matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(next.run(request).await);
    }

    // 2. 跳过公开路径
    if is_public_path(&path) {
        return Ok(next.run(request).await);
    }

    // 3. 提取并校验 CSRF Token 头
    let token_opt = request
        .headers()
        .get(CSRF_HDR_NAME)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let token = match token_opt {
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

    // 4. 提取客户端 IP（Wave 3 #7）
    let client_ip = extract_client_ip(&request);

    // 5. 一次性消费：成功匹配后立即从缓存中移除（Wave 3 #7 含 IP 校验）
    match state.cache.consume_csrf_token(&token, &client_ip) {
        CsrfConsumeResult::Ok => {
            // 通过：继续处理请求
        }
        CsrfConsumeResult::IpMismatch => {
            tracing::warn!(
                path = %path,
                method = %method,
                client_ip = %client_ip,
                "CSRF 验证失败：Token 绑定的 IP 与请求 IP 不一致（Wave 3 #7 防御）"
            );
            return Err(csrf_error_response(CODE_IP_MM, CSRF_IP_MISMATCH_MSG));
        }
        CsrfConsumeResult::NotFound => {
            tracing::warn!(
                path = %path,
                method = %method,
                "CSRF 验证失败：Token 不存在或已被消费/过期"
            );
            return Err(csrf_error_response(CODE_INVAL, CSRF_INVALID_MSG));
        }
    }

    Ok(next.run(request).await)
}

/// 构造 403 CSRF 错误响应
///
/// 响应体结构遵循项目统一 JSON 格式：
/// `{success: false, code: <业务码>, message: <描述>, data: null}`
fn csrf_error_response(code: &str, message: &str) -> Response {
    let body = json!({
        "success": false,
        "code": code,
        "message": message,
        "data": null,
    });
    (StatusCode::FORBIDDEN, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试安全方法（GET/HEAD/OPTIONS）通过 matches! 检查
    #[test]
    fn test_safe_methods_recognized() {
        assert!(matches!(
            Method::GET,
            Method::GET | Method::HEAD | Method::OPTIONS
        ));
        assert!(matches!(
            Method::HEAD,
            Method::GET | Method::HEAD | Method::OPTIONS
        ));
        assert!(matches!(
            Method::OPTIONS,
            Method::GET | Method::HEAD | Method::OPTIONS
        ));
    }

    /// 测试非安全方法不被放行
    #[test]
    fn test_unsafe_methods_not_recognized_as_safe() {
        assert!(!matches!(
            Method::POST,
            Method::GET | Method::HEAD | Method::OPTIONS
        ));
        assert!(!matches!(
            Method::PUT,
            Method::GET | Method::HEAD | Method::OPTIONS
        ));
        assert!(!matches!(
            Method::PATCH,
            Method::GET | Method::HEAD | Method::OPTIONS
        ));
        assert!(!matches!(
            Method::DELETE,
            Method::GET | Method::HEAD | Method::OPTIONS
        ));
    }

    /// 测试 403 缺失错误响应：状态码与负载
    #[tokio::test]
    async fn test_missing_response_payload() {
        let resp = csrf_error_response(CODE_MISS, CSRF_MISSING_MSG);
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body_bytes = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("读取响应体失败");
        let body: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("响应体不是合法 JSON");
        assert_eq!(body.get("success").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            body.get("code").and_then(|v| v.as_str()),
            Some("CSRF_TOKEN_MISSING")
        );
        assert_eq!(
            body.get("message").and_then(|v| v.as_str()),
            Some("CSRF Token 缺失")
        );
        assert!(body.get("data").map(|v| v.is_null()).unwrap_or(false));
    }

    /// 测试 403 无效错误响应：状态码与负载
    #[tokio::test]
    async fn test_invalid_response_payload() {
        let resp = csrf_error_response(CODE_INVAL, CSRF_INVALID_MSG);
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body_bytes = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("读取响应体失败");
        let body: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("响应体不是合法 JSON");
        assert_eq!(
            body.get("code").and_then(|v| v.as_str()),
            Some("CSRF_TOKEN_INVALID")
        );
        assert_eq!(
            body.get("message").and_then(|v| v.as_str()),
            Some("CSRF Token 无效或已过期")
        );
    }

    /// 测试 403 IP 不匹配错误响应：状态码与负载（Wave 3 #7）
    #[tokio::test]
    async fn test_ip_mismatch_response_payload() {
        let resp = csrf_error_response(CODE_IP_MM, CSRF_IP_MISMATCH_MSG);
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body_bytes = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("读取响应体失败");
        let body: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("响应体不是合法 JSON");
        assert_eq!(
            body.get("code").and_then(|v| v.as_str()),
            Some("CSRF_IP_MISMATCH")
        );
        assert_eq!(
            body.get("message").and_then(|v| v.as_str()),
            Some("CSRF Token IP 不匹配")
        );
    }

    /// 测试错误码常量值未被误改
    #[test]
    fn test_error_code_constants() {
        assert_eq!(CODE_MISS, "CSRF_TOKEN_MISSING");
        assert_eq!(CODE_INVAL, "CSRF_TOKEN_INVALID");
        assert_eq!(CSRF_MISSING_MSG, "CSRF Token 缺失");
        assert_eq!(CSRF_INVALID_MSG, "CSRF Token 无效或已过期");
        // Wave 3 #7：新增 IP 不匹配业务码
        assert_eq!(CODE_IP_MM, "CSRF_IP_MISMATCH");
        assert_eq!(CSRF_IP_MISMATCH_MSG, "CSRF Token IP 不匹配");
    }

    /// 测试 CSRF 头名常量
    #[test]
    fn test_csrf_header_name() {
        assert_eq!(CSRF_HDR_NAME, "x-csrf-token");
    }

    /// 测试 extract_client_ip 的多级降级（Wave 3 #7）
    #[test]
    fn test_extract_client_ip_priority() {
        use axum::body::Body;
        use axum::http::Request;

        // 场景 1: X-Real-IP 优先级最高
        let req = Request::builder()
            .uri("/")
            .header("x-real-ip", "203.0.113.10")
            .header("x-forwarded-for", "198.51.100.1, 10.0.0.1")
            .body(Body::empty())
            .expect("build");
        assert_eq!(extract_client_ip(&req), "203.0.113.10");

        // 场景 2: 无 X-Real-IP 时取 X-Forwarded-For 首段
        let req = Request::builder()
            .uri("/")
            .header("x-forwarded-for", "198.51.100.1, 10.0.0.1")
            .body(Body::empty())
            .expect("build");
        assert_eq!(extract_client_ip(&req), "198.51.100.1");

        // 场景 3: 都没有时回退 "unknown"
        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("build");
        assert_eq!(extract_client_ip(&req), "unknown");
    }
}
