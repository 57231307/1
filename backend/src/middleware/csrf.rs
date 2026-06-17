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
// - 错误消息走常量 [CSRF_MISSING_MSG] / [CSRF_INVALID_MSG]，禁止硬编码到响应体中。
// - 命名遵循 ≤9 个英文字符的内部约定（如 `CSRF_HDR`、`CODE_MISS` 等仅在本文件内使用）。
// - 任何死代码必须显式标注 `#[allow(dead_code)]` + TODO(tech-debt)，与 utils/ 模板保持一致。

use crate::middleware::public_routes::is_public_path;
use crate::utils::app_state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// CSRF 请求头名称（小写形式，对应 HTTP/2 规范）
const CSRF_HDR_NAME: &str = "x-csrf-token";

/// 业务错误码：缺失 CSRF Token
const CODE_MISS: &str = "CSRF_TOKEN_MISSING";

/// 业务错误码：CSRF Token 无效或已过期
const CODE_INVAL: &str = "CSRF_TOKEN_INVALID";

/// 业务错误消息：缺失 CSRF Token
const CSRF_MISSING_MSG: &str = "CSRF Token 缺失";

/// 业务错误消息：CSRF Token 无效或已过期
const CSRF_INVALID_MSG: &str = "CSRF Token 无效或已过期";

/// CSRF 验证中间件
///
/// 校验策略：
/// 1. 跳过方法：GET / HEAD / OPTIONS（HTTP 语义无副作用）。
/// 2. 跳过路径：[PUBLIC_PATHS](crate::middleware::public_routes::PUBLIC_PATHS) 中的端点。
/// 3. 其它情况：要求 `X-CSRF-Token` 头存在且与 [AppCache::consume_csrf_token] 匹配。
///
/// 失败响应：
/// - 缺失头 → 403 + `{success:false, code:CSRF_TOKEN_MISSING, message:"CSRF Token 缺失"}`
/// - 无效/过期 → 403 + `{success:false, code:CSRF_TOKEN_INVALID, message:"CSRF Token 无效或已过期"}`
pub async fn csrf_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 1. 跳过无副作用方法
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
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

    // 4. 一次性消费：成功匹配后立即从缓存中移除
    let valid = state.cache.consume_csrf_token(&token);
    if !valid {
        tracing::warn!(
            path = %path,
            method = %method,
            "CSRF 验证失败：Token 不存在或已被消费/过期"
        );
        return Err(csrf_error_response(CODE_INVAL, CSRF_INVALID_MSG));
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
        assert!(matches!(*Method::GET, Method::GET | Method::HEAD | Method::OPTIONS));
        assert!(matches!(*Method::HEAD, Method::GET | Method::HEAD | Method::OPTIONS));
        assert!(matches!(*Method::OPTIONS, Method::GET | Method::HEAD | Method::OPTIONS));
    }

    /// 测试非安全方法不被放行
    #[test]
    fn test_unsafe_methods_not_recognized_as_safe() {
        assert!(!matches!(*Method::POST, Method::GET | Method::HEAD | Method::OPTIONS));
        assert!(!matches!(*Method::PUT, Method::GET | Method::HEAD | Method::OPTIONS));
        assert!(!matches!(*Method::PATCH, Method::GET | Method::HEAD | Method::OPTIONS));
        assert!(!matches!(*Method::DELETE, Method::GET | Method::HEAD | Method::OPTIONS));
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

    /// 测试错误码常量值未被误改
    #[test]
    fn test_error_code_constants() {
        assert_eq!(CODE_MISS, "CSRF_TOKEN_MISSING");
        assert_eq!(CODE_INVAL, "CSRF_TOKEN_INVALID");
        assert_eq!(CSRF_MISSING_MSG, "CSRF Token 缺失");
        assert_eq!(CSRF_INVALID_MSG, "CSRF Token 无效或已过期");
    }

    /// 测试 CSRF 头名常量
    #[test]
    fn test_csrf_header_name() {
        assert_eq!(CSRF_HDR_NAME, "x-csrf-token");
    }
}
