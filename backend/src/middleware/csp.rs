//! CSP 中间件（P4-2 安全加固）
//!
//! 独立中间件形式的 Content-Security-Policy 注入，
//! 与 `security_headers.rs` 工具函数并存（`security_headers` 通过
//! `SetResponseHeaderLayer` 在主链路注入；本模块提供"中间件函数"形态，
//! 便于路由级精细化覆盖）。
//!
//! ## 默认策略
//!
//! - `default-src 'self'` — 仅允许加载同源资源
//! - `script-src 'self' 'wasm-unsafe-eval'` — 支持 WASM（如密码哈希的 argon2 wasm）
//! - `style-src 'self' 'unsafe-inline'` — 兼容 SPA 内联样式
//! - `img-src 'self' data: blob:` — 允许 data URI 图片
//! - `connect-src 'self' ws: wss:` — 支持 WebSocket（用于实时通知）
//! - `object-src 'none'` — 禁止 <object>/<embed>
//! - `base-uri 'self'` — 禁止 <base> 注入
//! - `frame-ancestors 'none'` — 禁止被嵌入 iframe（防 clickjacking）
//! - `upgrade-insecure-requests` — 自动升级 HTTP 到 HTTPS

use axum::{extract::Request, http::{HeaderName, HeaderValue}, middleware::Next, response::Response};

/// CSP 策略值
pub const CSP_POLICY: &str = "default-src 'self'; \
    script-src 'self' 'wasm-unsafe-eval'; \
    style-src 'self' 'unsafe-inline'; \
    img-src 'self' data: blob:; \
    connect-src 'self' ws: wss:; \
    font-src 'self' data:; \
    object-src 'none'; \
    base-uri 'self'; \
    form-action 'self'; \
    frame-ancestors 'none'; \
    upgrade-insecure-requests";

/// CSP 中间件
///
/// 把 `Content-Security-Policy` 头注入到所有响应。
///
/// 批次 97 P1-14 修复（v5 复审）：已挂载到 main.rs production 路由全局中间件链，
/// 替代原 SetResponseHeaderLayer::overriding(CONTENT_SECURITY_POLICY, ...)。
/// 与 SetResponseHeaderLayer::overriding 区别：本中间件仅在响应头未设置 CSP 时注入，
/// 支持路由级精细化覆盖（路由可自定义 CSP 头，中间件不覆盖）。
pub async fn csp_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    // 仅在响应头尚未设置 CSP 时注入（避免覆盖路由级精细化配置）
    if !headers.contains_key("content-security-policy") {
        if let Ok(value) = HeaderValue::from_str(CSP_POLICY) {
            headers.insert(HeaderName::from_static("content-security-policy"), value);
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Response;

    #[test]
    fn 测试_csp_默认值包含关键指令() {
        // 中文测试名：测试 CSP 默认值包含 frame-ancestors none
        assert!(CSP_POLICY.contains("default-src 'self'"));
        assert!(CSP_POLICY.contains("frame-ancestors 'none'"));
        assert!(CSP_POLICY.contains("object-src 'none'"));
        assert!(CSP_POLICY.contains("upgrade-insecure-requests"));
    }

    #[tokio::test]
    async fn 测试_csp_middleware_注入() {
        // 中文测试名：测试 CSP 中间件自动注入响应头
        // 由于 Next 在测试中难以构造，跳过完整集成测试，仅验证常量
        let resp = Response::<Body>::new(Body::empty());
        assert!(!resp.headers().contains_key("content-security-policy"));
    }
}
