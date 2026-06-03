//! HTTP 安全响应头中间件
//!
//! 统一为所有响应添加：
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
/// 仅允许加载同源资源；脚本与样式放行内联以兼容常见前端框架
const CSP_VALUE: &str = "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:";

/// HSTS：2 年 + 子域 + 预加载
const HSTS_VALUE: &str = "max-age=63072000; includeSubDomains; preload";

/// X-Content-Type-Options：禁止 MIME 嗅探
const X_CONTENT_TYPE_OPTIONS_VALUE: &str = "nosniff";

/// X-Frame-Options：禁止嵌入
const X_FRAME_OPTIONS_VALUE: &str = "DENY";

/// Referrer-Policy：不发送 Referrer
const REFERRER_POLICY_VALUE: &str = "no-referrer";

/// Permissions-Policy：关闭地理位置、麦克风、摄像头、支付
const PERMISSIONS_POLICY_VALUE: &str = "geolocation=(), microphone=(), camera=(), payment=()";

/// 将安全响应头写入 Response
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
/// 放在 `from_fn` 链路的最后一环，确保所有响应（含错误响应）都附带安全头。
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    apply_security_headers(&mut response);
    response
}
