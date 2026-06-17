//! HTTP 安全响应头常量
//!
//! 实际生效的安全响应头在 `main.rs` 中通过 `SetResponseHeaderLayer` 注入。
//! 本文件保留以下工具方法以供其他模块复用（如错误响应、静态资源响应等场景）。
//!
//! **安全响应头清单**：
//! - Content-Security-Policy
//! - Strict-Transport-Security
//! - X-Content-Type-Options
//! - X-Frame-Options
//! - Referrer-Policy
//! - Permissions-Policy

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

/// Content-Security-Policy 默认值
/// 仅允许加载同源资源；脚本放行 wasm-unsafe-eval 与内联样式以兼容 SPA
const CSP_VALUE: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ws: wss:; font-src 'self' data:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests;";

/// HSTS：1 年 + 子域 + 预加载
const HSTS_VALUE: &str = "max-age=31536000; includeSubDomains; preload";

/// X-Content-Type-Options：禁止 MIME 嗅探
const X_CONTENT_TYPE_OPTIONS_VALUE: &str = "nosniff";

/// X-Frame-Options：禁止嵌入
const X_FRAME_OPTIONS_VALUE: &str = "DENY";

/// Referrer-Policy：跨源仅发送 origin
const REFERRER_POLICY_VALUE: &str = "strict-origin-when-cross-origin";

/// Permissions-Policy：关闭地理位置、麦克风、摄像头
const PERMISSIONS_POLICY_VALUE: &str = "geolocation=(), microphone=(), camera=()";

/// 将安全响应头写入 Response
///
/// 可用于补充 `SetResponseHeaderLayer` 未覆盖到的响应路径（如内部错误降级响应）。
/// 实际主链路（main.rs `create_router`）已通过 `SetResponseHeaderLayer` 集中注入安全头。
pub fn apply_security_headers(response: &mut Response) {
    let headers = response.headers_mut();

    // Content-Security-Policy
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(CSP_VALUE),
    );

    // Strict-Transport-Security（HSTS）
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static(HSTS_VALUE),
    );

    // X-Content-Type-Options
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static(X_CONTENT_TYPE_OPTIONS_VALUE),
    );

    // X-Frame-Options
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static(X_FRAME_OPTIONS_VALUE),
    );

    // Referrer-Policy
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static(REFERRER_POLICY_VALUE),
    );

    // Permissions-Policy
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(PERMISSIONS_POLICY_VALUE),
    );
}

/// 安全响应头中间件（axum middleware 形式）
///
/// 用法示例：
/// ```ignore
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(axum::middleware::from_fn(security_headers_middleware));
/// ```
///
/// 当前主链路未使用本函数（`main.rs` 使用 `SetResponseHeaderLayer` 注入），
/// 但保留作为备用工具方法供需要"完整中间件"形式的场景调用。
#[allow(dead_code)] // TODO(tech-debt): 切换到中间件形式注入安全头时移除
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    apply_security_headers(&mut response);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Response;

    #[test]
    fn test_security_headers_values() {
        // 校验关键安全头值符合最佳实践
        assert!(CSP_VALUE.contains("default-src 'self'"));
        assert!(CSP_VALUE.contains("object-src 'none'"));
        assert!(CSP_VALUE.contains("frame-ancestors 'none'"));
        assert!(HSTS_VALUE.contains("max-age=31536000"));
        assert!(HSTS_VALUE.contains("includeSubDomains"));
        assert!(HSTS_VALUE.contains("preload"));
        assert_eq!(X_CONTENT_TYPE_OPTIONS_VALUE, "nosniff");
        assert_eq!(X_FRAME_OPTIONS_VALUE, "DENY");
    }

    #[tokio::test]
    async fn test_apply_security_headers() {
        let mut response = Response::<Body>::new(Body::empty());
        apply_security_headers(&mut response);
        let headers = response.headers();
        assert!(headers.contains_key("content-security-policy"));
        assert!(headers.contains_key("strict-transport-security"));
        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("referrer-policy"));
        assert!(headers.contains_key("permissions-policy"));
    }
}
